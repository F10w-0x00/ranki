//! Pure-Rust testable UI state machines (zero GTK/X11 dependencies):
//! - Built-in 20s iPhone Duo-style circular focus timer (blank hollow ring at 0s)
//! - Auto-expanded paginated Deck Browser state & hit-testing
//! - GDK-level zero-bar swipe & tap-zone gesture classifier
use std::collections::HashSet;
use crate::anki::DeckItem;
use crate::config::BUILTIN_FOCUS_TIMER_SECS;

/// ============================================================================
/// 1. Built-in 20s Circular Focus Timer State Machine
/// ============================================================================
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusTimerState {
    Hidden,
    Running { remaining: u32, total: u32 },
    Expired,
}

impl FocusTimerState {
    pub fn start_card_front() -> Self {
        Self::Running {
            remaining: BUILTIN_FOCUS_TIMER_SECS,
            total: BUILTIN_FOCUS_TIMER_SECS,
        }
    }

    pub fn hide() -> Self {
        Self::Hidden
    }

    /// Advance by 1 second. Returns `true` if timer should keep ticking (`g_timeout` continues).
    pub fn tick(&mut self) -> bool {
        match *self {
            Self::Running { remaining, total } => {
                if remaining <= 1 {
                    *self = Self::Expired;
                    false
                } else {
                    *self = Self::Running {
                        remaining: remaining - 1,
                        total,
                    };
                    true
                }
            }
            Self::Hidden | Self::Expired => false,
        }
    }

    /// Fraction remaining in [0.0, 1.0] for drawing clockwise monochrome pie sector.
    /// Returns `None` when Hidden, `Some(0.0)` when Expired (clean empty hollow ring).
    pub fn remaining_fraction(&self) -> Option<f64> {
        match *self {
            Self::Hidden => None,
            Self::Expired => Some(0.0),
            Self::Running { remaining, total } => {
                if total == 0 {
                    Some(0.0)
                } else {
                    Some((remaining as f64 / total as f64).clamp(0.0, 1.0))
                }
            }
        }
    }
}

/// ============================================================================
/// 2. Auto-Expanded Paginated Deck Browser Logic (10 rows / page)
/// ============================================================================
pub const DECK_PAGE_SIZE: usize = 10;

#[derive(Debug, Clone)]
pub struct DeckBrowserState {
    pub decks: Vec<DeckItem>,
    pub total_due: u32,
    pub studied_today: String,
    /// Empty by default -> ALL subdecks are auto-expanded out-of-the-box!
    pub collapsed: HashSet<String>,
    pub page: usize, // 0-based page index
}

impl Default for DeckBrowserState {
    fn default() -> Self {
        Self::new()
    }
}

impl DeckBrowserState {
    pub fn new() -> Self {
        Self {
            decks: Vec::new(),
            total_due: 0,
            studied_today: "今天在 0 秒内学习了 0 张卡片".to_string(),
            collapsed: HashSet::new(),
            page: 0,
        }
    }

    pub fn set_data(&mut self, decks: Vec<DeckItem>, total_due: u32, studied_today: String) {
        self.decks = decks;
        self.total_due = total_due;
        self.studied_today = studied_today;
        self.clamp_page();
    }

    pub fn visible_decks(&self) -> Vec<&DeckItem> {
        self.decks
            .iter()
            .filter(|d| {
                !self.collapsed.iter().any(|parent| {
                    d.full_name.len() > parent.len() + 2
                        && d.full_name.starts_with(parent)
                        && d.full_name[parent.len()..].starts_with("::")
                })
            })
            .collect()
    }

    pub fn total_pages(&self) -> usize {
        let count = self.visible_decks().len();
        if count == 0 {
            1
        } else {
            (count + DECK_PAGE_SIZE - 1) / DECK_PAGE_SIZE
        }
    }

    pub fn clamp_page(&mut self) {
        let max_p = self.total_pages().saturating_sub(1);
        if self.page > max_p {
            self.page = max_p;
        }
    }

    pub fn current_page_slice(&self) -> Vec<&DeckItem> {
        let vis = self.visible_decks();
        let start = self.page * DECK_PAGE_SIZE;
        if start >= vis.len() {
            return Vec::new();
        }
        let end = (start + DECK_PAGE_SIZE).min(vis.len());
        vis[start..end].to_vec()
    }

    pub fn toggle_collapse(&mut self, full_name: &str) {
        if !self.collapsed.remove(full_name) {
            self.collapsed.insert(full_name.to_string());
        }
        self.clamp_page();
    }

    pub fn next_page(&mut self) -> bool {
        if self.page + 1 < self.total_pages() {
            self.page += 1;
            true
        } else {
            false
        }
    }

    pub fn prev_page(&mut self) -> bool {
        if self.page > 0 {
            self.page -= 1;
            true
        } else {
            false
        }
    }
}

