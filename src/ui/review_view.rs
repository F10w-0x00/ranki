//! Page 1: Native Review Chrome + Isolated 54x54px 20s Duo Timer + Resident Card WebView
//! - Top 84px bar: < 牌组 | Deck Name | New + Learn + Review (active underlined) | 1/2 | 撤销 | ··· | 20s Circle
//! - Middle 1236px WebKitWebView: loaded ONCE (views/card.html), GDK-level zero-bar swipe/tap paging
//! - Bottom 128px bar: 显示答案 / 4-tier (interval + 重来/困难/良好/简单) / 返回牌组大厅
use std::ffi::CString;
use std::ptr;

use anki::scheduler::answering::Rating;

use crate::anki::{ActiveQueue, QueueCounts};
use crate::ffi::cairo::*;
use crate::ffi::gtk::*;
use crate::ffi::webkit::*;
use crate::logic::{classify_card_gesture, CardGesture};
use crate::ui::focus_timer::FocusTimerWidget;
use crate::ui::theme::*;

const TOP_AREA_W: i32 = SCREEN_W - 68; // 1004px left/center + 68px isolated timer slot

// Top bar hitboxes (1004x84)
const BTN_BACK: Rect = Rect::new(20.0, 16.0, 118.0, 52.0);
const BTN_UNDO: Rect = Rect::new(782.0, 16.0, 96.0, 52.0);
const BTN_MENU: Rect = Rect::new(890.0, 16.0, 94.0, 52.0);

