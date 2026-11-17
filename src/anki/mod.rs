//! Direct Zero-Copy Anki rslib Integration & Collection Service
use std::path::{Path, PathBuf};
use std::time::Instant;
use anki::card::CardId;
use anki::collection::{Collection, CollectionBuilder};
use anki::decks::DeckId;
use anki::scheduler::answering::{CardAnswer, Rating};
use anki::scheduler::states::SchedulingStates;
use anki::timestamp::TimestampMillis;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeckItem {
    pub id: i64,
    pub name: String,
    pub full_name: String,
    pub depth: usize,
    pub new_count: u32,
    pub learn_count: u32,
    pub review_count: u32,
    pub due_count: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CardPayload {
    pub card_id: i64,
    pub question_html: String,
    pub answer_html: String,
    pub css: String,
    pub intervals: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CardPipelinePayload {
    pub current: CardPayload,
    pub next: Option<CardPayload>,
    pub deck_name: String,
    pub new_count: u32,
    pub learn_count: u32,
    pub review_count: u32,
}

pub struct AnkiService {
    col: Option<Collection>,
    media_dir: PathBuf,
    current_card_id: Option<CardId>,
    card_shown_at: Option<Instant>,
    current_states: Option<SchedulingStates>,
}

impl AnkiService {
    pub fn new() -> Self {
        Self {
            col: None,
            media_dir: PathBuf::new(),
            current_card_id: None,
            card_shown_at: None,
            current_states: None,
        }
    }

    pub fn is_open(&self) -> bool {
        self.col.is_some()
    }

    pub fn open_collection(
        &mut self,
        col_path: &Path,
        media_folder: &Path,
        media_db: &Path,
    ) -> Result<(), String> {
        let mut builder = CollectionBuilder::new(col_path);
        builder.set_media_paths(media_folder, media_db);
        let col = builder.build().map_err(|e| format!("Failed to open collection: {:?}", e))?;
        self.col = Some(col);
        self.media_dir = media_folder.to_path_buf();
        println!("[ANKI] Opened collection at {:?}", col_path);
        Ok(())
    }

    pub fn get_deck_list(&mut self) -> Result<Vec<DeckItem>, String> {
        let col = self.col.as_mut().ok_or("Collection not open")?;
        let tree = col.deck_tree(None).map_err(|e| format!("Failed to get deck tree: {:?}", e))?;

        let mut list = Vec::new();
        flatten_deck_tree(&tree.children, 0, "", &mut list);
        Ok(list)
    }

    pub fn select_deck(&mut self, deck_id: i64) -> Result<(), String> {
        let col = self.col.as_mut().ok_or("Collection not open")?;
        col.set_current_deck(DeckId(deck_id))
            .map_err(|e| format!("Failed to select deck {}: {:?}", deck_id, e))?;
        println!("[ANKI] Selected deck {}", deck_id);
        Ok(())
    }

    pub fn get_next_card(&mut self) -> Result<Option<CardPayload>, String> {
        let col = self.col.as_mut().ok_or("Collection not open")?;
        let queued = col
            .get_queued_cards(1, false)
            .map_err(|e| format!("Failed to fetch queued cards: {:?}", e))?;

        if queued.cards.is_empty() {
            self.current_card_id = None;
            self.current_states = None;
            return Ok(None);
        }

        let queued_card = &queued.cards[0];
        let cid = queued_card.card.id();
        let states = queued_card.states.clone();
        self.current_card_id = Some(cid);
        self.card_shown_at = Some(Instant::now());

        let intervals = col
            .describe_next_states(&states)
            .unwrap_or_else(|_| vec!["<10分".into(), "1天".into(), "3天".into(), "5天".into()]);
        self.current_states = Some(states);

        let rendered = col
            .render_existing_card(cid, false, false)
            .map_err(|e| format!("Failed to render card {}: {:?}", cid, e))?;

        let media_uri_prefix = format!("file://{}/", self.media_dir.display());
        let question_html = rewrite_media_urls(&rendered.question(), &media_uri_prefix);
        let answer_html = rewrite_media_urls(&rendered.answer(), &media_uri_prefix);

        Ok(Some(CardPayload {
            card_id: cid.0,
            question_html,
            answer_html,
            css: rendered.css,
            intervals,
        }))
    }

    pub fn get_pipeline_cards(&mut self) -> Result<Option<CardPipelinePayload>, String> {
        let col = self.col.as_mut().ok_or("Collection not open")?;
        let queued = col
            .get_queued_cards(2, false)
            .map_err(|e| format!("Failed to fetch queued cards: {:?}", e))?;

        if queued.cards.is_empty() {
            self.current_card_id = None;
            self.current_states = None;
            return Ok(None);
        }

        let queued_card = &queued.cards[0];
        let cid = queued_card.card.id();
        let states = queued_card.states.clone();
        self.current_card_id = Some(cid);
        self.card_shown_at = Some(Instant::now());

        let intervals = col
            .describe_next_states(&states)
            .unwrap_or_else(|_| vec!["<10分".into(), "1天".into(), "3天".into(), "5天".into()]);
        self.current_states = Some(states);

        let rendered = col
            .render_existing_card(cid, false, false)
            .map_err(|e| format!("Failed to render card {}: {:?}", cid, e))?;

        let media_uri_prefix = format!("file://{}/", self.media_dir.display());
        let current = CardPayload {
            card_id: cid.0,
            question_html: rewrite_media_urls(&rendered.question(), &media_uri_prefix),
            answer_html: rewrite_media_urls(&rendered.answer(), &media_uri_prefix),
            css: rendered.css,
            intervals,
        };

        let next = if queued.cards.len() > 1 {
            let next_card = &queued.cards[1];
            let next_cid = next_card.card.id();
            let next_intervals = col
                .describe_next_states(&next_card.states)
                .unwrap_or_else(|_| vec!["<10分".into(), "1天".into(), "3天".into(), "5天".into()]);
            if let Ok(next_rendered) = col.render_existing_card(next_cid, false, false) {
                Some(CardPayload {
                    card_id: next_cid.0,
                    question_html: rewrite_media_urls(&next_rendered.question(), &media_uri_prefix),
                    answer_html: rewrite_media_urls(&next_rendered.answer(), &media_uri_prefix),
                    css: next_rendered.css,
                    intervals: next_intervals,
                })
            } else {
                None
            }
        } else {
            None
        };

        let deck_name = col.get_current_deck().map(|d| d.name.clone()).unwrap_or_else(|_| "复习".into());
        let counts = col.current_deck_tree()
            .ok()
            .flatten()
            .map(|t| (t.new_count, t.learn_count, t.review_count))
            .unwrap_or((0, 0, 0));

        Ok(Some(CardPipelinePayload {
            current,
            next,
            deck_name,
            new_count: counts.0,
            learn_count: counts.1,
            review_count: counts.2,
        }))
    }

    pub fn rate_and_fetch_next(&mut self, rating_str: &str) -> Result<Option<CardPayload>, String> {
        self.rate_card(rating_str)?;

        let col = self.col.as_mut().ok_or("Collection not open")?;
        let queued = col
            .get_queued_cards(2, false)
            .map_err(|e| format!("Failed to fetch queued cards: {:?}", e))?;

        if queued.cards.is_empty() {
            self.current_card_id = None;
            self.current_states = None;
            return Ok(None);
        }

        let current_card = &queued.cards[0];
        let cid = current_card.card.id();
        self.current_card_id = Some(cid);
        self.current_states = Some(current_card.states.clone());
        self.card_shown_at = Some(Instant::now());

        if queued.cards.len() > 1 {
            let next_card = &queued.cards[1];
            let next_cid = next_card.card.id();
            let next_intervals = col
                .describe_next_states(&next_card.states)
                .unwrap_or_else(|_| vec!["<10分".into(), "1天".into(), "3天".into(), "5天".into()]);
            if let Ok(next_rendered) = col.render_existing_card(next_cid, false, false) {
                let media_uri_prefix = format!("file://{}/", self.media_dir.display());
                return Ok(Some(CardPayload {
                    card_id: next_cid.0,
                    question_html: rewrite_media_urls(&next_rendered.question(), &media_uri_prefix),
                    answer_html: rewrite_media_urls(&next_rendered.answer(), &media_uri_prefix),
                    css: next_rendered.css,
                    intervals: next_intervals,
                }));
            }
        }

        Ok(None)
    }

    pub fn rate_card(&mut self, rating_str: &str) -> Result<(), String> {
        let cid = self.current_card_id.ok_or("No active card to rate")?;
        let states = self.current_states.take().ok_or("No active scheduling states")?;
        let col = self.col.as_mut().ok_or("Collection not open")?;

        let rating = match rating_str.to_lowercase().as_str() {
            "again" => Rating::Again,
            "hard" => Rating::Hard,
            "good" => Rating::Good,
            "easy" => Rating::Easy,
            _ => return Err(format!("Invalid rating: {}", rating_str)),
        };

        let new_state = match rating {
            Rating::Again => states.again,
            Rating::Hard => states.hard,
            Rating::Good => states.good,
            Rating::Easy => states.easy,
        };

        let elapsed_ms = self
            .card_shown_at
            .map(|t| t.elapsed().as_millis() as u32)
            .unwrap_or(1000);

        let mut answer = CardAnswer {
            card_id: cid,
            current_state: states.current,
            new_state,
            rating,
            answered_at: TimestampMillis::now(),
            milliseconds_taken: elapsed_ms,
            custom_data: None,
            from_queue: true,
        };

        col.answer_card(&mut answer)
            .map_err(|e| format!("Failed to answer card {}: {:?}", cid, e))?;

        self.current_card_id = None;
        println!("[ANKI] Rated card {} as {:?}", cid.0, rating_str);
        Ok(())
    }

    pub fn wal_checkpoint(&mut self) -> Result<(), String> {
        if let Some(col) = self.col.as_mut() {
            col.storage
                .db()
                .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
                .map_err(|e| format!("WAL checkpoint failed: {:?}", e))?;
            println!("[ANKI] WAL checkpoint completed.");
        }
        Ok(())
    }
}

fn flatten_deck_tree(
    nodes: &[anki_proto::decks::DeckTreeNode],
    depth: usize,
    prefix: &str,
    list: &mut Vec<DeckItem>,
) {
    for child in nodes {
        let due = child.new_count + child.learn_count + child.review_count;
        let full_name = if prefix.is_empty() {
            child.name.clone()
        } else {
            format!("{}::{}", prefix, child.name)
        };
        list.push(DeckItem {
            id: child.deck_id,
            name: child.name.clone(),
            full_name: full_name.clone(),
            depth,
            new_count: child.new_count,
            learn_count: child.learn_count,
            review_count: child.review_count,
            due_count: due,
        });
        flatten_deck_tree(&child.children, depth + 1, &full_name, list);
    }
}

fn rewrite_media_urls(html: &str, media_prefix: &str) -> String {
    let mut out = String::with_capacity(html.len() + 128);
    let mut rest = html;

    while let Some(idx) = rest.find("<img ") {
        out.push_str(&rest[..idx]);
        let tag_rest = &rest[idx..];
        if let Some(src_idx) = tag_rest.find("src=\"") {
            let src_start = src_idx + 5;
            out.push_str(&tag_rest[..src_start]);
            let after_src = &tag_rest[src_start..];
            if !after_src.starts_with("http://")
                && !after_src.starts_with("https://")
                && !after_src.starts_with("file://")
                && !after_src.starts_with("data:")
            {
                out.push_str(media_prefix);
            }
            rest = after_src;
        } else {
            out.push_str("<img ");
            rest = &tag_rest[5..];
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rewrite_media_urls() {
        let input = r#"<p>Formula</p><img src="paste-1.png"> text <img src="file:///already/abs.png">"#;
        let output = rewrite_media_urls(input, "file:///mnt/us/media/");
        assert!(output.contains(r#"src="file:///mnt/us/media/paste-1.png""#));
        assert!(output.contains(r#"src="file:///already/abs.png""#));
    }
}
