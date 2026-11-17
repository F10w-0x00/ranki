//! Bidirectional IPC protocol between Rust Host and WebKit Single Page App
use std::ffi::{CStr, CString};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex};
use crate::anki::AnkiService;
use crate::config::AppConfig;
use crate::ffi::*;

pub static ANKI_SERVICE: LazyLock<Mutex<AnkiService>> = LazyLock::new(|| Mutex::new(AnkiService::new()));
pub static APP_CONFIG: LazyLock<Mutex<AppConfig>> = LazyLock::new(|| Mutex::new(AppConfig::load()));
static LAST_TOKEN: AtomicU64 = AtomicU64::new(0);

/// Parse and handle document.title changes from WebKit
/// Format: "RA:<action>:<token>:<payload>"
pub fn handle_title_change(web_view: *mut GtkWidget, title: &str) {
    if !title.starts_with("RA:") {
        return;
    }

    let parts: Vec<&str> = title.splitn(4, ':').collect();
    if parts.len() < 4 {
        eprintln!("[IPC] Invalid message format: {}", title);
        return;
    }

    let action = parts[1];
    let token_str = parts[2];
    let payload = parts[3];

    let token = match token_str.parse::<u64>() {
        Ok(t) => t,
        Err(_) => {
            eprintln!("[IPC] Invalid token integer: {}", token_str);
            return;
        }
    };

    let prev = LAST_TOKEN.swap(token, Ordering::SeqCst);
    if prev == token {
        // Prevent duplicate events
        return;
    }

    println!("[IPC] Action: '{}' [token={}], payload len: {}", action, token, payload.len());

    match action {
        "ready" => {
            println!("[IPC] WebKit SPA ready. Loading deck tree & config...");
            let mut svc = ANKI_SERVICE.lock().unwrap();
            let config = APP_CONFIG.lock().unwrap();
            let has_hkey = !config.hkey.trim().is_empty();
            let sync_server = config.sync_server.clone();

            if svc.is_open() {
                let decks = svc.get_deck_list().unwrap_or_default();
                let studied_today = svc.studied_today();
                if let Ok(json) = serde_json::to_string(&serde_json::json!({
                    "decks": decks,
                    "studied_today": studied_today,
                    "has_hkey": has_hkey,
                    "sync_server": sync_server
                })) {
                    dispatch_to_webview(web_view, "LOAD_DECK_LIST", &json);
                    return;
                }
            }
            dispatch_to_webview(web_view, "HOST_ACK", "{\"status\":\"ready\"}");
        }
        "GET_CONFIG" => {
            let config = APP_CONFIG.lock().unwrap();
            let has_hkey = !config.hkey.trim().is_empty();
            let sync_server = config.sync_server.clone();
            let collection_dir = config.collection_dir.clone();
            let scale = config.scale;
            if let Ok(json) = serde_json::to_string(&serde_json::json!({
                "has_hkey": has_hkey,
                "sync_server": sync_server,
                "collection_dir": collection_dir,
                "scale": scale
            })) {
                dispatch_to_webview(web_view, "CONFIG_DATA", &json);
            }
        }
        "SAVE_SETTINGS" => {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(payload) {
                let mut config = APP_CONFIG.lock().unwrap();
                if let Some(s) = v["sync_server"].as_str() {
                    config.sync_server = s.trim().to_string();
                }
                if let Some(sc) = v["scale"].as_f64() {
                    config.scale = sc;
                }
                let _ = config.save();
                dispatch_to_webview(web_view, "SETTINGS_SAVED", "{\"success\":true}");
            }
        }
        "SYNC_LOGIN" => {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(payload) {
                let username = v["username"].as_str().unwrap_or_default().trim();
                let password = v["password"].as_str().unwrap_or_default().trim();
                let custom_server = v["sync_server"].as_str().map(|s| s.trim().to_string());

                if username.is_empty() || password.is_empty() {
                    dispatch_to_webview(
                        web_view,
                        "LOGIN_RESULT",
                        "{\"success\":false,\"message\":\"账号和密码不能为空\"}",
                    );
                    return;
                }

                let svc = ANKI_SERVICE.lock().unwrap();
                match svc.login(username, password, custom_server.clone()) {
                    Ok(new_hkey) => {
                        let mut config = APP_CONFIG.lock().unwrap();
                        config.hkey = new_hkey;
                        if let Some(srv) = custom_server {
                            config.sync_server = srv;
                        }
                        let _ = config.save();
                        dispatch_to_webview(
                            web_view,
                            "LOGIN_RESULT",
                            "{\"success\":true,\"message\":\"登录成功！密钥已安全保存。\"}",
                        );
                    }
                    Err(err) => {
                        let err_json = serde_json::to_string(&serde_json::json!({
                            "success": false,
                            "message": err
                        })).unwrap_or_else(|_| "{\"success\":false,\"message\":\"登录失败\"}".to_string());
                        dispatch_to_webview(web_view, "LOGIN_RESULT", &err_json);
                    }
                }
            }
        }
        "SYNC_COLLECTION" => {
            let (hkey, sync_server) = {
                let config = APP_CONFIG.lock().unwrap();
                (config.hkey.clone(), config.sync_server.clone())
            };

            if hkey.trim().is_empty() {
                dispatch_to_webview(
                    web_view,
                    "SYNC_RESULT",
                    "{\"success\":false,\"need_login\":true,\"message\":\"尚未配置 AnkiWeb 账号，请先点击设置进行登录。\"}",
                );
                return;
            }

            let endpoint = if sync_server.trim().is_empty() {
                None
            } else {
                Some(sync_server)
            };

            let mut svc = ANKI_SERVICE.lock().unwrap();
            match svc.sync(&hkey, endpoint) {
                Ok(msg) => {
                    let decks = svc.get_deck_list().unwrap_or_default();
                    let studied_today = svc.studied_today();
                    if let Ok(json) = serde_json::to_string(&serde_json::json!({
                        "success": true,
                        "message": msg,
                        "decks": decks,
                        "studied_today": studied_today
                    })) {
                        dispatch_to_webview(web_view, "SYNC_RESULT", &json);
                    }
                }
                Err(err) => {
                    let err_json = serde_json::to_string(&serde_json::json!({
                        "success": false,
                        "message": err
                    })).unwrap_or_else(|_| "{\"success\":false,\"message\":\"同步失败\"}".to_string());
                    dispatch_to_webview(web_view, "SYNC_RESULT", &err_json);
                }
            }
        }
        "SELECT_DECK" => {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(payload) {
                if let Some(did) = v["deck_id"].as_i64() {
                    let mut svc = ANKI_SERVICE.lock().unwrap();
                    let _ = svc.select_deck(did);
                    match svc.get_pipeline_cards() {
                        Ok(Some(pipeline)) => {
                            if let Ok(json) = serde_json::to_string(&pipeline) {
                                dispatch_to_webview(web_view, "SHOW_CARD_PIPELINE", &json);
                            }
                        }
                        _ => {
                            // Deck has 0 cards due today! Send notification to frontend instead of silent reload
                            let decks = svc.get_deck_list().unwrap_or_default();
                            let studied_today = svc.studied_today();
                            if let Ok(json) = serde_json::to_string(&serde_json::json!({
                                "message": "🎉 该牌组今日待复习卡片已全部完成！",
                                "decks": decks,
                                "studied_today": studied_today
                            })) {
                                dispatch_to_webview(web_view, "DECK_EMPTY", &json);
                            }
                        }
                    }
                }
            }
        }
        "RATE_AND_FETCH_NEXT" => {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(payload) {
                if let Some(rating) = v["rating"].as_str() {
                    let mut svc = ANKI_SERVICE.lock().unwrap();
                    match svc.rate_and_fetch_next(rating) {
                        Ok(Some(next_card)) => {
                            if let Ok(json) = serde_json::to_string(&serde_json::json!({
                                "next_card": next_card,
                                "has_next": true
                            })) {
                                dispatch_to_webview(web_view, "PREPARE_NEXT_CARD", &json);
                            }
                        }
                        Ok(None) => {
                            dispatch_to_webview(web_view, "PREPARE_NEXT_CARD", "{\"has_next\":false}");
                        }
                        Err(e) => {
                            eprintln!("[ANKI] rate_and_fetch_next error: {}", e);
                            if let Ok(decks) = svc.get_deck_list() {
                                let studied_today = svc.studied_today();
                                if let Ok(json) = serde_json::to_string(&serde_json::json!({
                                    "decks": decks,
                                    "studied_today": studied_today
                                })) {
                                    dispatch_to_webview(web_view, "LOAD_DECK_LIST", &json);
                                }
                            }
                        }
                    }
                }
            }
        }
        "RATE_CARD" => {
            // Backward-compatible fallback
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(payload) {
                if let Some(rating) = v["rating"].as_str() {
                    let mut svc = ANKI_SERVICE.lock().unwrap();
                    let _ = svc.rate_card(rating);
                    match svc.get_pipeline_cards() {
                        Ok(Some(pipeline)) => {
                            if let Ok(json) = serde_json::to_string(&pipeline) {
                                dispatch_to_webview(web_view, "SHOW_CARD_PIPELINE", &json);
                            }
                        }
                        _ => {
                            if let Ok(decks) = svc.get_deck_list() {
                                let studied_today = svc.studied_today();
                                if let Ok(json) = serde_json::to_string(&serde_json::json!({
                                    "decks": decks,
                                    "studied_today": studied_today
                                })) {
                                    dispatch_to_webview(web_view, "LOAD_DECK_LIST", &json);
                                }
                            }
                        }
                    }
                }
            }
        }
        "CARD_FLIPPED" => {
            // Silently absorb CARD_FLIPPED event
        }
        "EXIT_DECK" => {
            let mut svc = ANKI_SERVICE.lock().unwrap();
            let _ = svc.wal_checkpoint();
            if let Ok(decks) = svc.get_deck_list() {
                let studied_today = svc.studied_today();
                if let Ok(json) = serde_json::to_string(&serde_json::json!({
                    "decks": decks,
                    "studied_today": studied_today
                })) {
                    dispatch_to_webview(web_view, "LOAD_DECK_LIST", &json);
                }
            }
        }
        "exit" => {
            println!("[IPC] Exit requested from Web SPA. Checkpointing WAL & quitting GTK loop...");
            let mut svc = ANKI_SERVICE.lock().unwrap();
            let _ = svc.wal_checkpoint();
            unsafe {
                gtk_main_quit();
            }
        }
        _ => {
            println!("[IPC] Unhandled action: '{}'", action);
        }
    }
}

