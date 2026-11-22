//! Route B Native GTK2/PangoCairo Application Controller
//! Coordinates DeckView (Page 0), ReviewView (Page 1), SettingsView (Page 2),
//! AnkiService (direct rslib), background async sync/login, and Kindle e-ink hardware.
pub mod deck_view;
pub mod eink;
pub mod focus_timer;
pub mod review_view;
pub mod settings_view;
pub mod theme;

use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::path::PathBuf;
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};

use anki::scheduler::answering::Rating;

use crate::anki::sync::{
    spawn_collection_sync, spawn_login, LoginOutcome, SyncMode, SyncOutcome,
};
use crate::anki::{AdvanceResult, AnkiService};
use crate::config::{AppConfig, MAX_FONT_SIZE_PX, MIN_FONT_SIZE_PX};
use crate::ffi::cairo::*;
use crate::ffi::gtk::*;
use crate::ffi::webkit::webkit_web_view_get_title;
use crate::i18n::UiLang;
use crate::logic::CardGesture;
use crate::ui::deck_view::{DeckAction, DeckView};
use crate::ui::review_view::{exec_card_script, ReviewAction, ReviewStage, ReviewView};
use crate::ui::settings_view::{SettingsAction, SettingsView};
use crate::ui::theme::{
    draw_hline, draw_pill_button, draw_text_wrapped, fill_white, register_app_fonts, Rect,
    SCREEN_H, SCREEN_W,
};

const PAGE_DECKS: i32 = 0;
const PAGE_REVIEW: i32 = 1;
const PAGE_SETTINGS: i32 = 2;

const RESP_CANCEL: i32 = 0;
const RESP_BURY: i32 = 1;
const RESP_SUSPEND: i32 = 2;
const RESP_REFRESH: i32 = 3;
const RESP_SETTINGS: i32 = 4;
const RESP_FULL_DOWNLOAD: i32 = 10;
const RESP_FULL_UPLOAD: i32 = 11;

static LAST_CARD_TOKEN: AtomicU64 = AtomicU64::new(0);

struct ChoiceDialogState {
    dialog: *mut GtkWidget,
    area: *mut GtkWidget,
    title: String,
    message: String,
    buttons: Vec<(String, i32)>,
    button_rects: Vec<Rect>,
    pressed_idx: Option<usize>,
    lang: UiLang,
    width: f64,
    height: f64,
}

thread_local! {
    static APP: RefCell<Option<AppController>> = const { RefCell::new(None) };
    static CHOICE_DIALOG: RefCell<Option<ChoiceDialogState>> = const { RefCell::new(None) };
}

struct AppController {
    window: *mut GtkWidget,
    notebook: *mut GtkWidget,
    deck_view: DeckView,
    review_view: ReviewView,
    settings_view: SettingsView,

    config: AppConfig,
    anki: AnkiService,
    needs_clean_refresh: bool,
    pending_refresh_timer: Option<guint>,
    settings_return_page: i32,
    pending_card_script: Option<String>,

    sync_rx: Option<Receiver<SyncOutcome>>,
    login_rx: Option<Receiver<LoginOutcome>>,
    banner_clear_timer: Option<guint>,
}

fn with_app<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut AppController) -> R,
{
    APP.with(|cell| {
        if let Ok(mut opt) = cell.try_borrow_mut() {
            opt.as_mut().map(f)
        } else {
            None
        }
    })
}

fn with_app_ref<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&AppController) -> R,
{
    APP.with(|cell| {
        if let Ok(opt) = cell.try_borrow() {
            opt.as_ref().map(f)
        } else {
            None
        }
    })
}

pub fn run_application() {
    println!("====================================================");
    println!(" RAnki Route B — Native GTK2/Cairo + MathJax Host   ");
    println!(" Target: Kindle Paperwhite 3 (1072x1448 @ 300 PPI)  ");
    println!("====================================================");

    let config = AppConfig::load();
    let (col_path, media_path, media_db_path) = config.get_resolved_collection_paths();
    let mut anki = AnkiService::new();

    println!("[HOST] Collection path: {:?}", col_path);
    if col_path.exists() {
        if let Err(e) = anki.open_collection(&col_path, &media_path, &media_db_path) {
            eprintln!("[ANKI] Warning: could not open collection: {}", e);
        }
    } else {
        println!("[ANKI] Collection not found at {:?}; waiting for sync or test DB.", col_path);
    }

    let views_dir = resolve_views_dir();
    register_app_fonts(&views_dir);
    let card_uri = format!("file://{}", views_dir.join("card.html").display());

    unsafe {
        if std::env::var("DISPLAY").map(|v| v.is_empty()).unwrap_or(true) {
            std::env::set_var("DISPLAY", ":0");
        }

        gtk_init(&mut 0, ptr::null_mut());

        let window = gtk_window_new(GTK_WINDOW_TOPLEVEL);
        let win_title = CString::new("L:A_N:application_ID:ranki.ranki.ranki_O:U_PC:N").unwrap();
        gtk_window_set_title(window, win_title.as_ptr());
        gtk_window_set_default_size(window, SCREEN_W, SCREEN_H);
        gtk_window_set_decorated(window, FALSE);

        let white = GdkColor {
            pixel: 0,
            red: 0xFFFF,
            green: 0xFFFF,
            blue: 0xFFFF,
        };
        gtk_widget_modify_bg(window, GTK_STATE_NORMAL, &white);

        let sig_destroy = CString::new("destroy").unwrap();
        g_signal_connect_data(
            window as gpointer,
            sig_destroy.as_ptr(),
            Some(std::mem::transmute(on_window_destroy as usize)),
            ptr::null_mut(),
            None,
            0,
        );

        let notebook = gtk_notebook_new();
        gtk_notebook_set_show_tabs(notebook, FALSE);
        gtk_notebook_set_show_border(notebook, FALSE);
        gtk_widget_modify_bg(notebook, GTK_STATE_NORMAL, &white);

        let mut deck_view = DeckView::new(
            on_deck_top_expose,
            on_deck_top_press,
            on_deck_top_release,
            on_deck_list_expose,
            on_deck_list_press,
            on_deck_list_release,
            on_deck_footer_expose,
            on_deck_footer_press,
            on_deck_footer_release,
        );

        let review_view = ReviewView::new(
            &card_uri,
            on_review_top_expose,
            on_review_top_press,
            on_review_top_release,
            on_timer_expose,
            on_card_title_notify,
            on_card_press,
            on_card_motion,
            on_card_release,
            on_review_bottom_expose,
            on_review_bottom_press,
            on_review_bottom_release,
        );

        let mut settings_view = SettingsView::new(
            on_settings_top_expose,
            on_settings_top_press,
            on_settings_top_release,
            on_entry_focus_in,
            on_entry_focus_out,
            on_settings_ctrl_expose,
            on_settings_ctrl_press,
            on_settings_ctrl_release,
        );

        if anki.is_open() {
            if let Ok(data) = anki.get_deck_list_data() {
                deck_view.state.set_data(data.decks, data.total_due, data.studied_today);
            }
        }
        settings_view.sync_from_config(&config, None);

        gtk_notebook_append_page(notebook, deck_view.root, ptr::null_mut());
        gtk_notebook_append_page(notebook, review_view.root, ptr::null_mut());
        gtk_notebook_append_page(notebook, settings_view.root, ptr::null_mut());
        gtk_container_add(window, notebook);

        APP.with(|cell| {
            *cell.borrow_mut() = Some(AppController {
                window,
                notebook,
                deck_view,
                review_view,
                settings_view,
                config,
                anki,
                needs_clean_refresh: false,
                pending_refresh_timer: None,
                settings_return_page: PAGE_DECKS,
                pending_card_script: None,
                sync_rx: None,
                login_rx: None,
                banner_clear_timer: None,
            });
        });

        gtk_widget_show_all(window);
        gtk_notebook_set_current_page(notebook, PAGE_DECKS);
        // Clean cold-start GC16 flash after Xorg settles initial Deck Browser (avoids EPDC collision)
        schedule_clean_refresh(450);
        println!("[HOST] Native GTK2/Cairo window shown. Entering gtk_main()...");
        gtk_main();
        println!("[HOST] Cleanly exited gtk_main().");
    }
}

