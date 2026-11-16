//! Bidirectional IPC protocol between Rust Host and WebKit Single Page App
use std::ffi::{CStr, CString};
use std::sync::atomic::{AtomicU64, Ordering};
use crate::ffi::*;

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
            println!("[IPC] WebKit SPA ready. Acknowledging handshake...");
            dispatch_to_webview(web_view, "HOST_ACK", "{\"status\":\"ready\"}");
        }
        "exit" => {
            println!("[IPC] Exit requested from Web SPA. Quitting GTK loop...");
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
    gtk_main_quit();
}