/// Send down-channel command from Rust to WebKit
pub fn dispatch_to_webview(web_view: *mut GtkWidget, event: &str, json_payload: &str) {
    let script = format!(
        "if (window.RAnki && window.RAnki.dispatch) {{ window.RAnki.dispatch(\"{}\", {}); }}",
        event, json_payload
    );
    if let Ok(c_script) = CString::new(script) {
        unsafe {
            webkit_web_view_execute_script(web_view, c_script.as_ptr());
        }
    }
}

/// C callback for "notify::title" GObject signal on WebKitWebView
pub unsafe extern "C" fn on_webview_title_notify(
    web_view: *mut GtkWidget,
    _pspec: gpointer,
    _user_data: gpointer,
) {
    let title_ptr = webkit_web_view_get_title(web_view);
    if title_ptr.is_null() {
        return;
    }
    let title_cstr = CStr::from_ptr(title_ptr);
    if let Ok(title_str) = title_cstr.to_str() {
        handle_title_change(web_view, title_str);
    }
}

/// C callback for "destroy" GObject signal on GtkWindow
pub unsafe extern "C" fn on_window_destroy(
    _window: *mut GtkWidget,
    _user_data: gpointer,
) {
    let mut svc = ANKI_SERVICE.lock().unwrap();
    let _ = svc.wal_checkpoint();
    gtk_main_quit();
}