fn resolve_views_dir() -> PathBuf {
    let device_views = PathBuf::from("/mnt/us/extensions/ranki-dev/views");
    if device_views.join("card.html").exists() {
        return device_views;
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let sibling = dir.join("views");
            if sibling.join("card.html").exists() {
                return sibling;
            }
        }
    }
    std::env::current_dir()
        .unwrap_or_default()
        .join("views")
}

// ============================================================================
// High-Level Coordinated Actions (Never hold RefCell across execute_script/dialog!)
// ============================================================================

fn run_card_js(script: &str) {
    let wv = with_app(|app| {
        if app.review_view.card_engine_ready {
            Some(app.review_view.webview)
        } else {
            app.pending_card_script = Some(script.to_string());
            None
        }
    })
    .flatten();

    if let Some(webview) = wv {
        unsafe { exec_card_script(webview, script) };
    }
}

fn push_card_config_to_webview() {
    let info = with_app_ref(|app| {
        let json = serde_json::json!({
            "font_size_px": app.config.font_size_px,
            "lang": app.config.language.as_code()
        })
        .to_string();
        let script = format!("if(window.CardEngine){{window.CardEngine.setConfig({});}}", json);
        (app.review_view.card_engine_ready, app.review_view.webview, script)
    });
    if let Some((true, wv, script)) = info {
        unsafe { exec_card_script(wv, &script) };
    }
}

/// Debounced hardware GC16 clean refresh (waits delay_ms so GTK2/WebKit finishes painting first).
fn schedule_clean_refresh(delay_ms: u32) {
    with_app(|app| unsafe {
        if let Some(id) = app.pending_refresh_timer.take() {
            g_source_remove(id);
        }
        app.pending_refresh_timer =
            Some(g_timeout_add(delay_ms, on_scheduled_clean_refresh, ptr::null_mut()));
    });
}

unsafe extern "C" fn on_scheduled_clean_refresh(_data: gpointer) -> gboolean {
    with_app(|app| {
        app.pending_refresh_timer = None;
    });
    eink::trigger_full_refresh();
    FALSE
}

fn show_deck_banner(msg: &str, auto_clear_ms: Option<u32>) {
    with_app(|app| {
        unsafe {
            if let Some(id) = app.banner_clear_timer.take() {
                g_source_remove(id);
            }
            app.deck_view.status_banner = Some(msg.to_string());
            gtk_widget_queue_draw(app.deck_view.list_area);
            if let Some(ms) = auto_clear_ms {
                app.banner_clear_timer =
                    Some(g_timeout_add(ms, on_clear_deck_banner, ptr::null_mut()));
            }
        }
    });
}

fn apply_advance_result(res: AdvanceResult) {
    let script_opt = with_app(|app| {
        match res {
            AdvanceResult::PrefetchHit {
                shown_card_id,
                counts,
                intervals,
                prefetch_next,
                can_undo,
                deck_name,
            } => {
                app.review_view.stage = ReviewStage::Unflipped;
                app.review_view.deck_name = deck_name;
                app.review_view.counts = counts;
                app.review_view.intervals = intervals;
                app.review_view.can_undo = can_undo;
                app.review_view.page_cur = 1;
                app.review_view.page_total = 1;
                app.review_view.active_card_id = Some(shown_card_id);
                app.review_view.prefetched_card_id = prefetch_next.as_ref().map(|c| c.card_id);

                unsafe {
                    app.review_view.timer.start_front(on_focus_timer_tick);
                    app.review_view.queue_redraw_bars();
                    gtk_notebook_set_current_page(app.notebook, PAGE_REVIEW);
                }

                let payload = serde_json::json!({
                    "expected_card_id": shown_card_id,
                    "prefetch_next": prefetch_next
                })
                .to_string();
                Some(format!(
                    "if(window.CardEngine){{window.CardEngine.swapToPrefetched({});}}",
                    payload
                ))
            }
            AdvanceResult::RenderImmediate {
                current,
                next,
                counts,
                can_undo,
                deck_name,
            } => {
                app.review_view.stage = ReviewStage::Unflipped;
                app.review_view.deck_name = deck_name;
                app.review_view.counts = counts;
                app.review_view.intervals = current.intervals.clone();
                app.review_view.can_undo = can_undo;
                app.review_view.page_cur = 1;
                app.review_view.page_total = 1;
                app.review_view.active_card_id = Some(current.card_id);
                app.review_view.prefetched_card_id = next.as_ref().map(|c| c.card_id);

                unsafe {
                    app.review_view.timer.start_front(on_focus_timer_tick);
                    app.review_view.queue_redraw_bars();
                    gtk_notebook_set_current_page(app.notebook, PAGE_REVIEW);
                }

                let payload = serde_json::json!({
                    "current": current,
                    "next": next
                })
                .to_string();
                Some(format!(
                    "if(window.CardEngine){{window.CardEngine.renderImmediate({});}}",
                    payload
                ))
            }
            AdvanceResult::DeckFinished {
                studied_today,
                can_undo,
                deck_name,
            } => {
                app.review_view.stage = ReviewStage::Finished;
                app.review_view.deck_name = deck_name;
                app.review_view.can_undo = can_undo;
                app.review_view.page_cur = 1;
                app.review_view.page_total = 1;
                app.review_view.active_card_id = None;
                app.review_view.prefetched_card_id = None;

                unsafe {
                    app.review_view.timer.hide();
                    app.review_view.queue_redraw_bars();
                    gtk_notebook_set_current_page(app.notebook, PAGE_REVIEW);
                }

                let fallback_msg = app.config.language.studied_today_fallback();
                let msg_json = serde_json::to_string(&studied_today)
                    .unwrap_or_else(|_| format!("{:?}", fallback_msg));
                Some(format!(
                    "if(window.CardEngine){{window.CardEngine.showFinished({});}}",
                    msg_json
                ))
            }
        }
    })
    .flatten();

    if let Some(script) = script_opt {
        run_card_js(&script);
    }
}

