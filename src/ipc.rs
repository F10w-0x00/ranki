//! Bidirectional IPC protocol between Rust Host and WebKit Single Page App
use std::ffi::{CStr, CString};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex};
use crate::anki::AnkiService;
use crate::ffi::*;

pub static ANKI_SERVICE: LazyLock<Mutex<AnkiService>> = LazyLock::new(|| Mutex::new(AnkiService::new()));
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
            println!("[IPC] WebKit SPA ready. Loading deck tree...");
            let mut svc = ANKI_SERVICE.lock().unwrap();
            if svc.is_open() {
                if let Ok(decks) = svc.get_deck_list() {
                    if let Ok(json) = serde_json::to_string(&serde_json::json!({ "decks": decks })) {
                        dispatch_to_webview(web_view, "LOAD_DECK_LIST", &json);
                        return;
                    }
                }
            }
            dispatch_to_webview(web_view, "HOST_ACK", "{\"status\":\"ready\"}");
        }
        "SELECT_DECK" => {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(payload) {
                if let Some(did) = v["deck_id"].as_i64() {
                    let mut svc = ANKI_SERVICE.lock().unwrap();
                    let _ = svc.select_deck(did);
                    match svc.get_next_card() {
                        Ok(Some(card)) => {
                            if let Ok(json) = serde_json::to_string(&card) {
                                dispatch_to_webview(web_view, "SHOW_CARD", &json);
                            }
                        }
                        _ => {
                            if let Ok(decks) = svc.get_deck_list() {
                                if let Ok(json) = serde_json::to_string(&serde_json::json!({ "decks": decks })) {
                                    dispatch_to_webview(web_view, "LOAD_DECK_LIST", &json);
                                }
                            }
                        }
                    }
                }
            }
        }
        "RATE_CARD" => {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(payload) {
                if let Some(rating) = v["rating"].as_str() {
                    let mut svc = ANKI_SERVICE.lock().unwrap();
                    let _ = svc.rate_card(rating);
                    match svc.get_next_card() {
                        Ok(Some(card)) => {
                            if let Ok(json) = serde_json::to_string(&card) {
                                dispatch_to_webview(web_view, "SHOW_CARD", &json);
                            }
                        }
                        _ => {
                            if let Ok(decks) = svc.get_deck_list() {
                                if let Ok(json) = serde_json::to_string(&serde_json::json!({ "decks": decks })) {
                                    dispatch_to_webview(web_view, "LOAD_DECK_LIST", &json);
                                }
                            }
                        }
                    }
                }
            }
        }
        "EXIT_DECK" => {
            let mut svc = ANKI_SERVICE.lock().unwrap();
            let _ = svc.wal_checkpoint();
            if let Ok(decks) = svc.get_deck_list() {
                if let Ok(json) = serde_json::to_string(&serde_json::json!({ "decks": decks })) {
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
