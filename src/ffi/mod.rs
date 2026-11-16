//! Minimal direct C-FFI for GTK2 and WebKit 1.0 (~80 LOC)
#![allow(non_camel_case_types, non_snake_case, dead_code)]

use std::ffi::{c_char, c_int, c_ulong, c_void};

pub type gpointer = *mut c_void;
pub type gboolean = c_int;
pub type gulong = c_ulong;
pub type GCallback = Option<unsafe extern "C" fn()>;
pub type GClosureNotify = Option<unsafe extern "C" fn(data: gpointer, closure: gpointer)>;

pub enum GtkWidget {}
pub enum GParamSpec {}

pub const GTK_WINDOW_TOPLEVEL: c_int = 0;
pub const FALSE: gboolean = 0;
pub const TRUE: gboolean = 1;

extern "C" {
    // GTK Lifecycle
    pub fn gtk_init(argc: *mut c_int, argv: *mut *mut *mut c_char);
    pub fn gtk_main();
    pub fn gtk_main_quit();

    // GtkWindow
    pub fn gtk_window_new(type_: c_int) -> *mut GtkWidget;
    pub fn gtk_window_set_title(window: *mut GtkWidget, title: *const c_char);
    pub fn gtk_window_set_default_size(window: *mut GtkWidget, width: c_int, height: c_int);
    pub fn gtk_window_set_decorated(window: *mut GtkWidget, setting: gboolean);
    pub fn gtk_container_add(container: *mut GtkWidget, widget: *mut GtkWidget);
    pub fn gtk_widget_show_all(widget: *mut GtkWidget);
    pub fn gtk_widget_destroy(widget: *mut GtkWidget);

    // GObject Signals
    pub fn g_signal_connect_data(
        instance: gpointer,
        detailed_signal: *const c_char,
        c_handler: GCallback,
        data: gpointer,
        destroy_data: GClosureNotify,
        connect_flags: c_int,
    ) -> gulong;

    // WebKitGTK 1.0
    pub fn webkit_web_view_new() -> *mut GtkWidget;
    pub fn webkit_web_view_load_uri(web_view: *mut GtkWidget, uri: *const c_char);
    pub fn webkit_web_view_load_string(
        web_view: *mut GtkWidget,
        content: *const c_char,
        mime_type: *const c_char,
        encoding: *const c_char,
        base_uri: *const c_char,
    );
    pub fn webkit_web_view_get_title(web_view: *mut GtkWidget) -> *const c_char;
    pub fn webkit_web_view_execute_script(web_view: *mut GtkWidget, script: *const c_char);
    pub fn webkit_web_view_set_full_content_zoom(web_view: *mut GtkWidget, full_content_zoom: gboolean);
    pub fn webkit_web_view_set_zoom_level(web_view: *mut GtkWidget, zoom_level: f32);
}