fn handle_deck_action(action: DeckAction) {
    match action {
        DeckAction::None => {}
        DeckAction::RefreshScreen => eink::trigger_full_refresh(),
        DeckAction::ExitApp => {
            with_app(|app| {
                unsafe { app.review_view.timer.hide() };
                app.anki.close_collection();
                unsafe { gtk_main_quit() };
            });
        }
        DeckAction::OpenSettings => open_settings_page(PAGE_DECKS, None),
        DeckAction::Sync => start_collection_sync(SyncMode::Normal),
        DeckAction::PrevPage => {
            with_app(|app| {
                if app.deck_view.state.prev_page() {
                    unsafe { app.deck_view.queue_redraw_all() };
                }
            });
        }
        DeckAction::NextPage => {
            with_app(|app| {
                if app.deck_view.state.next_page() {
                    unsafe { app.deck_view.queue_redraw_all() };
                }
            });
        }
        DeckAction::ToggleCollapse(full_name) => {
            with_app(|app| {
                app.deck_view.state.toggle_collapse(&full_name);
                unsafe { app.deck_view.queue_redraw_all() };
            });
        }
        DeckAction::SelectDeck(did) => {
            let lang = with_app_ref(|app| app.config.language).unwrap_or_default();
            let result = with_app(|app| {
                if !app.anki.is_open() {
                    return Err(app.config.language.banner_db_not_open().to_string());
                }
                app.needs_clean_refresh = true;
                app.anki.enter_deck(did)
            });

            match result {
                Some(Ok(AdvanceResult::DeckFinished { .. })) => {
                    show_deck_banner(lang.banner_deck_done(), Some(3000));
                }
                Some(Ok(adv)) => {
                    apply_advance_result(adv);
                }
                Some(Err(e)) => {
                    show_deck_banner(&e, Some(4000));
                }
                None => {}
            }
        }
    }
}

fn handle_review_action(action: ReviewAction) {
    match action {
        ReviewAction::None => {}
        ReviewAction::BackToDecks => {
            with_app(|app| {
                unsafe { app.review_view.timer.hide() };
                let _ = app.anki.wal_checkpoint();
                if let Ok(data) = app.anki.get_deck_list_data() {
                    app.deck_view
                        .state
                        .set_data(data.decks, data.total_due, data.studied_today);
                }
                unsafe {
                    app.deck_view.queue_redraw_all();
                    gtk_notebook_set_current_page(app.notebook, PAGE_DECKS);
                }
            });
            schedule_clean_refresh(450);
        }
        ReviewAction::FlipCard => {
            let wv = with_app(|app| {
                if app.review_view.stage != ReviewStage::Unflipped {
                    return None;
                }
                app.review_view.stage = ReviewStage::Flipped;
                unsafe {
                    app.review_view.timer.hide();
                    app.review_view.queue_redraw_bars();
                }
                Some(app.review_view.webview)
            })
            .flatten();
            if let Some(webview) = wv {
                unsafe {
                    exec_card_script(
                        webview,
                        "if(window.CardEngine){window.CardEngine.flipToAnswer();}",
                    );
                }
            }
        }
        ReviewAction::Rate(rating) => {
            let outcome = with_app(|app| {
                let cid = app.review_view.active_card_id?;
                let prefetched = app.review_view.prefetched_card_id;
                Some(app.anki.rate_card(cid, rating, prefetched))
            })
            .flatten();

            if let Some(res) = outcome {
                match res {
                    Ok(adv) => apply_advance_result(adv),
                    Err(e) => eprintln!("[ANKI] rate_card error: {}", e),
                }
            }
        }
        ReviewAction::Undo => {
            let res = with_app(|app| app.anki.undo());
            if let Some(Ok(adv)) = res {
                apply_advance_result(adv);
            }
        }
        ReviewAction::OpenMenu => {
            let ctx = with_app_ref(|app| (app.window, app.config.language));
            if let Some((win, lang)) = ctx {
                let choice = unsafe {
                    run_choice_dialog(
                        win,
                        lang,
                        lang.menu_title(),
                        lang.menu_subtitle(),
                        &[
                            (lang.menu_bury(), RESP_BURY),
                            (lang.menu_suspend(), RESP_SUSPEND),
                            (lang.menu_refresh(), RESP_REFRESH),
                            (lang.menu_settings(), RESP_SETTINGS),
                            (lang.btn_cancel(), RESP_CANCEL),
                        ],
                    )
                };
                match choice {
                    RESP_BURY => {
                        if let Some(Ok(adv)) = with_app(|app| app.anki.bury_current()) {
                            apply_advance_result(adv);
                        }
                    }
                    RESP_SUSPEND => {
                        if let Some(Ok(adv)) = with_app(|app| app.anki.suspend_current()) {
                            apply_advance_result(adv);
                        }
                    }
                    RESP_REFRESH => schedule_clean_refresh(450),
                    RESP_SETTINGS => {
                        with_app(|app| unsafe { app.review_view.timer.hide() });
                        open_settings_page(PAGE_REVIEW, None);
                    }
                    _ => {}
                }
            }
        }
    }
}

