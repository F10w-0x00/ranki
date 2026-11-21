//! Minimal WebKitGTK 1.0 C-FFI bindings (card canvas only)
use std::ffi::c_char;
use super::gtk::GtkWidget;

extern "C" {
    pub fn webkit_web_view_new() -> *mut GtkWidget;
    pub fn webkit_web_view_load_uri(web_view: *mut GtkWidget, uri: *const c_char);
    pub fn webkit_web_view_get_title(web_view: *mut GtkWidget) -> *const c_char;
    pub fn webkit_web_view_execute_script(web_view: *mut GtkWidget, script: *const c_char);
}
