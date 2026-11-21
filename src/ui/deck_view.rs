//! Page 0: 100% PangoCairo Minimalist Auto-Expanded Paginated Deck Browser
//! - Centered 960px paper-book column, 10 generous rows/page, zero scrollbars, zero ghosting
//! - All subdecks auto-expanded by default (optional chevron tap to collapse/expand)
//! - Supports both footer pagination buttons and swipe gestures (Up/Down or Left/Right)
use std::ffi::CString;
use std::ptr;

use crate::ffi::cairo::*;
use crate::ffi::gtk::*;
use crate::logic::{classify_deck_swipe, DeckBrowserState, DeckSwipe};
use crate::ui::theme::*;

const COL_X: f64 = 56.0;
const COL_W: f64 = 960.0;
const ROW_TOP_Y: f64 = 16.0;
const ROW_H: f64 = 114.0;

// Top bar action pill hitboxes (1072x84)
const BTN_SYNC: Rect = Rect::new(626.0, 16.0, 96.0, 52.0);
const BTN_SETTINGS: Rect = Rect::new(734.0, 16.0, 96.0, 52.0);
const BTN_REFRESH: Rect = Rect::new(842.0, 16.0, 94.0, 52.0);
const BTN_EXIT: Rect = Rect::new(948.0, 16.0, 94.0, 52.0);