fn open_settings_page(return_page: i32, custom_status: Option<&str>) {
    with_app(|app| {
        app.settings_return_page = return_page;
        unsafe {
            app.settings_view.sync_from_config(&app.config, custom_status);
            gtk_notebook_set_current_page(app.notebook, PAGE_SETTINGS);
            // Clear initial entry focus so Kindle virtual keyboard only opens when tapping an input box
            gtk_window_set_focus(app.window, ptr::null_mut());
        }
    });
}

fn start_collection_sync(mode: SyncMode) {
    let prep = with_app(|app| {
        let lang = app.config.language;
        if app.sync_rx.is_some() {
            return Err(None); // Already syncing
        }
        if app.config.hkey.trim().is_empty() {
            return Err(Some(lang.msg_login_required()));
        }
        let (col_path, media_dir, media_db) = app.config.get_resolved_collection_paths();
        if let Some(parent) = col_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::create_dir_all(&media_dir);
        app.anki.close_collection();

        let rx = spawn_collection_sync(
            col_path,
            media_dir,
            media_db,
            app.config.hkey.clone(),
            app.config.effective_sync_server(),
            mode,
            lang,
        );
        app.sync_rx = Some(rx);
        unsafe {
            g_timeout_add(120, on_sync_poll, ptr::null_mut());
        }
        Ok(lang)
    });

    match prep {
        Some(Ok(lang)) => {
            let banner = match mode {
                SyncMode::Normal => lang.banner_syncing_normal(),
                SyncMode::FullDownload => lang.banner_syncing_download(),
                SyncMode::FullUpload => lang.banner_syncing_upload(),
            };
            show_deck_banner(banner, None);
        }
        Some(Err(Some(msg))) => open_settings_page(PAGE_DECKS, Some(msg)),
        _ => {}
    }
}

fn estimate_wrapped_lines(text: &str, max_w: f64, font_px: f64) -> usize {
    let chars_per_line = (max_w / font_px).max(8.0);
    let mut lines: usize = 0;
    for para in text.split('\n') {
        let units: f64 = para
            .chars()
            .map(|c| if c.is_ascii() { 0.58 } else { 1.05 })
            .sum();
        let n = (units / chars_per_line).ceil() as usize;
        lines += n.max(1);
    }
    lines.max(1)
}

unsafe extern "C" fn on_choice_expose(
    _w: *mut GtkWidget,
    event: *mut GdkEventExpose,
    _data: gpointer,
) -> gboolean {
    let cr = gdk_cairo_create((*event).window);
    CHOICE_DIALOG.with(|cell| {
        if let Ok(mut guard) = cell.try_borrow_mut() {
            if let Some(st) = guard.as_mut() {
                let pad_x = 36.0;
                let pad_y = 30.0;
                let inner_w = st.width - pad_x * 2.0;

                // Pure white paper canvas + crisp 2px pure-black outer frame (zero GTK2 gray dither!)
                fill_white(cr);
                cairo_set_source_rgb(cr, 0.0, 0.0, 0.0);
                cairo_set_line_width(cr, 2.0);
                cairo_rectangle(cr, 1.0, 1.0, st.width - 2.0, st.height - 2.0);
                cairo_stroke(cr);

                let title_bold = !st.lang.is_cjk();
                let title_h = draw_text_wrapped(
                    cr, pad_x, pad_y, inner_w, &st.title, 32.0, title_bold, 0.0,
                ) as f64;

                let msg_y = pad_y + title_h + 10.0;
                let msg_h = if st.message.trim().is_empty() {
                    0.0
                } else {
                    draw_text_wrapped(cr, pad_x, msg_y, inner_w, &st.message, 25.0, false, 0.18)
                        as f64
                };

                let sep_y = msg_y + msg_h + 18.0;
                draw_hline(cr, pad_x, st.width - pad_x, sep_y, 1.5, 0.82);

                let btn_h = 78.0;
                let btn_gap = 14.0;
                let mut cur_y = sep_y + 20.0;
                st.button_rects.clear();
                for (i, (label, resp_id)) in st.buttons.iter().enumerate() {
                    let r = Rect::new(pad_x, cur_y, inner_w, btn_h);
                    st.button_rects.push(r);
                    let pressed = st.pressed_idx == Some(i);
                    let is_cancel = *resp_id == RESP_CANCEL;
                    // Use Regular weight for CJK to eliminate FreeType fake-bold blur; static Bold for EN primary!
                    let btn_bold = !is_cancel && !st.lang.is_cjk();
                    let border_gray = if is_cancel { 0.55 } else { 0.0 };
                    let text_gray = if is_cancel { 0.15 } else { 0.0 };
                    let font_px = if is_cancel { 28.0 } else { 30.0 };
                    draw_pill_button(
                        cr,
                        r,
                        label,
                        font_px,
                        btn_bold,
                        border_gray,
                        text_gray,
                        pressed,
                    );
                    cur_y += btn_h + btn_gap;
                }
            }
        }
    });
    cairo_destroy(cr);
    TRUE
}

unsafe extern "C" fn on_choice_press(
    _w: *mut GtkWidget,
    event: *mut GdkEventButton,
    _data: gpointer,
) -> gboolean {
    let x = (*event).x;
    let y = (*event).y;
    CHOICE_DIALOG.with(|cell| {
        if let Ok(mut guard) = cell.try_borrow_mut() {
            if let Some(st) = guard.as_mut() {
                st.pressed_idx = st.button_rects.iter().position(|r| r.contains(x, y));
                if st.pressed_idx.is_some() {
                    gtk_widget_queue_draw(st.area);
                }
            }
        }
    });
    TRUE
}

unsafe extern "C" fn on_choice_release(
    _w: *mut GtkWidget,
    event: *mut GdkEventButton,
    _data: gpointer,
) -> gboolean {
    let x = (*event).x;
    let y = (*event).y;
    let picked: Option<(*mut GtkWidget, i32)> = CHOICE_DIALOG.with(|cell| {
        if let Ok(mut guard) = cell.try_borrow_mut() {
            if let Some(st) = guard.as_mut() {
                let was = st.pressed_idx.take();
                gtk_widget_queue_draw(st.area);
                if let Some(idx) = was {
                    if st.button_rects.get(idx).map(|r| r.contains(x, y)).unwrap_or(false) {
                        if let Some((_, resp_id)) = st.buttons.get(idx) {
                            return Some((st.dialog, *resp_id));
                        }
                    }
                }
            }
        }
        None
    });
    if let Some((dlg, resp_id)) = picked {
        gtk_dialog_response(dlg, resp_id);
    }
    TRUE
}