// Bottom bar hitboxes (1072x128)
const BTN_FULL: Rect = Rect::new(28.0, 18.0, 1016.0, 92.0);
const RATE_W: f64 = 243.5;
const RATE_GAP: f64 = 14.0;
const RATE_Y: f64 = 18.0;
const RATE_H: f64 = 92.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewStage {
    Unflipped,
    Flipped,
    Finished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewAction {
    None,
    BackToDecks,
    Undo,
    OpenMenu,
    FlipCard,
    Rate(Rating),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TopPress {
    None,
    Back,
    Undo,
    Menu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BottomPress {
    None,
    Full,
    RateIdx(usize),
}

pub struct ReviewView {
    pub root: *mut GtkWidget,
    pub top_area: *mut GtkWidget,
    pub timer: FocusTimerWidget,
    pub webview: *mut GtkWidget,
    pub bottom_area: *mut GtkWidget,

    pub stage: ReviewStage,
    pub deck_name: String,
    pub counts: QueueCounts,
    pub intervals: [String; 4],
    pub can_undo: bool,
    pub page_cur: u32,
    pub page_total: u32,

    pub active_card_id: Option<i64>,
    pub prefetched_card_id: Option<i64>,
    pub card_engine_ready: bool,

    top_pressed: TopPress,
    bottom_pressed: BottomPress,
    card_drag_start: Option<(f64, f64)>,
}

impl ReviewView {
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn new(
        card_uri: &str,
        top_expose: unsafe extern "C" fn(*mut GtkWidget, *mut GdkEventExpose, gpointer) -> gboolean,
        top_press: unsafe extern "C" fn(*mut GtkWidget, *mut GdkEventButton, gpointer) -> gboolean,
        top_release: unsafe extern "C" fn(*mut GtkWidget, *mut GdkEventButton, gpointer) -> gboolean,
        timer_expose: unsafe extern "C" fn(*mut GtkWidget, *mut GdkEventExpose, gpointer) -> gboolean,
        title_notify: unsafe extern "C" fn(*mut GtkWidget, gpointer, gpointer),
        wv_press: unsafe extern "C" fn(*mut GtkWidget, *mut GdkEventButton, gpointer) -> gboolean,
        wv_motion: unsafe extern "C" fn(*mut GtkWidget, *mut GdkEventMotion, gpointer) -> gboolean,
        wv_release: unsafe extern "C" fn(*mut GtkWidget, *mut GdkEventButton, gpointer) -> gboolean,
        bot_expose: unsafe extern "C" fn(*mut GtkWidget, *mut GdkEventExpose, gpointer) -> gboolean,
        bot_press: unsafe extern "C" fn(*mut GtkWidget, *mut GdkEventButton, gpointer) -> gboolean,
        bot_release: unsafe extern "C" fn(*mut GtkWidget, *mut GdkEventButton, gpointer) -> gboolean,
    ) -> Self {
        let root = gtk_vbox_new(FALSE, 0);
        let white = GdkColor {
            pixel: 0,
            red: 0xFFFF,
            green: 0xFFFF,
            blue: 0xFFFF,
        };
        gtk_widget_modify_bg(root, GTK_STATE_NORMAL, &white);

        // 1. Top Bar: GtkHBox [ top_area (1004x84) | isolated timer_box (68x84) ]
        let top_hbox = gtk_hbox_new(FALSE, 0);
        gtk_widget_set_size_request(top_hbox, SCREEN_W, TOP_BAR_H);
        gtk_widget_modify_bg(top_hbox, GTK_STATE_NORMAL, &white);

        let top_area = gtk_drawing_area_new();
        gtk_widget_set_size_request(top_area, TOP_AREA_W, TOP_BAR_H);
        gtk_widget_add_events(top_area, GDK_BUTTON_PRESS_MASK | GDK_BUTTON_RELEASE_MASK);
        connect_event(top_area, "expose-event", top_expose as usize);
        connect_event(top_area, "button-press-event", top_press as usize);
        connect_event(top_area, "button-release-event", top_release as usize);

        let timer = FocusTimerWidget::new(timer_expose);
        let timer_wrap = gtk_vbox_new(FALSE, 0);
        gtk_widget_set_size_request(timer_wrap, 68, TOP_BAR_H);
        gtk_widget_modify_bg(timer_wrap, GTK_STATE_NORMAL, &white);
        let timer_hbox = gtk_hbox_new(FALSE, 0);
        gtk_box_pack_start(timer_hbox, timer.drawing_area, FALSE, FALSE, 4);
        gtk_box_pack_start(timer_wrap, timer_hbox, FALSE, FALSE, 14);

        // 1.5px bottom border under the timer slot so the top-bar rule is continuous
        let timer_rule = gtk_drawing_area_new();
        gtk_widget_set_size_request(timer_rule, 68, 2);
        let black = GdkColor {
            pixel: 0,
            red: 0,
            green: 0,
            blue: 0,
        };
        gtk_widget_modify_bg(timer_rule, GTK_STATE_NORMAL, &black);
        gtk_box_pack_end(timer_wrap, timer_rule, FALSE, FALSE, 0);

        gtk_box_pack_start(top_hbox, top_area, TRUE, TRUE, 0);
        gtk_box_pack_end(top_hbox, timer_wrap, FALSE, FALSE, 0);

        // 2. Resident Card Canvas (WebKitWebView — loaded ONCE, GDK gesture interception)
        let webview = webkit_web_view_new();
        gtk_widget_add_events(
            webview,
            GDK_BUTTON_PRESS_MASK | GDK_BUTTON_RELEASE_MASK | GDK_POINTER_MOTION_MASK,
        );
        connect_event(webview, "notify::title", title_notify as usize);
        connect_event(webview, "button-press-event", wv_press as usize);
        connect_event(webview, "motion-notify-event", wv_motion as usize);
        connect_event(webview, "button-release-event", wv_release as usize);

        if let Ok(c_uri) = CString::new(card_uri) {
            println!("[REVIEW] Loading resident card canvas once: {}", card_uri);
            webkit_web_view_load_uri(webview, c_uri.as_ptr());
        }

        // 3. Bottom Action Bar (1072x128)
        let bottom_area = gtk_drawing_area_new();
        gtk_widget_set_size_request(bottom_area, SCREEN_W, BOTTOM_BAR_H);
        gtk_widget_add_events(bottom_area, GDK_BUTTON_PRESS_MASK | GDK_BUTTON_RELEASE_MASK);
        connect_event(bottom_area, "expose-event", bot_expose as usize);
        connect_event(bottom_area, "button-press-event", bot_press as usize);
        connect_event(bottom_area, "button-release-event", bot_release as usize);

        gtk_box_pack_start(root, top_hbox, FALSE, FALSE, 0);
        gtk_box_pack_start(root, webview, TRUE, TRUE, 0);
        gtk_box_pack_end(root, bottom_area, FALSE, FALSE, 0);

        Self {
            root,
            top_area,
            timer,
            webview,
            bottom_area,
            stage: ReviewStage::Unflipped,
            deck_name: "复习".to_string(),
            counts: QueueCounts {
                new: 0,
                learn: 0,
                review: 0,
                active_queue: ActiveQueue::Review,
            },
            intervals: ["<10m".into(), "1d".into(), "3d".into(), "5d".into()],
            can_undo: false,
            page_cur: 1,
            page_total: 1,
            active_card_id: None,
            prefetched_card_id: None,
            card_engine_ready: false,
            top_pressed: TopPress::None,
            bottom_pressed: BottomPress::None,
            card_drag_start: None,
        }
    }

    pub unsafe fn queue_redraw_bars(&self) {
        gtk_widget_queue_draw(self.top_area);
        gtk_widget_queue_draw(self.bottom_area);
    }

    pub unsafe fn draw_top(&self, cr: *mut cairo_t) {
        fill_white(cr);
        draw_hline(cr, 0.0, TOP_AREA_W as f64, (TOP_BAR_H as f64) - 0.75, 1.5, 0.0);

        // Back button
        draw_pill_button(
            cr,
            BTN_BACK,
            "< 牌组",
            24.0,
            true,
            0.0,
            0.0,
            self.top_pressed == TopPress::Back,
        );

        // Deck short name
        draw_text_left(
            cr,
            Rect::new(152.0, 0.0, 270.0, TOP_BAR_H as f64),
            &self.deck_name,
            26.0,
            true,
            0.0,
            true,
        );

        // Official Anki 3-queue counter: New + Learn + Review (active queue underlined!)
        let mut cx = 434.0;
        let parts = [
            (self.counts.new.to_string(), ActiveQueue::New),
            (self.counts.learn.to_string(), ActiveQueue::Learn),
            (self.counts.review.to_string(), ActiveQueue::Review),
        ];
        for (idx, (num_str, q_kind)) in parts.iter().enumerate() {
            if idx > 0 {
                let (pw, _) = draw_text_left(
                    cr,
                    Rect::new(cx, 0.0, 40.0, TOP_BAR_H as f64),
                    " + ",
                    24.0,
                    false,
                    0.40,
                    false,
                );
                cx += pw as f64;
            }
            let active = self.stage != ReviewStage::Finished && self.counts.active_queue == *q_kind;
            let (nw, nh) = draw_text_left(
                cr,
                Rect::new(cx, 0.0, 120.0, TOP_BAR_H as f64),
                num_str,
                25.0,
                active,
                if active { 0.0 } else { 0.25 },
                false,
            );
            if active && nw > 0 {
                let uy = ((TOP_BAR_H as f64) + (nh as f64)) / 2.0 + 3.0;
                draw_hline(cr, cx, cx + (nw as f64), uy, 2.5, 0.0);
            }
            cx += nw as f64;
        }

        // Subtle long-card page indicator ONLY when card spans > 1 screen (zero paging bar!)
        if self.page_total > 1 {
            let p_str = format!("{}/{}", self.page_cur, self.page_total);
            draw_text_centered(
                cr,
                Rect::new(698.0, 0.0, 76.0, TOP_BAR_H as f64),
                &p_str,
                22.0,
                true,
                0.35,
            );
        }

        // Undo button
        draw_pill_button(
            cr,
            BTN_UNDO,
            "撤销",
            23.0,
            self.can_undo,
            if self.can_undo { 0.72 } else { 0.88 },
            if self.can_undo { 0.12 } else { 0.68 },
            self.can_undo && self.top_pressed == TopPress::Undo,
        );

        // Compact overflow menu button (Bury / Suspend / GC16 Refresh / Settings)
        draw_pill_button(
            cr,
            BTN_MENU,
            "···",
            26.0,
            true,
            0.72,
            0.12,
            self.top_pressed == TopPress::Menu,
        );
    }

    pub unsafe fn on_top_press(&mut self, x: f64, y: f64) {
        self.top_pressed = if BTN_BACK.contains(x, y) {
            TopPress::Back
        } else if self.can_undo && BTN_UNDO.contains(x, y) {
            TopPress::Undo
        } else if BTN_MENU.contains(x, y) {
            TopPress::Menu
        } else {
            TopPress::None
        };
        if self.top_pressed != TopPress::None {
            gtk_widget_queue_draw(self.top_area);
        }
    }

    pub unsafe fn on_top_release(&mut self, x: f64, y: f64) -> ReviewAction {
        let was = self.top_pressed;
        self.top_pressed = TopPress::None;
        gtk_widget_queue_draw(self.top_area);

        match was {
            TopPress::Back if BTN_BACK.contains(x, y) => ReviewAction::BackToDecks,
            TopPress::Undo if self.can_undo && BTN_UNDO.contains(x, y) => ReviewAction::Undo,
            TopPress::Menu if BTN_MENU.contains(x, y) => ReviewAction::OpenMenu,
            _ => ReviewAction::None,
        }
    }

    pub fn on_card_press(&mut self, x: f64, y: f64) {
        self.card_drag_start = Some((x, y));
    }

    pub unsafe fn on_card_release(&mut self, x: f64, y: f64) -> CardGesture {
        let start = match self.card_drag_start.take() {
            Some(s) => s,
            None => return CardGesture::None,
        };
        let mut alloc = GtkAllocation::default();
        gtk_widget_get_allocation(self.webview, &mut alloc);
        let h = if alloc.height > 100 {
            alloc.height as f64
        } else {
            1236.0
        };
        classify_card_gesture(start.0, start.1, x, y, h)
    }

    fn rate_rect(idx: usize) -> Rect {
        let x = 28.0 + (idx as f64) * (RATE_W + RATE_GAP);
        Rect::new(x, RATE_Y, RATE_W, RATE_H)
    }

    pub unsafe fn draw_bottom(&self, cr: *mut cairo_t) {
        fill_white(cr);
        draw_hline(cr, 0.0, SCREEN_W as f64, 0.75, 1.5, 0.0);

        match self.stage {
            ReviewStage::Unflipped => {
                draw_pill_button(
                    cr,
                    BTN_FULL,
                    "显  示  答  案",
                    34.0,
                    true,
                    0.0,
                    0.0,
                    self.bottom_pressed == BottomPress::Full,
                );
            }
            ReviewStage::Finished => {
                draw_pill_button(
                    cr,
                    BTN_FULL,
                    "返回牌组大厅",
                    32.0,
                    true,
                    0.0,
                    0.0,
                    self.bottom_pressed == BottomPress::Full,
                );
            }
            ReviewStage::Flipped => {
                let labels = ["重来", "困难", "良好", "简单"];
                for (i, label) in labels.iter().enumerate() {
                    let r = Self::rate_rect(i);
                    let pressed = self.bottom_pressed == BottomPress::RateIdx(i);

                    rounded_rect_path(cr, r, 10.0);
                    if pressed {
                        cairo_set_source_rgb(cr, 0.0, 0.0, 0.0);
                        cairo_fill_preserve(cr);
                        cairo_set_source_rgb(cr, 0.0, 0.0, 0.0);
                    } else {
                        cairo_set_source_rgb(cr, 1.0, 1.0, 1.0);
                        cairo_fill_preserve(cr);
                        cairo_set_source_rgb(cr, 0.0, 0.0, 0.0);
                    }
                    cairo_set_line_width(cr, 1.5);
                    cairo_stroke(cr);

                    let int_fg = if pressed { 0.85 } else { 0.30 };
                    let lbl_fg = if pressed { 1.0 } else { 0.0 };

                    draw_text_centered(
                        cr,
                        Rect::new(r.x, r.y + 8.0, r.w, 32.0),
                        &self.intervals[i],
                        23.0,
                        false,
                        int_fg,
                    );
                    draw_text_centered(
                        cr,
                        Rect::new(r.x, r.y + 40.0, r.w, 46.0),
                        label,
                        32.0,
                        true,
                        lbl_fg,
                    );
                }
            }
        }
    }

    pub unsafe fn on_bottom_press(&mut self, x: f64, y: f64) {
        self.bottom_pressed = match self.stage {
            ReviewStage::Unflipped | ReviewStage::Finished => {
                if BTN_FULL.contains(x, y) {
                    BottomPress::Full
                } else {
                    BottomPress::None
                }
            }
            ReviewStage::Flipped => {
                let mut found = BottomPress::None;
                for i in 0..4 {
                    if Self::rate_rect(i).contains(x, y) {
                        found = BottomPress::RateIdx(i);
                        break;
                    }
                }
                found
            }
        };
        if self.bottom_pressed != BottomPress::None {
            gtk_widget_queue_draw(self.bottom_area);
        }
    }

    pub unsafe fn on_bottom_release(&mut self, x: f64, y: f64) -> ReviewAction {
        let was = self.bottom_pressed;
        self.bottom_pressed = BottomPress::None;
        gtk_widget_queue_draw(self.bottom_area);

        match (self.stage, was) {
            (ReviewStage::Unflipped, BottomPress::Full) if BTN_FULL.contains(x, y) => {
                ReviewAction::FlipCard
            }
            (ReviewStage::Finished, BottomPress::Full) if BTN_FULL.contains(x, y) => {
                ReviewAction::BackToDecks
            }
            (ReviewStage::Flipped, BottomPress::RateIdx(i)) if Self::rate_rect(i).contains(x, y) => {
                let rating = match i {
                    0 => Rating::Again,
                    1 => Rating::Hard,
                    2 => Rating::Good,
                    _ => Rating::Easy,
                };
                ReviewAction::Rate(rating)
            }
            _ => ReviewAction::None,
        }
    }
}

pub unsafe fn exec_card_script(webview: *mut GtkWidget, script: &str) {
    if let Ok(c_script) = CString::new(script) {
        webkit_web_view_execute_script(webview, c_script.as_ptr());
    }
}

unsafe fn connect_event(widget: *mut GtkWidget, sig_name: &str, handler: usize) {
    let sig = CString::new(sig_name).unwrap();
    g_signal_connect_data(
        widget as gpointer,
        sig.as_ptr(),
        Some(std::mem::transmute(handler)),
        ptr::null_mut(),
        None,
        0,
    );
}
