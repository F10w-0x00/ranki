mod anki;
mod config;
mod ffi;
mod ipc;

use std::ffi::CString;
use std::ptr;
use ffi::*;
use ipc::{on_webview_title_notify, on_window_destroy, ANKI_SERVICE, APP_CONFIG};

fn main() {
    println!("====================================================");
    println!(" Starting RAnki Unified Web-SPA Pure Rust Host");
    println!(" Target: Kindle Paperwhite 3 (1072x1448)");
    println!("====================================================");

    // 1. Initialize Anki Collection via AppConfig (strictly targets anki_data_test, NEVER anki_data)
    let config = APP_CONFIG.lock().unwrap().clone();
    let (col_path, media_path, media_db_path) = config.get_resolved_collection_paths();

    println!("[HOST] Collection target path: {:?}", col_path);
    if col_path.exists() {
        let mut svc = ANKI_SERVICE.lock().unwrap();
        if let Err(e) = svc.open_collection(&col_path, &media_path, &media_db_path) {
            eprintln!("[ANKI] Warning: Could not open collection at {:?}: {}", col_path, e);
        }
    } else {
        println!("[ANKI] Note: Test collection not found at {:?}. Running in demo/mock mode.", col_path);
    }

    unsafe {
        // 2. Ensure DISPLAY is set for Kindle OS
        if std::env::var("DISPLAY").map(|v| v.is_empty()).unwrap_or(true) {
            println!("[HOST] DISPLAY not set or empty, defaulting to :0");
            std::env::set_var("DISPLAY", ":0");
        }

        // 3. Initialize GTK
        gtk_init(&mut 0, ptr::null_mut());

        // 4. Create Top-level Window
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

        // 5. Create WebKitWebView
        let webview = webkit_web_view_new();
        gtk_container_add(window, webview);

        // 6. Connect notify::title for Bidirectional IPC
        let title_notify_sig = CString::new("notify::title").unwrap();
        g_signal_connect_data(
            webview as gpointer,
            title_notify_sig.as_ptr(),
            Some(std::mem::transmute(on_webview_title_notify as usize)),
            ptr::null_mut(),
            None,
            0,
        );

        // 7. Determine URI to load
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

        // 8. Show all widgets and enter GTK event loop
        gtk_widget_show_all(window);
        println!("[HOST] WebKit window displayed. Entering gtk_main()...");
        gtk_main();

        println!("[HOST] Exited gtk_main(). Goodbye.");
    }
}