/// Pure PangoCairo paper-book modal action sheet (zero GTK2 gray button theme dithering,
/// zero FreeType CJK fake-bold blur, dynamic wrapped message layout).
unsafe fn run_choice_dialog(
    parent: *mut GtkWidget,
    lang: UiLang,
    title: &str,
    message: &str,
    buttons: &[(&str, i32)],
) -> i32 {
    let dlg_w: f64 = 760.0;
    let inner_w: f64 = dlg_w - 72.0;
    let msg_lines = if message.trim().is_empty() {
        0
    } else {
        estimate_wrapped_lines(message, inner_w, 25.0)
    };
    let est_h = (30.0
        + 46.0
        + 10.0
        + (msg_lines as f64) * 38.0
        + 38.0
        + (buttons.len() as f64) * 92.0
        + 26.0)
        .min((SCREEN_H - 120) as f64);

    let dialog = gtk_dialog_new();
    let d_title = CString::new("L:D_N:dialog_ID:ranki.ranki.ranki_PC:N").unwrap();
    gtk_window_set_title(dialog, d_title.as_ptr());
    gtk_window_set_transient_for(dialog, parent);
    gtk_window_set_position(dialog, GTK_WIN_POS_CENTER_ON_PARENT);
    gtk_container_set_border_width(dialog, 0);
    gtk_dialog_set_has_separator(dialog, FALSE);

    let action_area = gtk_dialog_get_action_area(dialog);
    if !action_area.is_null() {
        gtk_widget_set_no_show_all(action_area, TRUE);
        gtk_widget_hide(action_area);
    }

    let white = GdkColor {
        pixel: 0,
        red: 0xFFFF,
        green: 0xFFFF,
        blue: 0xFFFF,
    };
    gtk_widget_modify_bg(dialog, GTK_STATE_NORMAL, &white);

    let draw_area = gtk_drawing_area_new();
    gtk_widget_set_size_request(draw_area, dlg_w as i32, est_h as i32);
    gtk_widget_add_events(draw_area, GDK_BUTTON_PRESS_MASK | GDK_BUTTON_RELEASE_MASK);

    let sig_exp = CString::new("expose-event").unwrap();
    g_signal_connect_data(
        draw_area as gpointer,
        sig_exp.as_ptr(),
        Some(std::mem::transmute(on_choice_expose as usize)),
        ptr::null_mut(),
        None,
        0,
    );
    let sig_p = CString::new("button-press-event").unwrap();
    g_signal_connect_data(
        draw_area as gpointer,
        sig_p.as_ptr(),
        Some(std::mem::transmute(on_choice_press as usize)),
        ptr::null_mut(),
        None,
        0,
    );
    let sig_r = CString::new("button-release-event").unwrap();
    g_signal_connect_data(
        draw_area as gpointer,
        sig_r.as_ptr(),
        Some(std::mem::transmute(on_choice_release as usize)),
        ptr::null_mut(),
        None,
        0,
    );

    let content = gtk_dialog_get_content_area(dialog);
    gtk_box_pack_start(content, draw_area, TRUE, TRUE, 0);

    CHOICE_DIALOG.with(|cell| {
        *cell.borrow_mut() = Some(ChoiceDialogState {
            dialog,
            area: draw_area,
            title: title.to_string(),
            message: message.to_string(),
            buttons: buttons.iter().map(|&(l, id)| (l.to_string(), id)).collect(),
            button_rects: Vec::with_capacity(buttons.len()),
            pressed_idx: None,
            lang,
            width: dlg_w,
            height: est_h,
        });
    });

    gtk_widget_show_all(dialog);
    let response = gtk_dialog_run(dialog);
    CHOICE_DIALOG.with(|cell| {
        *cell.borrow_mut() = None;
    });
    gtk_widget_destroy(dialog);

    // Wipe modal dialog frame ghosting cleanly after Xorg finishes repainting underlying view
    if response != RESP_REFRESH {
        schedule_clean_refresh(450);
    }
    response
}

// ============================================================================
// GTK2 / GLib / WebKitGTK C Callbacks
// ============================================================================

unsafe extern "C" fn on_window_destroy(_w: *mut GtkWidget, _data: gpointer) {
    eink::close_keyboard();
    with_app(|app| {
        app.review_view.timer.hide();
        app.anki.close_collection();
    });
    gtk_main_quit();
}

unsafe extern "C" fn on_clear_deck_banner(_data: gpointer) -> gboolean {
    with_app(|app| {
        app.banner_clear_timer = None;
        app.deck_view.status_banner = None;
        gtk_widget_queue_draw(app.deck_view.list_area);
    });
    FALSE
}

unsafe extern "C" fn on_focus_timer_tick(_data: gpointer) -> gboolean {
    let keep = with_app(|app| app.review_view.timer.on_tick()).unwrap_or(false);
    if keep { TRUE } else { FALSE }
}

unsafe extern "C" fn on_timer_expose(
    _w: *mut GtkWidget,
    event: *mut GdkEventExpose,
    _data: gpointer,
) -> gboolean {
    if event.is_null() {
        return FALSE;
    }
    let cr = gdk_cairo_create((*event).window);
    with_app_ref(|app| app.review_view.timer.draw(cr));
    cairo_destroy(cr);
    TRUE
}

// --- Deck View Callbacks ---

unsafe extern "C" fn on_deck_top_expose(
    _w: *mut GtkWidget,
    event: *mut GdkEventExpose,
    _data: gpointer,
) -> gboolean {
    let cr = gdk_cairo_create((*event).window);
    with_app_ref(|app| app.deck_view.draw_top(cr, app.config.language));
    cairo_destroy(cr);
    TRUE
}

unsafe extern "C" fn on_deck_top_press(
    _w: *mut GtkWidget,
    event: *mut GdkEventButton,
    _data: gpointer,
) -> gboolean {
    with_app(|app| app.deck_view.on_top_press((*event).x, (*event).y));
    TRUE
}

unsafe extern "C" fn on_deck_top_release(
    _w: *mut GtkWidget,
    event: *mut GdkEventButton,
    _data: gpointer,
) -> gboolean {
    let act = with_app(|app| app.deck_view.on_top_release((*event).x, (*event).y))
        .unwrap_or(DeckAction::None);
    handle_deck_action(act);
    TRUE
}