// Footer pager pill hitboxes (1072x112)
const BTN_PREV_PAGE: Rect = Rect::new(244.0, 12.0, 164.0, 52.0);
const BTN_NEXT_PAGE: Rect = Rect::new(664.0, 12.0, 164.0, 52.0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeckAction {
    None,
    Sync,
    OpenSettings,
    RefreshScreen,
    ExitApp,
    SelectDeck(i64),
    ToggleCollapse(String),
    PrevPage,
    NextPage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TopPress {
    None,
    Sync,
    Settings,
    Refresh,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FooterPress {
    None,
    Prev,
    Next,
}

pub struct DeckView {
    pub root: *mut GtkWidget,
    pub top_area: *mut GtkWidget,
    pub list_area: *mut GtkWidget,
    pub footer_area: *mut GtkWidget,
    pub state: DeckBrowserState,
    pub status_banner: Option<String>,
    top_pressed: TopPress,
    footer_pressed: FooterPress,
    pressed_row_idx: Option<usize>,
    drag_start: Option<(f64, f64)>,
}

impl DeckView {
    pub unsafe fn new(
        top_expose: unsafe extern "C" fn(*mut GtkWidget, *mut GdkEventExpose, gpointer) -> gboolean,
        top_press: unsafe extern "C" fn(*mut GtkWidget, *mut GdkEventButton, gpointer) -> gboolean,
        top_release: unsafe extern "C" fn(*mut GtkWidget, *mut GdkEventButton, gpointer) -> gboolean,
        list_expose: unsafe extern "C" fn(*mut GtkWidget, *mut GdkEventExpose, gpointer) -> gboolean,
        list_press: unsafe extern "C" fn(*mut GtkWidget, *mut GdkEventButton, gpointer) -> gboolean,
        list_release: unsafe extern "C" fn(*mut GtkWidget, *mut GdkEventButton, gpointer) -> gboolean,
        footer_expose: unsafe extern "C" fn(*mut GtkWidget, *mut GdkEventExpose, gpointer) -> gboolean,
        footer_press: unsafe extern "C" fn(*mut GtkWidget, *mut GdkEventButton, gpointer) -> gboolean,
        footer_release: unsafe extern "C" fn(*mut GtkWidget, *mut GdkEventButton, gpointer) -> gboolean,
    ) -> Self {
        let root = gtk_vbox_new(FALSE, 0);

        let top_area = make_interactive_area(SCREEN_W, TOP_BAR_H, top_expose, top_press, top_release);
        let list_area = make_interactive_area(
            SCREEN_W,
            SCREEN_H - TOP_BAR_H - DECK_FOOTER_H,
            list_expose,
            list_press,
            list_release,
        );
        let footer_area =
            make_interactive_area(SCREEN_W, DECK_FOOTER_H, footer_expose, footer_press, footer_release);

        gtk_box_pack_start(root, top_area, FALSE, FALSE, 0);
        gtk_box_pack_start(root, list_area, TRUE, TRUE, 0);
        gtk_box_pack_end(root, footer_area, FALSE, FALSE, 0);

        Self {
            root,
            top_area,
            list_area,
            footer_area,
            state: DeckBrowserState::new(),
            status_banner: None,
            top_pressed: TopPress::None,
            footer_pressed: FooterPress::None,
            pressed_row_idx: None,
            drag_start: None,
        }
    }

    pub unsafe fn queue_redraw_all(&self) {
        gtk_widget_queue_draw(self.top_area);
        gtk_widget_queue_draw(self.list_area);
        gtk_widget_queue_draw(self.footer_area);
    }

    pub unsafe fn draw_top(&self, cr: *mut cairo_t) {
        fill_white(cr);
        draw_hline(cr, 0.0, SCREEN_W as f64, (TOP_BAR_H as f64) - 0.75, 1.5, 0.0);

        // Left: Brand + deduplicated root due count
        draw_text_left(cr, Rect::new(32.0, 0.0, 140.0, TOP_BAR_H as f64), "RAnki", 32.0, true, 0.0, false);
        let sub = format!("待复习 {} 张", self.state.total_due);
        draw_text_left(cr, Rect::new(160.0, 2.0, 440.0, TOP_BAR_H as f64), &sub, 23.0, false, 0.35, true);

        // Right: Minimalist tactile action pills
        draw_pill_button(cr, BTN_SYNC, "同步", 24.0, true, 0.0, 0.0, self.top_pressed == TopPress::Sync);
        draw_pill_button(cr, BTN_SETTINGS, "设置", 24.0, true, 0.0, 0.0, self.top_pressed == TopPress::Settings);
        draw_pill_button(cr, BTN_REFRESH, "刷新", 24.0, false, 0.78, 0.22, self.top_pressed == TopPress::Refresh);
        draw_pill_button(cr, BTN_EXIT, "退出", 24.0, false, 0.78, 0.22, self.top_pressed == TopPress::Exit);
    }

    pub unsafe fn on_top_press(&mut self, x: f64, y: f64) {
        self.top_pressed = if BTN_SYNC.contains(x, y) {
            TopPress::Sync
        } else if BTN_SETTINGS.contains(x, y) {
            TopPress::Settings
        } else if BTN_REFRESH.contains(x, y) {
            TopPress::Refresh
        } else if BTN_EXIT.contains(x, y) {
            TopPress::Exit
        } else {
            TopPress::None
        };
        if self.top_pressed != TopPress::None {
            gtk_widget_queue_draw(self.top_area);
        }
    }

    pub unsafe fn on_top_release(&mut self, x: f64, y: f64) -> DeckAction {
        let was = self.top_pressed;
        self.top_pressed = TopPress::None;
        gtk_widget_queue_draw(self.top_area);

        match was {
            TopPress::Sync if BTN_SYNC.contains(x, y) => DeckAction::Sync,
            TopPress::Settings if BTN_SETTINGS.contains(x, y) => DeckAction::OpenSettings,
            TopPress::Refresh if BTN_REFRESH.contains(x, y) => DeckAction::RefreshScreen,
            TopPress::Exit if BTN_EXIT.contains(x, y) => DeckAction::ExitApp,
            _ => DeckAction::None,
        }
    }

    pub unsafe fn draw_list(&self, cr: *mut cairo_t) {
        fill_white(cr);

        let page_rows = self.state.current_page_slice();
        if page_rows.is_empty() {
            draw_text_centered(
                cr,
                Rect::new(COL_X, 240.0, COL_W, 80.0),
                "暂无牌组（请点击右上角「设置」登录并同步 AnkiWeb）",
                26.0,
                false,
                0.4,
            );
        } else {
            for (i, deck) in page_rows.iter().enumerate() {
                let ry = ROW_TOP_Y + (i as f64) * ROW_H;
                let pressed = self.pressed_row_idx == Some(i);

                if pressed {
                    cairo_set_source_rgb(cr, 0.0, 0.0, 0.0);
                    cairo_rectangle(cr, COL_X, ry, COL_W, ROW_H);
                    cairo_fill(cr);
                }

                let fg = if pressed { 1.0 } else { 0.0 };
                let zero_fg = if pressed { 0.8 } else { 0.62 };
                let chev_fg = if pressed { 1.0 } else { 0.30 };

                let indent = (deck.depth as f64) * 34.0;
                let mut text_x = COL_X + 12.0 + indent;

                if deck.has_children {
                    // Draw crisp Cairo vector chevron (zero font-tofu risk on Kindle OS)
                    let cx = text_x + 15.0;
                    let cy = ry + ROW_H / 2.0;
                    cairo_set_source_rgb(cr, chev_fg, chev_fg, chev_fg);
                    cairo_new_path(cr);
                    if self.state.collapsed.contains(&deck.full_name) {
                        // Right-pointing collapsed triangle
                        cairo_move_to(cr, cx - 5.0, cy - 8.0);
                        cairo_line_to(cr, cx + 7.0, cy);
                        cairo_line_to(cr, cx - 5.0, cy + 8.0);
                    } else {
                        // Down-pointing expanded triangle
                        cairo_move_to(cr, cx - 8.0, cy - 5.0);
                        cairo_line_to(cr, cx + 8.0, cy - 5.0);
                        cairo_line_to(cr, cx, cy + 7.0);
                    }
                    cairo_close_path(cr);
                    cairo_fill(cr);
                    text_x += 38.0;
                }

                let name_w = (COL_X + COL_W - 246.0 - text_x).max(160.0);
                let name_px = if deck.has_children { 31.0 } else { 29.0 };
                draw_text_left(
                    cr,
                    Rect::new(text_x, ry, name_w, ROW_H),
                    &deck.name,
                    name_px,
                    deck.has_children,
                    fg,
                    true,
                );

                // Right-aligned 3 tabular columns: New | Learn | Review
                let counts = [deck.new_count, deck.learn_count, deck.review_count];
                for (ci, &val) in counts.iter().enumerate() {
                    let cx = COL_X + COL_W - 236.0 + (ci as f64) * 78.0;
                    let s = val.to_string();
                    draw_text_right(
                        cr,
                        Rect::new(cx, ry, 68.0, ROW_H),
                        &s,
                        27.0,
                        val > 0,
                        if val > 0 { fg } else { zero_fg },
                    );
                }

                draw_hline(cr, COL_X, COL_X + COL_W, ry + ROW_H, 1.0, 0.85);
            }
        }

        // Optional clean inline status banner near bottom of list canvas
        if let Some(ref msg) = self.status_banner {
            let banner = Rect::new(COL_X + 40.0, 1170.0, COL_W - 80.0, 58.0);
            rounded_rect_path(cr, banner, 8.0);
            cairo_set_source_rgb(cr, 0.0, 0.0, 0.0);
            cairo_fill(cr);
            draw_text_centered(cr, banner, msg, 24.0, true, 1.0);
        }
    }

    pub unsafe fn on_list_press(&mut self, x: f64, y: f64) {
        self.drag_start = Some((x, y));
        self.pressed_row_idx = self.hit_row_index(y);
        if self.pressed_row_idx.is_some() {
            gtk_widget_queue_draw(self.list_area);
        }
    }

    pub unsafe fn on_list_release(&mut self, x: f64, y: f64) -> DeckAction {
        let start = self.drag_start.take();
        let was_row = self.pressed_row_idx.take();
        gtk_widget_queue_draw(self.list_area);

        if let Some((sx, sy)) = start {
            match classify_deck_swipe(sx, sy, x, y) {
                DeckSwipe::NextPage => return DeckAction::NextPage,
                DeckSwipe::PrevPage => return DeckAction::PrevPage,
                DeckSwipe::Tap => {}
            }
        }

        let row_idx = match was_row {
            Some(idx) if self.hit_row_index(y) == Some(idx) => idx,
            _ => return DeckAction::None,
        };

        let page_rows = self.state.current_page_slice();
        let deck = match page_rows.get(row_idx) {
            Some(d) => (*d).clone(),
            None => return DeckAction::None,
        };

        // Generous chevron hit area on parent decks
        if deck.has_children {
            let chev_left = COL_X + (deck.depth as f64) * 34.0;
            let chev_right = chev_left + 72.0;
            if x >= chev_left && x <= chev_right {
                return DeckAction::ToggleCollapse(deck.full_name);
            }
        }

        DeckAction::SelectDeck(deck.id)
    }

    fn hit_row_index(&self, y: f64) -> Option<usize> {
        if y < ROW_TOP_Y {
            return None;
        }
        let idx = ((y - ROW_TOP_Y) / ROW_H) as usize;
        let len = self.state.current_page_slice().len();
        if idx < len {
            Some(idx)
        } else {
            None
        }
    }

    pub unsafe fn draw_footer(&self, cr: *mut cairo_t) {
        fill_white(cr);
        draw_hline(cr, COL_X, COL_X + COL_W, 1.0, 1.5, 0.0);

        let cur_p = self.state.page + 1;
        let tot_p = self.state.total_pages();
        let has_prev = cur_p > 1;
        let has_next = cur_p < tot_p;

        draw_pill_button(
            cr,
            BTN_PREV_PAGE,
            "< 上一页",
            23.0,
            has_prev,
            if has_prev { 0.0 } else { 0.80 },
            if has_prev { 0.0 } else { 0.65 },
            has_prev && self.footer_pressed == FooterPress::Prev,
        );

        let info = format!("第 {} / {} 页", cur_p, tot_p);
        draw_text_centered(cr, Rect::new(416.0, 12.0, 240.0, 52.0), &info, 24.0, true, 0.15);

        draw_pill_button(
            cr,
            BTN_NEXT_PAGE,
            "下一页 >",
            23.0,
            has_next,
            if has_next { 0.0 } else { 0.80 },
            if has_next { 0.0 } else { 0.65 },
            has_next && self.footer_pressed == FooterPress::Next,
        );

        draw_text_centered(
            cr,
            Rect::new(COL_X, 72.0, COL_W, 34.0),
            &self.state.studied_today,
            21.0,
            false,
            0.35,
        );
    }

    pub unsafe fn on_footer_press(&mut self, x: f64, y: f64) {
        self.footer_pressed = if BTN_PREV_PAGE.contains(x, y) {
            FooterPress::Prev
        } else if BTN_NEXT_PAGE.contains(x, y) {
            FooterPress::Next
        } else {
            FooterPress::None
        };
        if self.footer_pressed != FooterPress::None {
            gtk_widget_queue_draw(self.footer_area);
        }
    }

    pub unsafe fn on_footer_release(&mut self, x: f64, y: f64) -> DeckAction {
        let was = self.footer_pressed;
        self.footer_pressed = FooterPress::None;
        gtk_widget_queue_draw(self.footer_area);

        match was {
            FooterPress::Prev if BTN_PREV_PAGE.contains(x, y) => DeckAction::PrevPage,
            FooterPress::Next if BTN_NEXT_PAGE.contains(x, y) => DeckAction::NextPage,
            _ => DeckAction::None,
        }
    }
}

unsafe fn make_interactive_area(
    w: i32,
    h: i32,
    expose_cb: unsafe extern "C" fn(*mut GtkWidget, *mut GdkEventExpose, gpointer) -> gboolean,
    press_cb: unsafe extern "C" fn(*mut GtkWidget, *mut GdkEventButton, gpointer) -> gboolean,
    release_cb: unsafe extern "C" fn(*mut GtkWidget, *mut GdkEventButton, gpointer) -> gboolean,
) -> *mut GtkWidget {
    let da = gtk_drawing_area_new();
    gtk_widget_set_size_request(da, w, h);
    gtk_widget_add_events(
        da,
        GDK_BUTTON_PRESS_MASK | GDK_BUTTON_RELEASE_MASK | GDK_POINTER_MOTION_MASK,
    );

    let sig_exp = CString::new("expose-event").unwrap();
    g_signal_connect_data(
        da as gpointer,
        sig_exp.as_ptr(),
        Some(std::mem::transmute(expose_cb as usize)),
        ptr::null_mut(),
        None,
        0,
    );
    let sig_p = CString::new("button-press-event").unwrap();
    g_signal_connect_data(
        da as gpointer,
        sig_p.as_ptr(),
        Some(std::mem::transmute(press_cb as usize)),
        ptr::null_mut(),
        None,
        0,
    );
    let sig_r = CString::new("button-release-event").unwrap();
    g_signal_connect_data(
        da as gpointer,
        sig_r.as_ptr(),
        Some(std::mem::transmute(release_cb as usize)),
        ptr::null_mut(),
        None,
        0,
    );
    da
}
