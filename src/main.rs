mod ffi;
mod ipc;

use std::ffi::CString;
use std::ptr;
use ffi::*;
use ipc::{on_webview_title_notify, on_window_destroy};

fn main() {
    println!("====================================================");
    println!(" Starting RAnki Unified Web-SPA Pure Rust Host");
    println!(" Target: Kindle Paperwhite 3 (1072x1448)");
    println!("====================================================");

    unsafe {
        // 1. Initialize GTK
        gtk_init(&mut 0, ptr::null_mut());

        // 2. Create Top-level Window
        let window = gtk_window_new(GTK_WINDOW_TOPLEVEL);

        // Kindle OS Lab126 Window Manager Protocol:
        // L:A = Layer Application, N:application, ID:ranki, O:U = Orientation Upright, PC:N = Prevent Chrome No
        let win_title = CString::new("L:A_N:application_ID:ranki.ranki.ranki_O:U_PC:N").unwrap();
        gtk_window_set_title(window, win_title.as_ptr());
        gtk_window_set_default_size(window, 1072, 1448);
        gtk_window_set_decorated(window, FALSE);

        let destroy_sig = CString::new("destroy").unwrap();
        g_signal_connect_data(
            window as gpointer,
            destroy_sig.as_ptr(),
            Some(std::mem::transmute(on_window_destroy as usize)),
            ptr::null_mut(),
            None,
            0,
        );

        // 3. Create WebKitWebView
        let webview = webkit_web_view_new();
        gtk_container_add(window, webview);

        // 4. Connect notify::title for Bidirectional IPC
        let title_notify_sig = CString::new("notify::title").unwrap();
        g_signal_connect_data(
            webview as gpointer,
            title_notify_sig.as_ptr(),
            Some(std::mem::transmute(on_webview_title_notify as usize)),
            ptr::null_mut(),
            None,
            0,
        );

        // 5. Determine URI to load
        // On device: file:///mnt/us/extensions/ranki-dev/views/index.html
        // Local fallback: views/index.html in current working directory
        let device_path = "/mnt/us/extensions/ranki-dev/views/index.html";
        let uri = if std::path::Path::new(device_path).exists() {
            format!("file://{}", device_path)
        } else {
            let current_dir = std::env::current_dir().unwrap_or_default();
            let local_path = current_dir.join("views").join("index.html");
            format!("file://{}", local_path.display())
        };

        println!("[HOST] Loading SPA URI: {}", uri);
        let c_uri = CString::new(uri).unwrap();
        webkit_web_view_load_uri(webview, c_uri.as_ptr());

        // 6. Show all widgets and enter GTK event loop
        gtk_widget_show_all(window);
        gtk_main();
    }

    println!("[HOST] Exited cleanly.");
}