unsafe extern "C" fn on_deck_list_expose(
    _w: *mut GtkWidget,
    event: *mut GdkEventExpose,
    _data: gpointer,
) -> gboolean {
    let cr = gdk_cairo_create((*event).window);
    with_app_ref(|app| app.deck_view.draw_list(cr, app.config.language));
    cairo_destroy(cr);
    TRUE
}

unsafe extern "C" fn on_deck_list_press(
    _w: *mut GtkWidget,
    event: *mut GdkEventButton,
    _data: gpointer,
) -> gboolean {
    with_app(|app| app.deck_view.on_list_press((*event).x, (*event).y));
    TRUE
}

unsafe extern "C" fn on_deck_list_release(
    _w: *mut GtkWidget,
    event: *mut GdkEventButton,
    _data: gpointer,
) -> gboolean {
    let act = with_app(|app| app.deck_view.on_list_release((*event).x, (*event).y))
        .unwrap_or(DeckAction::None);
    handle_deck_action(act);
    TRUE
}

unsafe extern "C" fn on_deck_footer_expose(
    _w: *mut GtkWidget,
    event: *mut GdkEventExpose,
    _data: gpointer,
) -> gboolean {
    let cr = gdk_cairo_create((*event).window);
    with_app_ref(|app| app.deck_view.draw_footer(cr, app.config.language));
    cairo_destroy(cr);
    TRUE
}

unsafe extern "C" fn on_deck_footer_press(
    _w: *mut GtkWidget,
    event: *mut GdkEventButton,
    _data: gpointer,
) -> gboolean {
    with_app(|app| app.deck_view.on_footer_press((*event).x, (*event).y));
    TRUE
}

unsafe extern "C" fn on_deck_footer_release(
    _w: *mut GtkWidget,
    event: *mut GdkEventButton,
    _data: gpointer,
) -> gboolean {
    let act = with_app(|app| app.deck_view.on_footer_release((*event).x, (*event).y))
        .unwrap_or(DeckAction::None);
    handle_deck_action(act);
    TRUE
}

// --- Review View & Resident Card WebView Callbacks ---

unsafe extern "C" fn on_review_top_expose(
    _w: *mut GtkWidget,
    event: *mut GdkEventExpose,
    _data: gpointer,
) -> gboolean {
    let cr = gdk_cairo_create((*event).window);
    with_app_ref(|app| app.review_view.draw_top(cr, app.config.language));
    cairo_destroy(cr);
    TRUE
}

unsafe extern "C" fn on_review_top_press(
    _w: *mut GtkWidget,
    event: *mut GdkEventButton,
    _data: gpointer,
) -> gboolean {
    with_app(|app| app.review_view.on_top_press((*event).x, (*event).y));
    TRUE
}

unsafe extern "C" fn on_review_top_release(
    _w: *mut GtkWidget,
    event: *mut GdkEventButton,
    _data: gpointer,
) -> gboolean {
    let act = with_app(|app| app.review_view.on_top_release((*event).x, (*event).y))
        .unwrap_or(ReviewAction::None);
    handle_review_action(act);
    TRUE
}

unsafe extern "C" fn on_review_bottom_expose(
    _w: *mut GtkWidget,
    event: *mut GdkEventExpose,
    _data: gpointer,
) -> gboolean {
    let cr = gdk_cairo_create((*event).window);
    with_app_ref(|app| app.review_view.draw_bottom(cr, app.config.language));
    cairo_destroy(cr);
    TRUE
}

unsafe extern "C" fn on_review_bottom_press(
    _w: *mut GtkWidget,
    event: *mut GdkEventButton,
    _data: gpointer,
) -> gboolean {
    with_app(|app| app.review_view.on_bottom_press((*event).x, (*event).y));
    TRUE
}

unsafe extern "C" fn on_review_bottom_release(
    _w: *mut GtkWidget,
    event: *mut GdkEventButton,
    _data: gpointer,
) -> gboolean {
    let act = with_app(|app| app.review_view.on_bottom_release((*event).x, (*event).y))
        .unwrap_or(ReviewAction::None);
    handle_review_action(act);
    TRUE
}

// GDK-level zero-bar swipe & tap-zone interception on WebKitWebView (0 drag frames!)
unsafe extern "C" fn on_card_press(
    _w: *mut GtkWidget,
    event: *mut GdkEventButton,
    _data: gpointer,
) -> gboolean {
    with_app(|app| app.review_view.on_card_press((*event).x, (*event).y));
    TRUE // Consume so WebKit never starts accidental text selection or inertial drag!
}

unsafe extern "C" fn on_card_motion(
    _w: *mut GtkWidget,
    _event: *mut GdkEventMotion,
    _data: gpointer,
) -> gboolean {
    TRUE // 0 intermediate drag repaints -> 0 e-ink ghosting!
}

unsafe extern "C" fn on_card_release(
    _w: *mut GtkWidget,
    event: *mut GdkEventButton,
    _data: gpointer,
) -> gboolean {
    let gesture = with_app(|app| app.review_view.on_card_release((*event).x, (*event).y))
        .unwrap_or(CardGesture::None);
    match gesture {
        CardGesture::PageNext => {
            run_card_js("if(window.CardEngine){window.CardEngine.pageScroll(1);}");
        }
        CardGesture::PagePrev => {
            run_card_js("if(window.CardEngine){window.CardEngine.pageScroll(-1);}");
        }
        CardGesture::None => {}
    }
    TRUE
}