/// ============================================================================
/// 3. GDK-Level Zero-Bar Swipe & Tap-Zone Gesture Classifier
/// ============================================================================
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardGesture {
    PageNext,
    PagePrev,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeckSwipe {
    NextPage,
    PrevPage,
    Tap,
}

pub fn classify_card_gesture(
    start_x: f64,
    start_y: f64,
    end_x: f64,
    end_y: f64,
    viewport_height: f64,
) -> CardGesture {
    let dx = end_x - start_x;
    let dy = end_y - start_y;

    // 1. Vertical swipe (Up -> Next Page, Down -> Prev Page)
    if dy.abs() >= 45.0 && dy.abs() > dx.abs() {
        return if dy < 0.0 {
            CardGesture::PageNext
        } else {
            CardGesture::PagePrev
        };
    }

    // 2. Clean tap in bottom 35% (PageNext) or top 35% (PagePrev); center 30% is safe no-op
    if dx.abs() < 25.0 && dy.abs() < 25.0 && viewport_height > 0.0 {
        let rel_y = end_y / viewport_height;
        if rel_y > 0.65 {
            return CardGesture::PageNext;
        }
        if rel_y < 0.35 {
            return CardGesture::PagePrev;
        }
    }

    CardGesture::None
}

pub fn classify_deck_swipe(start_x: f64, start_y: f64, end_x: f64, end_y: f64) -> DeckSwipe {
    let dx = end_x - start_x;
    let dy = end_y - start_y;

    if dy.abs() >= 60.0 && dy.abs() > dx.abs() {
        return if dy < 0.0 {
            DeckSwipe::NextPage
        } else {
            DeckSwipe::PrevPage
        };
    }
    if dx.abs() >= 60.0 && dx.abs() > dy.abs() {
        return if dx < 0.0 {
            DeckSwipe::NextPage
        } else {
            DeckSwipe::PrevPage
        };
    }
    DeckSwipe::Tap
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_deck(id: i64, full: &str, depth: usize, has_children: bool) -> DeckItem {
        let name = full.rsplit("::").next().unwrap_or(full).to_string();
        DeckItem {
            id,
            name,
            full_name: full.to_string(),
            depth,
            has_children,
            new_count: 1,
            learn_count: 0,
            review_count: 2,
            due_count: 3,
        }
    }

    #[test]
    fn focus_timer_ticks_20s_to_empty_expired_ring_and_hides_on_flip() {
        let mut t = FocusTimerState::start_card_front();
        assert_eq!(t.remaining_fraction(), Some(1.0));
        for step in 0..19 {
            assert!(t.tick(), "step {} should keep ticking", step);
        }
        assert_eq!(t, FocusTimerState::Running { remaining: 1, total: 20 });
        assert!(!t.tick(), "20th tick transitions to Expired and stops timer");
        assert_eq!(t, FocusTimerState::Expired);
        assert_eq!(t.remaining_fraction(), Some(0.0));
        t = FocusTimerState::hide();
        assert_eq!(t.remaining_fraction(), None);
    }

    #[test]
    fn deck_browser_auto_expands_all_subdecks_by_default_and_paginates() {
        let mut st = DeckBrowserState::new();
        let mut items = vec![
            mk_deck(1, "STEM", 0, true),
            mk_deck(2, "STEM::Math", 1, false),
            mk_deck(3, "STEM::Physics", 1, false),
        ];
        for i in 4..=14 {
            items.push(mk_deck(i, &format!("Deck{}", i), 0, false));
        }
        st.set_data(items, 36, "today".into());
        // Auto-expanded by default: all 14 rows visible -> 2 pages
        assert_eq!(st.visible_decks().len(), 14);
        assert_eq!(st.total_pages(), 2);
        assert_eq!(st.current_page_slice().len(), 10);
        assert!(st.next_page());
        assert_eq!(st.current_page_slice().len(), 4);

        // Optional manual collapse hides children cleanly
        st.toggle_collapse("STEM");
        assert_eq!(st.visible_decks().len(), 12);
    }

    #[test]
    fn gesture_classifier_supports_swipe_and_safe_tap_zones() {
        assert_eq!(classify_card_gesture(500.0, 800.0, 505.0, 700.0, 1200.0), CardGesture::PageNext);
        assert_eq!(classify_card_gesture(500.0, 400.0, 500.0, 520.0, 1200.0), CardGesture::PagePrev);
        assert_eq!(classify_card_gesture(500.0, 1000.0, 502.0, 1004.0, 1200.0), CardGesture::PageNext);
        assert_eq!(classify_card_gesture(500.0, 200.0, 503.0, 202.0, 1200.0), CardGesture::PagePrev);
        // Middle 30% tap is a safe no-op
        assert_eq!(classify_card_gesture(500.0, 600.0, 501.0, 601.0, 1200.0), CardGesture::None);
    }
}
