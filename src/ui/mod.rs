//! Route B Native GTK2/PangoCairo Application Controller
//! Coordinates DeckView (Page 0), ReviewView (Page 1), SettingsView (Page 2),
//! AnkiService (direct rslib), background async sync/login, and Kindle e-ink hardware.
pub mod deck_view;
pub mod eink;
pub mod focus_timer;
pub mod review_view;
pub mod settings_view;
pub mod theme;

use std::cell::{Cell, RefCell};
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
use crate::logic::CardGesture;
use crate::ui::deck_view::{DeckAction, DeckView};
use crate::ui::review_view::{exec_card_script, ReviewAction, ReviewStage, ReviewView};
use crate::ui::settings_view::SettingsView;
use crate::ui::theme::{make_font_desc, register_app_fonts, SCREEN_H, SCREEN_W};

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

thread_local! {
    static APP: RefCell<Option<AppController>> = const { RefCell::new(None) };
    static ACTIVE_CHOICE_DIALOG: Cell<*mut GtkWidget> = const { Cell::new(ptr::null_mut()) };
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

        let settings_view = SettingsView::new(
            on_settings_top_expose,
            on_settings_top_press,
            on_settings_top_release,
            on_entry_focus_in,
            on_entry_focus_out,
            on_settings_login_click,
            on_settings_save_server_click,
            on_settings_logout_click,
            on_settings_font_down_click,
            on_settings_font_up_click,
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
            "font_size_px": app.config.font_size_px
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

                let msg_json = serde_json::to_string(&studied_today).unwrap_or_else(|_| "\"今日复习已完成\"".to_string());
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
            let result = with_app(|app| {
                if !app.anki.is_open() {
                    return Err("本地数据库未打开，请先同步或检查测试库。".to_string());
                }
                app.needs_clean_refresh = true;
                app.anki.enter_deck(did)
            });

            match result {
                Some(Ok(AdvanceResult::DeckFinished { .. })) => {
                    show_deck_banner("该牌组今日待复习卡片已全部完成", Some(3000));
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
            let parent = with_app_ref(|app| app.window);
            if let Some(win) = parent {
                let choice = unsafe {
                    run_choice_dialog(
                        win,
                        "当前卡片与操作",
                        "请选择要执行的操作：",
                        &[
                            ("搁置此卡 (今日跳过)", RESP_BURY),
                            ("暂停此卡 (永久移出复习)", RESP_SUSPEND),
                            ("全屏清残影 (GC16 刷新)", RESP_REFRESH),
                            ("字号与账号设置", RESP_SETTINGS),
                            ("取消", RESP_CANCEL),
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
            // Focus a non-entry button initially so Kindle keyboard only opens when user taps an entry
            gtk_widget_grab_focus(app.settings_view.btn_font_down);
        }
    });
}

fn start_collection_sync(mode: SyncMode) {
    let prep = with_app(|app| {
        if app.sync_rx.is_some() {
            return Err(None); // Already syncing
        }
        if app.config.hkey.trim().is_empty() {
            return Err(Some("尚未登录 AnkiWeb，请先输入账号密码登录"));
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
            app.config.sync_server.clone(),
            mode,
        );
        app.sync_rx = Some(rx);
        unsafe {
            g_timeout_add(120, on_sync_poll, ptr::null_mut());
        }
        Ok(())
    });

    match prep {
        Some(Ok(())) => {
            let banner = match mode {
                SyncMode::Normal => "正在与云端同步集合（不下载媒体）...",
                SyncMode::FullDownload => "正在从云端完整下载最新集合...",
                SyncMode::FullUpload => "正在上传本地集合至云端...",
            };
            show_deck_banner(banner, None);
        }
        Some(Err(Some(msg))) => open_settings_page(PAGE_DECKS, Some(msg)),
        _ => {}
    }
}

unsafe extern "C" fn on_choice_btn_clicked(_btn: *mut GtkWidget, data: gpointer) {
    let resp_id = data as isize as i32;
    let dlg = ACTIVE_CHOICE_DIALOG.with(|c| c.get());
    if !dlg.is_null() {
        gtk_dialog_response(dlg, resp_id);
    }
}

/// Vertical full-width e-ink action sheet dialog (avoids GTK2's horizontal GtkHButtonBox overflow!).
unsafe fn run_choice_dialog(
    parent: *mut GtkWidget,
    title: &str,
    message: &str,
    buttons: &[(&str, i32)],
) -> i32 {
    let dialog = gtk_dialog_new();
    let d_title = CString::new("L:D_N:dialog_ID:ranki.ranki.ranki_PC:N").unwrap();
    gtk_window_set_title(dialog, d_title.as_ptr());
    gtk_window_set_transient_for(dialog, parent);
    gtk_window_set_position(dialog, GTK_WIN_POS_CENTER_ON_PARENT);
    gtk_widget_set_size_request(dialog, 720, -1);

    let white = GdkColor {
        pixel: 0,
        red: 0xFFFF,
        green: 0xFFFF,
        blue: 0xFFFF,
    };
    gtk_widget_modify_bg(dialog, GTK_STATE_NORMAL, &white);

    let area = gtk_dialog_get_content_area(dialog);
    let vbox = gtk_vbox_new(FALSE, 14);
    gtk_container_set_border_width(vbox, 32);

    let f_title = make_font_desc(30.0, true);
    let f_msg = make_font_desc(24.0, false);
    let f_btn = make_font_desc(26.0, true);

    if let Ok(ct) = CString::new(title) {
        let lt = gtk_label_new(ct.as_ptr());
        gtk_misc_set_alignment(lt, 0.0, 0.5);
        gtk_widget_modify_font(lt, f_title as *mut std::ffi::c_void);
        gtk_box_pack_start(vbox, lt, FALSE, FALSE, 0);
    }
    if let Ok(cm) = CString::new(message) {
        let lm = gtk_label_new(cm.as_ptr());
        gtk_misc_set_alignment(lm, 0.0, 0.5);
        gtk_label_set_line_wrap(lm, TRUE);
        gtk_widget_modify_font(lm, f_msg as *mut std::ffi::c_void);
        gtk_box_pack_start(vbox, lm, FALSE, FALSE, 6);
    }

    let sig_clicked = CString::new("clicked").unwrap();
    for &(label, resp_id) in buttons {
        if let Ok(cl) = CString::new(label) {
            let btn = gtk_button_new_with_label(cl.as_ptr());
            gtk_widget_set_size_request(btn, 640, 68);
            gtk_widget_modify_font(btn, f_btn as *mut std::ffi::c_void);
            let child = gtk_bin_get_child(btn);
            if !child.is_null() {
                gtk_widget_modify_font(child, f_btn as *mut std::ffi::c_void);
            }
            g_signal_connect_data(
                btn as gpointer,
                sig_clicked.as_ptr(),
                Some(std::mem::transmute(on_choice_btn_clicked as usize)),
                resp_id as isize as gpointer,
                None,
                0,
            );
            gtk_box_pack_start(vbox, btn, FALSE, FALSE, 0);
        }
    }

    gtk_box_pack_start(area, vbox, TRUE, TRUE, 0);

    pango_font_description_free(f_title);
    pango_font_description_free(f_msg);
    pango_font_description_free(f_btn);

    ACTIVE_CHOICE_DIALOG.with(|c| c.set(dialog));
    gtk_widget_show_all(dialog);
    let response = gtk_dialog_run(dialog);
    ACTIVE_CHOICE_DIALOG.with(|c| c.set(ptr::null_mut()));
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
    with_app_ref(|app| app.deck_view.draw_top(cr));
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
    with_app_ref(|app| app.deck_view.draw_list(cr));
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
    with_app_ref(|app| app.deck_view.draw_footer(cr));
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
    with_app_ref(|app| app.review_view.draw_top(cr));
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
    with_app_ref(|app| app.review_view.draw_bottom(cr));
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

unsafe extern "C" fn on_settings_login_click(_w: *mut GtkWidget, _data: gpointer) {
    eink::close_keyboard();
    with_app(|app| {
        if app.login_rx.is_some() {
            return;
        }
        let (user, pass, srv) = app.settings_view.read_credentials();
        app.settings_view
            .set_status_for_config(&app.config, Some("正在连接服务器验证账号，请稍候..."));
        app.login_rx = Some(spawn_login(user, pass, srv));
        g_timeout_add(120, on_login_poll, ptr::null_mut());
    });
}

unsafe extern "C" fn on_settings_save_server_click(_w: *mut GtkWidget, _data: gpointer) {
    eink::close_keyboard();
    with_app(|app| {
        let (_, _, srv) = app.settings_view.read_credentials();
        app.config.sync_server = srv.trim().to_string();
        let _ = app.config.save();
        app.settings_view
            .sync_from_config(&app.config, Some("同步服务器地址已保存。"));
    });
}

unsafe extern "C" fn on_settings_logout_click(_w: *mut GtkWidget, _data: gpointer) {
    eink::close_keyboard();
    with_app(|app| {
        app.config.hkey.clear();
        let _ = app.config.save();
        app.settings_view.clear_password_entry();
        app.settings_view
            .set_status_for_config(&app.config, Some("已清除本地登录 Token。"));
    });
}

unsafe extern "C" fn on_settings_font_down_click(_w: *mut GtkWidget, _data: gpointer) {
    with_app(|app| {
        let next = app.config.font_size_px.saturating_sub(2).max(MIN_FONT_SIZE_PX);
        app.config.set_font_size_px(next);
        let _ = app.config.save();
        app.settings_view.sync_reading_controls(&app.config);
    });
    push_card_config_to_webview();
}

unsafe extern "C" fn on_settings_font_up_click(_w: *mut GtkWidget, _data: gpointer) {
    with_app(|app| {
        let next = (app.config.font_size_px + 2).min(MAX_FONT_SIZE_PX);
        app.config.set_font_size_px(next);
        let _ = app.config.save();
        app.settings_view.sync_reading_controls(&app.config);
    });
    push_card_config_to_webview();
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
                Some(LoginOutcome::Error("登录后台线程异常中断".to_string()))
            }
        },
        None => Some(LoginOutcome::Error(String::new())),
    });

    match outcome {
        None | Some(None) => TRUE, // Keep polling
        Some(Some(LoginOutcome::Success { hkey, sync_server })) => {
            with_app(|app| {
                app.config.hkey = hkey;
                app.config.sync_server = sync_server;
                let _ = app.config.save();
                app.settings_view.clear_password_entry();
                app.settings_view.sync_from_config(
                    &app.config,
                    Some("登录成功！安全 Token 已保存，即将自动同步集合..."),
                );
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
                Some(SyncOutcome::Error("同步后台线程异常中断".to_string()))
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
                SyncOutcome::Success(msg) => {
                    show_deck_banner(&msg, Some(4000));
                }
                SyncOutcome::Error(err) => {
                    if !err.is_empty() {
                        show_deck_banner(&err, Some(5000));
                    }
                }
                SyncOutcome::FullSyncRequired {
                    upload_ok,
                    download_ok,
                    server_message,
                } => {
                    show_deck_banner("需要全量同步，请选择方向", Some(3000));
                    let parent = with_app_ref(|app| app.window);
                    if let Some(win) = parent {
                        let body = if server_message.trim().is_empty() {
                            "本地与云端集合存在结构差异，必须执行单向全量同步（仅同步数据库，不下载媒体）：".to_string()
                        } else {
                            format!("{}\n\n请选择全量同步方向：", server_message)
                        };
                        let mut choices: Vec<(&str, i32)> = Vec::new();
                        if download_ok {
                            choices.push(("从云端下载覆盖本地", RESP_FULL_DOWNLOAD));
                        }
                        if upload_ok {
                            choices.push(("上传本地覆盖云端", RESP_FULL_UPLOAD));
                        }
                        choices.push(("取消", RESP_CANCEL));

                        let picked = run_choice_dialog(win, "需要全量同步", &body, &choices);
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