unsafe extern "C" fn on_card_title_notify(
    web_view: *mut GtkWidget,
    _pspec: gpointer,
    _data: gpointer,
) {
    let ptr = webkit_web_view_get_title(web_view);
    if ptr.is_null() {
        return;
    }
    let title = match CStr::from_ptr(ptr).to_str() {
        Ok(s) => s,
        Err(_) => return,
    };
    if !title.starts_with("RA:") {
        return;
    }
    let parts: Vec<&str> = title.splitn(4, ':').collect();
    if parts.len() < 4 {
        return;
    }
    let action = parts[1];
    let token = match parts[2].parse::<u64>() {
        Ok(t) => t,
        Err(_) => return,
    };
    if LAST_CARD_TOKEN.swap(token, Ordering::SeqCst) == token {
        return;
    }
    let payload_str = parts[3];

    match action {
        "CARD_READY" => {
            println!("[CARD] Resident card canvas ready: {}", payload_str);
            let pending = with_app(|app| {
                app.review_view.card_engine_ready = true;
                app.pending_card_script.take()
            })
            .flatten();
            push_card_config_to_webview();
            if let Some(script) = pending {
                exec_card_script(web_view, &script);
            }
        }
        "PAGE_STATE" | "RENDER_DONE" | "FLIP_DONE" => {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(payload_str) {
                if action != "PAGE_STATE" {
                    println!(
                        "[PERF] {} cid={} mode={} ms={}",
                        action,
                        v["card_id"].as_i64().unwrap_or(0),
                        v["mode"].as_str().unwrap_or("-"),
                        v["ms"].as_i64().unwrap_or(-1)
                    );
                }
                let p = v["page"].as_u64().unwrap_or(1) as u32;
                let t = v["total"].as_u64().unwrap_or(1) as u32;
                let trigger_clean = with_app(|app| {
                    if app.review_view.page_cur != p || app.review_view.page_total != t {
                        app.review_view.page_cur = p;
                        app.review_view.page_total = t;
                        gtk_widget_queue_draw(app.review_view.top_area);
                    }
                    if action == "RENDER_DONE" && app.needs_clean_refresh {
                        app.needs_clean_refresh = false;
                        true
                    } else {
                        false
                    }
                })
                .unwrap_or(false);

                if trigger_clean {
                    schedule_clean_refresh(450);
                }
            }
        }
        "PREFETCH_MISS" => {
            println!(
                "[CARD] Prefetch miss reported ({}); scheduling immediate re-render.",
                payload_str
            );
            with_app(|app| {
                app.review_view.prefetched_card_id = None;
            });
            g_idle_add(on_prefetch_miss_idle, ptr::null_mut());
        }
        _ => {}
    }
}

unsafe extern "C" fn on_prefetch_miss_idle(_data: gpointer) -> gboolean {
    let res = with_app(|app| {
        if app.anki.is_open() {
            Some(app.anki.refresh_current_immediate())
        } else {
            None
        }
    })
    .flatten();
    if let Some(Ok(adv)) = res {
        apply_advance_result(adv);
    }
    FALSE
}

// --- Settings & Background Async Sync/Login Callbacks ---

unsafe extern "C" fn on_settings_top_expose(
    _w: *mut GtkWidget,
    event: *mut GdkEventExpose,
    _data: gpointer,
) -> gboolean {
    let cr = gdk_cairo_create((*event).window);
    with_app_ref(|app| app.settings_view.draw_top(cr));
    cairo_destroy(cr);
    TRUE
}

unsafe extern "C" fn on_settings_top_press(
    _w: *mut GtkWidget,
    event: *mut GdkEventButton,
    _data: gpointer,
) -> gboolean {
    with_app(|app| app.settings_view.on_top_press((*event).x, (*event).y));
    TRUE
}

unsafe extern "C" fn on_settings_top_release(
    _w: *mut GtkWidget,
    event: *mut GdkEventButton,
    _data: gpointer,
) -> gboolean {
    let back = with_app(|app| app.settings_view.on_top_release((*event).x, (*event).y))
        .unwrap_or(false);
    if back {
        eink::close_keyboard();
        with_app(|app| {
            gtk_window_set_focus(app.window, ptr::null_mut());
            let ret = app.settings_return_page;
            if ret == PAGE_DECKS {
                app.deck_view.queue_redraw_all();
            } else if ret == PAGE_REVIEW && app.review_view.stage == ReviewStage::Unflipped {
                app.review_view.timer.start_front(on_focus_timer_tick);
            }
            gtk_notebook_set_current_page(app.notebook, ret);
        });
        schedule_clean_refresh(450);
    }
    TRUE
}

unsafe extern "C" fn on_entry_focus_in(
    _w: *mut GtkWidget,
    _ev: gpointer,
    _data: gpointer,
) -> gboolean {
    let on_settings = with_app_ref(|app| gtk_notebook_get_current_page(app.notebook) == PAGE_SETTINGS)
        .unwrap_or(false);
    if on_settings {
        eink::open_keyboard();
    }
    FALSE
}

unsafe extern "C" fn on_entry_focus_out(
    _w: *mut GtkWidget,
    _ev: gpointer,
    _data: gpointer,
) -> gboolean {
    eink::close_keyboard();
    FALSE
}

unsafe extern "C" fn on_settings_ctrl_expose(
    _w: *mut GtkWidget,
    event: *mut GdkEventExpose,
    _data: gpointer,
) -> gboolean {
    let cr = gdk_cairo_create((*event).window);
    with_app_ref(|app| app.settings_view.draw_controls(cr));
    cairo_destroy(cr);
    TRUE
}

unsafe extern "C" fn on_settings_ctrl_press(
    _w: *mut GtkWidget,
    event: *mut GdkEventButton,
    _data: gpointer,
) -> gboolean {
    with_app(|app| app.settings_view.on_ctrl_press((*event).x, (*event).y));
    TRUE
}

unsafe extern "C" fn on_settings_ctrl_release(
    _w: *mut GtkWidget,
    event: *mut GdkEventButton,
    _data: gpointer,
) -> gboolean {
    let act = with_app(|app| app.settings_view.on_ctrl_release((*event).x, (*event).y))
        .unwrap_or(SettingsAction::None);
    handle_settings_action(act);
    TRUE
}

