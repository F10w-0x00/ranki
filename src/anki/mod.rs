//! Direct Zero-Copy Official Anki rslib Engine for RAnki Route B
//! - Card-ID-verified review pipeline (eliminates Again / buried sibling / Undo desync)
//! - Accurate deduplicated root due totals & official active-queue underline state
//! - Native Undo, Bury, Suspend, and power-safe SQLite WAL checkpointing
pub mod sync;

use std::path::{Path, PathBuf};
use std::time::Instant;

use anki::card::{CardId, CardQueueNumber};
use anki::collection::{Collection, CollectionBuilder};
use anki::decks::DeckId;
use anki::scheduler::answering::{CardAnswer, Rating};
use anki::scheduler::states::SchedulingStates;
use anki::timestamp::{TimestampMillis, TimestampSecs};
use anki_proto::scheduler::bury_or_suspend_cards_request::Mode as BuryOrSuspendMode;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DeckItem {
    pub id: i64,
    pub name: String,
    pub full_name: String,
    pub depth: usize,
    pub has_children: bool,
    pub new_count: u32,
    pub learn_count: u32,
    pub review_count: u32,
    pub due_count: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DeckListData {
    pub decks: Vec<DeckItem>,
    pub total_due: u32,
    pub studied_today: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveQueue {
    New,
    Learn,
    Review,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueCounts {
    pub new: u32,
    pub learn: u32,
    pub review: u32,
    pub active_queue: ActiveQueue,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CardContent {
    pub card_id: i64,
    pub question_html: String,
    pub answer_html: String,
    pub css: String,
    pub intervals: [String; 4],
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum AdvanceResult {
    /// Actual next card matches frontend's prefetched slot -> <5ms instant swap
    PrefetchHit {
        shown_card_id: i64,
        counts: QueueCounts,
        intervals: [String; 4],
        prefetch_next: Option<CardContent>,
        can_undo: bool,
        deck_name: String,
    },
    /// Queue diverged (Again / buried Cloze sibling / Undo / Bury / Suspend) -> immediate render
    RenderImmediate {
        current: CardContent,
        next: Option<CardContent>,
        counts: QueueCounts,
        can_undo: bool,
        deck_name: String,
    },
    /// Deck has no remaining due cards today
    DeckFinished {
        studied_today: String,
        can_undo: bool,
        deck_name: String,
    },
}

pub struct AnkiService {
    col: Option<Collection>,
    col_path: PathBuf,
    media_dir: PathBuf,
    media_db: PathBuf,
    current_card_id: Option<CardId>,
    card_shown_at: Option<Instant>,
    current_states: Option<SchedulingStates>,
}

impl Default for AnkiService {
    fn default() -> Self {
        Self::new()
    }
}

impl AnkiService {
    pub fn new() -> Self {
        Self {
            col: None,
            col_path: PathBuf::new(),
            media_dir: PathBuf::new(),
            media_db: PathBuf::new(),
            current_card_id: None,
            card_shown_at: None,
            current_states: None,
        }
    }

    pub fn is_open(&self) -> bool {
        self.col.is_some()
    }

    pub fn collection_paths(&self) -> (PathBuf, PathBuf, PathBuf) {
        (
            self.col_path.clone(),
            self.media_dir.clone(),
            self.media_db.clone(),
        )
    }

    pub fn open_collection(
        &mut self,
        col_path: &Path,
        media_folder: &Path,
        media_db: &Path,
    ) -> Result<(), String> {
        self.close_collection();
        let mut builder = CollectionBuilder::new(col_path);
        builder.set_media_paths(media_folder, media_db);
        let col = builder
            .build()
            .map_err(|e| format!("Failed to open collection {:?}: {:?}", col_path, e))?;
        self.col = Some(col);
        self.col_path = col_path.to_path_buf();
        self.media_dir = media_folder.to_path_buf();
        self.media_db = media_db.to_path_buf();
        self.clear_active_card();
        println!("[ANKI] Opened collection {:?}", col_path);
        Ok(())
    }

    pub fn reopen_collection(&mut self) -> Result<(), String> {
        if self.col_path.as_os_str().is_empty() {
            return Err("No collection path recorded".to_string());
        }
        let col_path = self.col_path.clone();
        let media_dir = self.media_dir.clone();
        let media_db = self.media_db.clone();
        self.open_collection(&col_path, &media_dir, &media_db)
    }

    pub fn close_collection(&mut self) {
        let _ = self.wal_checkpoint();
        if let Some(col) = self.col.take() {
            let _ = col.close(None);
        }
        self.clear_active_card();
    }

    #[cfg(test)]
    pub fn open_in_memory(&mut self) -> Result<(), String> {
        let col = CollectionBuilder::default()
            .build()
            .map_err(|e| format!("In-memory collection build failed: {:?}", e))?;
        self.col = Some(col);
        self.clear_active_card();
        Ok(())
    }

    #[cfg(test)]
    pub fn collection_mut(&mut self) -> Option<&mut Collection> {
        self.col.as_mut()
    }

    fn clear_active_card(&mut self) {
        self.current_card_id = None;
        self.current_states = None;
        self.card_shown_at = None;
    }

    pub fn active_card_id(&self) -> Option<i64> {
        self.current_card_id.map(|c| c.0)
    }

    pub fn can_undo(&self) -> bool {
        self.col
            .as_ref()
            .and_then(|c| c.can_undo())
            .is_some()
    }

    pub fn get_deck_list_data(&mut self) -> Result<DeckListData, String> {
        let col = self.col.as_mut().ok_or("Collection not open")?;
        let tree = col
            .deck_tree(Some(TimestampSecs::now()))
            .map_err(|e| format!("Failed to get deck tree: {:?}", e))?;

        // Root nodes already aggregate child counts in Anki's deck_tree; sum ONLY roots!
        let total_due: u32 = tree
            .children
            .iter()
            .map(|root| root.new_count + root.learn_count + root.review_count)
            .sum();

        let mut decks = Vec::new();
        flatten_deck_tree(&tree.children, 0, "", &mut decks);

        let studied_today = strip_invisible_bidi(
            &col.studied_today()
                .map(|s| s.replace('\n', " "))
                .unwrap_or_else(|_| "今天在 0 秒内学习了 0 张卡片".to_string()),
        );

        Ok(DeckListData {
            decks,
            total_due,
            studied_today,
        })
    }

    pub fn studied_today(&mut self) -> String {
        if let Some(col) = self.col.as_mut() {
            let raw = col
                .studied_today()
                .map(|s| s.replace('\n', " "))
                .unwrap_or_else(|_| "今天在 0 秒内学习了 0 张卡片".to_string());
            strip_invisible_bidi(&raw)
        } else {
            "今天在 0 秒内学习了 0 张卡片".to_string()
        }
    }

    pub fn enter_deck(&mut self, deck_id: i64) -> Result<AdvanceResult, String> {
        {
            let col = self.col.as_mut().ok_or("Collection not open")?;
            col.set_current_deck(DeckId(deck_id))
                .map_err(|e| format!("Failed to select deck {}: {:?}", deck_id, e))?;
        }
        println!("[ANKI] Selected deck {}", deck_id);
        self.advance_after_mutation(None)
    }

    pub fn rate_card(
        &mut self,
        expected_cid: i64,
        rating: Rating,
        prefetched_cid: Option<i64>,
    ) -> Result<AdvanceResult, String> {
        let active_cid = self.current_card_id.ok_or("No active card to rate")?;
        if active_cid.0 != expected_cid {
            return Err(format!(
                "Card ID mismatch: active={} expected={}",
                active_cid.0, expected_cid
            ));
        }
        let states = self
            .current_states
            .take()
            .ok_or("No active scheduling states")?;

        let new_state = match rating {
            Rating::Again => states.again,
            Rating::Hard => states.hard,
            Rating::Good => states.good,
            Rating::Easy => states.easy,
        };

        let elapsed_ms = self
            .card_shown_at
            .map(|t| (t.elapsed().as_millis() as u32).min(60_000))
            .unwrap_or(1000);

        let mut answer = CardAnswer {
            card_id: active_cid,
            current_state: states.current,
            new_state,
            rating,
            answered_at: TimestampMillis::now(),
            milliseconds_taken: elapsed_ms,
            custom_data: None,
            from_queue: true,
        };

        {
            let col = self.col.as_mut().ok_or("Collection not open")?;
            col.answer_card(&mut answer)
                .map_err(|e| format!("Failed to answer card {}: {:?}", active_cid.0, e))?;
        }
        self.current_card_id = None;
        println!(
            "[ANKI] Rated card {} as {}",
            active_cid.0,
            rating_label(rating)
        );

        self.advance_after_mutation(prefetched_cid)
    }

    pub fn undo(&mut self) -> Result<AdvanceResult, String> {
        {
            let col = self.col.as_mut().ok_or("Collection not open")?;
            col.undo()
                .map_err(|e| format!("Undo failed: {:?}", e))?;
        }
        self.clear_active_card();
        println!("[ANKI] Undo succeeded.");
        // Undo always invalidates any prefetched slot -> force RenderImmediate
        self.advance_after_mutation(None)
    }

    pub fn bury_current(&mut self) -> Result<AdvanceResult, String> {
        let cid = self.current_card_id.ok_or("No active card to bury")?;
        {
            let col = self.col.as_mut().ok_or("Collection not open")?;
            col.bury_or_suspend_cards(&[cid], BuryOrSuspendMode::BuryUser)
                .map_err(|e| format!("Bury card {} failed: {:?}", cid.0, e))?;
        }
        self.clear_active_card();
        println!("[ANKI] Buried card {}", cid.0);
        self.advance_after_mutation(None)
    }

    pub fn suspend_current(&mut self) -> Result<AdvanceResult, String> {
        let cid = self.current_card_id.ok_or("No active card to suspend")?;
        {
            let col = self.col.as_mut().ok_or("Collection not open")?;
            col.bury_or_suspend_cards(&[cid], BuryOrSuspendMode::Suspend)
                .map_err(|e| format!("Suspend card {} failed: {:?}", cid.0, e))?;
        }
        self.clear_active_card();
        println!("[ANKI] Suspended card {}", cid.0);
        self.advance_after_mutation(None)
    }

    /// Force an immediate re-render of the head of the current queue (self-healing PREFETCH_MISS).
    pub fn refresh_current_immediate(&mut self) -> Result<AdvanceResult, String> {
        self.advance_after_mutation(None)
    }

    fn advance_after_mutation(
        &mut self,
        prefetched_cid: Option<i64>,
    ) -> Result<AdvanceResult, String> {
        let can_undo = self.can_undo();
        let col = self.col.as_mut().ok_or("Collection not open")?;

        let deck_name = col
            .get_current_deck()
            .map(|d| short_deck_name(&d.name.human_name()))
            .unwrap_or_else(|_| "复习".to_string());

        let queued = col
            .get_queued_cards(2, false)
            .map_err(|e| format!("Failed to fetch queued cards: {:?}", e))?;

        if queued.cards.is_empty() {
            self.clear_active_card();
            let studied_today = self.studied_today();
            return Ok(AdvanceResult::DeckFinished {
                studied_today,
                can_undo,
                deck_name,
            });
        }

        let current_queued = &queued.cards[0];
        let cur_cid = current_queued.card.id();
        let active_queue = match current_queued.card.queue_number() {
            CardQueueNumber::New => ActiveQueue::New,
            CardQueueNumber::Learning => ActiveQueue::Learn,
            CardQueueNumber::Review | CardQueueNumber::Invalid => ActiveQueue::Review,
        };
        let counts = QueueCounts {
            new: queued.new_count as u32,
            learn: queued.learning_count as u32,
            review: queued.review_count as u32,
            active_queue,
        };
        let cur_intervals = extract_intervals(col, &current_queued.states);

        self.current_card_id = Some(cur_cid);
        self.current_states = Some(current_queued.states.clone());
        self.card_shown_at = Some(Instant::now());

        let next_payload = if queued.cards.len() > 1 {
            let next_q = &queued.cards[1];
            render_card_content(col, next_q.card.id(), &next_q.states).ok()
        } else {
            None
        };

        // Strict Card-ID check: ONLY use PrefetchHit when scheduler's true next card
        // exactly equals what the frontend already pre-rendered!
        if prefetched_cid == Some(cur_cid.0) {
            return Ok(AdvanceResult::PrefetchHit {
                shown_card_id: cur_cid.0,
                counts,
                intervals: cur_intervals,
                prefetch_next: next_payload,
                can_undo,
                deck_name,
            });
        }

        let current_payload = render_card_content(col, cur_cid, &current_queued.states)?;
        Ok(AdvanceResult::RenderImmediate {
            current: current_payload,
            next: next_payload,
            counts,
            can_undo,
            deck_name,
        })
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

fn render_card_content(
    col: &mut Collection,
    cid: CardId,
    states: &SchedulingStates,
) -> Result<CardContent, String> {
    let intervals = extract_intervals(col, states);
    let rendered = col
        .render_existing_card(cid, false, false)
        .map_err(|e| format!("Failed to render card {}: {:?}", cid.0, e))?;

    Ok(CardContent {
        card_id: cid.0,
        question_html: rendered.question().into_owned(),
        answer_html: rendered.answer().into_owned(),
        css: rendered.css,
        intervals,
    })
}

fn rating_label(rating: Rating) -> &'static str {
    match rating {
        Rating::Again => "Again",
        Rating::Hard => "Hard",
        Rating::Good => "Good",
        Rating::Easy => "Easy",
    }
}

fn extract_intervals(col: &mut Collection, states: &SchedulingStates) -> [String; 4] {
    let raw = col
        .describe_next_states(states)
        .unwrap_or_else(|_| vec!["<10m".into(), "1d".into(), "3d".into(), "5d".into()]);

    let get = |idx: usize, fallback: &str| -> String {
        raw.get(idx)
            .map(|s| strip_invisible_bidi(s))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| fallback.to_string())
    };
    [
        get(0, "<10m"),
        get(1, "1d"),
        get(2, "3d"),
        get(3, "5d"),
    ]
}

pub fn strip_invisible_bidi(s: &str) -> String {
    s.chars()
        .filter(|&ch| {
            ch != '\u{2068}'
                && ch != '\u{2069}'
                && ch != '\u{200B}'
                && ch != '\u{200E}'
                && ch != '\u{200F}'
                && ch != '\u{FEFF}'
        })
        .collect::<String>()
        .trim()
        .to_string()
}

pub fn short_deck_name(full_name: &str) -> String {
    full_name
        .rsplit("::")
        .next()
        .unwrap_or(full_name)
        .to_string()
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
        let has_children = !child.children.is_empty();
        list.push(DeckItem {
            id: child.deck_id,
            name: child.name.clone(),
            full_name: full_name.clone(),
            depth,
            has_children,
            new_count: child.new_count,
            learn_count: child.learn_count,
            review_count: child.review_count,
            due_count: due,
        });
        if has_children {
            flatten_deck_tree(&child.children, depth + 1, &full_name, list);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_service_with_decks() -> (AnkiService, i64, i64) {
        let mut svc = AnkiService::new();
        svc.open_in_memory().expect("open in-memory collection");
        let (parent_id, child_id) = {
            let col = svc.collection_mut().unwrap();
            let parent = col.get_or_create_normal_deck("STEM").unwrap();
            let child = col.get_or_create_normal_deck("STEM::Calculus").unwrap();
            let nts = col.get_all_notetypes().unwrap();
            let basic = nts
                .iter()
                .find(|nt| {
                    matches!(nt.config.kind(), anki::notetype::NotetypeKind::Normal)
                        && nt.fields.len() >= 2
                })
                .cloned()
                .expect("basic notetype");

            for i in 1..=3 {
                let mut note = basic.new_note();
                note.set_field(0, format!("Q{}", i)).unwrap();
                note.set_field(1, format!("A{}", i)).unwrap();
                col.add_note(&mut note, child.id).unwrap();
            }
            (parent.id.0, child.id.0)
        };
        (svc, parent_id, child_id)
    }

    #[test]
    fn deck_tree_deduplicates_root_total_due() {
        let (mut svc, _parent_id, _child_id) = setup_service_with_decks();
        let data = svc.get_deck_list_data().unwrap();
        // 3 cards live under STEM::Calculus -> root STEM has 3, child Calculus has 3 -> total_due MUST be 3, NOT 6!
        assert_eq!(data.total_due, 3);
        let stem = data.decks.iter().find(|d| d.full_name == "STEM").unwrap();
        let calc = data
            .decks
            .iter()
            .find(|d| d.full_name == "STEM::Calculus")
            .unwrap();
        assert_eq!(stem.depth, 0);
        assert!(stem.has_children);
        assert_eq!(calc.depth, 1);
        assert!(!calc.has_children);
    }

    #[test]
    fn card_id_verified_pipeline_hits_on_good_and_guards_mismatch() {
        let (mut svc, _parent, child_id) = setup_service_with_decks();
        let first = svc.enter_deck(child_id).unwrap();
        let (c1, c2) = match first {
            AdvanceResult::RenderImmediate { current, next, counts, .. } => {
                assert_eq!(counts.new, 3);
                assert_eq!(counts.active_queue, ActiveQueue::New);
                (current.card_id, next.expect("prefetched card 2").card_id)
            }
            other => panic!("Expected RenderImmediate on enter_deck, got {:?}", other),
        };

        // Rating with wrong card ID must be rejected!
        let err = svc.rate_card(c1 + 999, Rating::Good, Some(c2));
        assert!(err.is_err());

        // Rating c1 as Easy (leaves intraday learning) with prefetched c2 -> PrefetchHit!
        let second = svc.rate_card(c1, Rating::Easy, Some(c2)).unwrap();
        match second {
            AdvanceResult::PrefetchHit { shown_card_id, can_undo, .. } => {
                assert_eq!(shown_card_id, c2);
                assert!(can_undo);
            }
            other => panic!("Expected PrefetchHit for card 2, got {:?}", other),
        }
    }

    #[test]
    fn undo_bury_and_suspend_maintain_exact_state() {
        let (mut svc, _parent, child_id) = setup_service_with_decks();
        let c1 = match svc.enter_deck(child_id).unwrap() {
            AdvanceResult::RenderImmediate { current, .. } => current.card_id,
            _ => panic!("expected card"),
        };

        // Bury c1 -> moves to c2
        let c2 = match svc.bury_current().unwrap() {
            AdvanceResult::RenderImmediate { current, can_undo, .. } => {
                assert_ne!(current.card_id, c1);
                assert!(can_undo);
                current.card_id
            }
            _ => panic!("expected c2"),
        };

        // Undo bury -> brings c1 right back!
        match svc.undo().unwrap() {
            AdvanceResult::RenderImmediate { current, .. } => {
                assert_eq!(current.card_id, c1);
            }
            _ => panic!("expected c1 restored after undo"),
        }

        // Suspend c1 -> moves to c2 permanently
        match svc.suspend_current().unwrap() {
            AdvanceResult::RenderImmediate { current, .. } => {
                assert_eq!(current.card_id, c2);
            }
            _ => panic!("expected c2 after suspend"),
        }
    }

    #[test]
    fn bidi_isolate_stripping_removes_anki_invisible_chars() {
        assert_eq!(strip_invisible_bidi("\u{2068}4d\u{2069}"), "4d");
        assert_eq!(short_deck_name("理科::M1 高等数学"), "M1 高等数学");
    }
}