unsafe fn handle_settings_action(action: SettingsAction) {
    match action {
        SettingsAction::None => {}
        SettingsAction::Login => {
            eink::close_keyboard();
            with_app(|app| {
                gtk_window_set_focus(app.window, ptr::null_mut());
                if app.login_rx.is_some() {
                    return;
                }
                let lang = app.config.language;
                let (user, pass, srv) = app.settings_view.read_credentials();
                app.settings_view
                    .set_status_for_config(&app.config, Some(lang.msg_signing_in()));
                app.login_rx = Some(spawn_login(user, pass, srv, lang));
                g_timeout_add(120, on_login_poll, ptr::null_mut());
            });
        }
        SettingsAction::SaveServer => {
            eink::close_keyboard();
            with_app(|app| {
                gtk_window_set_focus(app.window, ptr::null_mut());
                let (_, _, srv) = app.settings_view.read_credentials();
                app.config.set_custom_sync_server(&srv);
                let _ = app.config.save();
                let msg = app.config.language.msg_server_saved();
                app.settings_view.sync_from_config(&app.config, Some(msg));
            });
        }
        SettingsAction::Logout => {
            eink::close_keyboard();
            with_app(|app| {
                gtk_window_set_focus(app.window, ptr::null_mut());
                app.config.clear_sync_auth();
                let _ = app.config.save();
                app.settings_view.clear_password_entry();
                let msg = app.config.language.msg_signed_out();
                app.settings_view.set_status_for_config(&app.config, Some(msg));
            });
        }
        SettingsAction::FontDown => {
            with_app(|app| {
                let next = app.config.font_size_px.saturating_sub(2).max(MIN_FONT_SIZE_PX);
                app.config.set_font_size_px(next);
                let _ = app.config.save();
                app.settings_view.sync_reading_controls(&app.config);
            });
            push_card_config_to_webview();
        }
        SettingsAction::FontUp => {
            with_app(|app| {
                let next = (app.config.font_size_px + 2).min(MAX_FONT_SIZE_PX);
                app.config.set_font_size_px(next);
                let _ = app.config.save();
                app.settings_view.sync_reading_controls(&app.config);
            });
            push_card_config_to_webview();
        }
        SettingsAction::SetLang(new_lang) => {
            eink::close_keyboard();
            let changed = with_app(|app| {
                gtk_window_set_focus(app.window, ptr::null_mut());
                if app.config.language == new_lang {
                    return false;
                }
                app.config.language = new_lang;
                let _ = app.config.save();
                app.settings_view.sync_from_config(&app.config, None);
                app.deck_view.queue_redraw_all();
                app.review_view.queue_redraw_bars();
                println!("[CONFIG] Switched UI language -> {}", new_lang.as_code());
                true
            })
            .unwrap_or(false);
            if changed {
                push_card_config_to_webview();
            }
        }
    }
}

unsafe extern "C" fn on_login_poll(_data: gpointer) -> gboolean {
    let outcome = with_app(|app| match app.login_rx.as_ref() {
        Some(rx) => match rx.try_recv() {
            Ok(res) => {
                app.login_rx = None;
                Some(res)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                app.login_rx = None;
                Some(LoginOutcome::Error(
                    app.config.language.err_login_thread().to_string(),
                ))
            }
        },
        None => Some(LoginOutcome::Error(String::new())),
    });

    match outcome {
        None | Some(None) => TRUE, // Keep polling
        Some(Some(LoginOutcome::Success { hkey, sync_server })) => {
            with_app(|app| {
                app.config.hkey = hkey;
                app.config.set_custom_sync_server(&sync_server);
                app.config.sync_endpoint_cache.clear();
                let _ = app.config.save();
                app.settings_view.clear_password_entry();
                let msg = app.config.language.msg_login_success();
                app.settings_view.sync_from_config(&app.config, Some(msg));
                gtk_notebook_set_current_page(app.notebook, PAGE_DECKS);
            });
            start_collection_sync(SyncMode::Normal);
            FALSE
        }
        Some(Some(LoginOutcome::Error(msg))) => {
            if !msg.is_empty() {
                with_app(|app| {
                    app.settings_view.set_status_for_config(&app.config, Some(&msg));
                });
            }
            FALSE
        }
    }
}

unsafe extern "C" fn on_sync_poll(_data: gpointer) -> gboolean {
    let outcome = with_app(|app| match app.sync_rx.as_ref() {
        Some(rx) => match rx.try_recv() {
            Ok(res) => {
                app.sync_rx = None;
                Some(res)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                app.sync_rx = None;
                Some(SyncOutcome::Error(
                    app.config.language.err_sync_thread().to_string(),
                ))
            }
        },
        None => Some(SyncOutcome::Error(String::new())),
    });

    match outcome {
        None | Some(None) => TRUE, // Still running in background worker
        Some(Some(res)) => {
            // Reopen main-thread collection and refresh deck browser
            with_app(|app| {
                let (col_path, media_dir, media_db) = app.config.get_resolved_collection_paths();
                if col_path.exists() {
                    if let Err(e) = app.anki.open_collection(&col_path, &media_dir, &media_db) {
                        eprintln!("[SYNC] Failed to reopen collection after sync: {}", e);
                    } else if let Ok(data) = app.anki.get_deck_list_data() {
                        app.deck_view.state.set_data(
                            data.decks,
                            data.total_due,
                            data.studied_today,
                        );
                    }
                }
                app.deck_view.queue_redraw_all();
            });

            match res {
                SyncOutcome::Success {
                    message,
                    new_endpoint,
                } => {
                    persist_redirected_endpoint(new_endpoint);
                    show_deck_banner(&message, Some(4000));
                }
                SyncOutcome::Error(err) => {
                    if !err.is_empty() {
                        eprintln!("[SYNC] UI banner error: {}", err);
                        show_deck_banner(&err, Some(6000));
                    }
                }
                SyncOutcome::FullSyncRequired {
                    upload_ok,
                    download_ok,
                    server_message,
                    new_endpoint,
                } => {
                    persist_redirected_endpoint(new_endpoint);
                    let ctx = with_app_ref(|app| (app.window, app.config.language));
                    if let Some((win, lang)) = ctx {
                        show_deck_banner(lang.banner_full_sync_needed(), Some(3000));
                        let body = lang.dialog_full_sync_body(&server_message);
                        let mut choices: Vec<(&str, i32)> = Vec::new();
                        if download_ok {
                            choices.push((lang.btn_full_download(), RESP_FULL_DOWNLOAD));
                        }
                        if upload_ok {
                            choices.push((lang.btn_full_upload(), RESP_FULL_UPLOAD));
                        }
                        choices.push((lang.btn_cancel(), RESP_CANCEL));

                        let picked = run_choice_dialog(
                            win,
                            lang,
                            lang.dialog_full_sync_title(),
                            &body,
                            &choices,
                        );
                        match picked {
                            RESP_FULL_DOWNLOAD => start_collection_sync(SyncMode::FullDownload),
                            RESP_FULL_UPLOAD => start_collection_sync(SyncMode::FullUpload),
                            _ => {}
                        }
                    }
                }
            }
            FALSE
        }
    }
}

fn persist_redirected_endpoint(new_endpoint: Option<String>) {
    if let Some(ep) = new_endpoint {
        let trimmed = ep.trim().to_string();
        if !trimmed.is_empty() {
            with_app(|app| {
                if app.config.sync_endpoint_cache != trimmed {
                    println!("[SYNC] Persisting redirected sync endpoint: {}", trimmed);
                    app.config.sync_endpoint_cache = trimmed;
                    let _ = app.config.save();
                }
            });
        }
    }
}
