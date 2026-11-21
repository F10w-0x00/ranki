//! Thin GTK+ 2.24 / GDK 2.24 / GLib 2.0 C-FFI bindings (exact 32-bit ARM EABI struct layouts)
use std::ffi::{c_char, c_double, c_int, c_uint, c_ulong, c_void};

pub type gpointer = *mut c_void;
pub type gboolean = c_int;
pub type gulong = c_ulong;
pub type guint = c_uint;
pub type gint = c_int;
pub type GCallback = Option<unsafe extern "C" fn()>;
pub type GClosureNotify = Option<unsafe extern "C" fn(data: gpointer, closure: gpointer)>;
pub type GSourceFunc = unsafe extern "C" fn(data: gpointer) -> gboolean;

pub enum GtkWidget {}
pub enum GtkWindow {}
pub enum GtkDialog {}
pub enum GtkContainer {}
pub enum GtkBox {}
pub enum GtkNotebook {}
pub enum GtkEntry {}
pub enum GtkLabel {}
pub enum GtkButton {}
pub enum GdkWindow {}

pub const GTK_WINDOW_TOPLEVEL: c_int = 0;
pub const GTK_WIN_POS_CENTER_ON_PARENT: c_int = 4;
pub const GTK_DIALOG_MODAL: c_int = 1;
pub const GTK_MESSAGE_INFO: c_int = 0;
pub const GTK_MESSAGE_WARNING: c_int = 1;
pub const GTK_MESSAGE_QUESTION: c_int = 2;
pub const GTK_MESSAGE_ERROR: c_int = 3;
pub const GTK_BUTTONS_NONE: c_int = 0;
pub const GTK_BUTTONS_OK: c_int = 1;
pub const GTK_BUTTONS_CLOSE: c_int = 2;

pub const FALSE: gboolean = 0;
pub const TRUE: gboolean = 1;

pub const GDK_EXPOSURE_MASK: c_int = 1 << 1;
pub const GDK_POINTER_MOTION_MASK: c_int = 1 << 2;
pub const GDK_BUTTON_PRESS_MASK: c_int = 1 << 8;
pub const GDK_BUTTON_RELEASE_MASK: c_int = 1 << 9;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct GdkRectangle {
    pub x: c_int,
    pub y: c_int,
    pub width: c_int,
    pub height: c_int,
}

pub type GtkAllocation = GdkRectangle;

#[repr(C)]
pub struct GdkEventExpose {
    pub type_: c_int,
    pub window: *mut GdkWindow,
    pub send_event: i8,
    pub area: GdkRectangle,
    pub region: *mut c_void,
    pub count: c_int,
}

#[repr(C)]
pub struct GdkEventButton {
    pub type_: c_int,
    pub window: *mut GdkWindow,
    pub send_event: i8,
    pub time: u32,
    pub x: c_double,
    pub y: c_double,
    pub axes: *mut c_double,
    pub state: c_uint,
    pub button: c_uint,
    pub device: *mut c_void,
    pub x_root: c_double,
    pub y_root: c_double,
}

#[repr(C)]
pub struct GdkEventMotion {
    pub type_: c_int,
    pub window: *mut GdkWindow,
    pub send_event: i8,
    pub time: u32,
    pub x: c_double,
    pub y: c_double,
    pub axes: *mut c_double,
    pub state: c_uint,
    pub is_hint: i16,
    pub device: *mut c_void,
    pub x_root: c_double,
    pub y_root: c_double,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct GdkColor {
    pub pixel: u32,
    pub red: u16,
    pub green: u16,
    pub blue: u16,
}

pub const GTK_STATE_NORMAL: c_int = 0;

extern "C" {
    // GTK Lifecycle
    pub fn gtk_init(argc: *mut c_int, argv: *mut *mut *mut c_char);
    pub fn gtk_main();
    pub fn gtk_main_quit();

    // GLib Event Loop Sources
    pub fn g_timeout_add(interval: guint, function: GSourceFunc, data: gpointer) -> guint;
    pub fn g_idle_add(function: GSourceFunc, data: gpointer) -> guint;
    pub fn g_source_remove(tag: guint) -> gboolean;

    // GObject Signals
    pub fn g_signal_connect_data(
        instance: gpointer,
        detailed_signal: *const c_char,
        c_handler: GCallback,
        data: gpointer,
        destroy_data: GClosureNotify,
        connect_flags: c_int,
    ) -> gulong;

    // GtkWindow
    pub fn gtk_window_new(type_: c_int) -> *mut GtkWidget;
    pub fn gtk_window_set_title(window: *mut GtkWidget, title: *const c_char);
    pub fn gtk_window_set_default_size(window: *mut GtkWidget, width: c_int, height: c_int);
    pub fn gtk_window_set_decorated(window: *mut GtkWidget, setting: gboolean);
    pub fn gtk_window_set_transient_for(window: *mut GtkWidget, parent: *mut GtkWidget);
    pub fn gtk_window_set_position(window: *mut GtkWidget, position: c_int);

    // GtkWidget
    pub fn gtk_widget_show_all(widget: *mut GtkWidget);
    pub fn gtk_widget_show(widget: *mut GtkWidget);
    pub fn gtk_widget_hide(widget: *mut GtkWidget);
    pub fn gtk_widget_destroy(widget: *mut GtkWidget);
    pub fn gtk_widget_set_size_request(widget: *mut GtkWidget, width: c_int, height: c_int);
    pub fn gtk_widget_add_events(widget: *mut GtkWidget, events: c_int);
    pub fn gtk_widget_queue_draw(widget: *mut GtkWidget);
    pub fn gtk_widget_queue_draw_area(
        widget: *mut GtkWidget,
        x: c_int,
        y: c_int,
        width: c_int,
        height: c_int,
    );
    pub fn gtk_widget_get_allocation(widget: *mut GtkWidget, allocation: *mut GtkAllocation);
    pub fn gtk_widget_modify_bg(widget: *mut GtkWidget, state: c_int, color: *const GdkColor);
    pub fn gtk_widget_modify_font(widget: *mut GtkWidget, font_desc: *mut c_void);
    pub fn gtk_widget_set_sensitive(widget: *mut GtkWidget, sensitive: gboolean);
    pub fn gtk_widget_grab_focus(widget: *mut GtkWidget);

    // Containers & Boxes
    pub fn gtk_container_add(container: *mut GtkWidget, widget: *mut GtkWidget);
    pub fn gtk_container_set_border_width(container: *mut GtkWidget, border_width: c_uint);
    pub fn gtk_vbox_new(homogeneous: gboolean, spacing: c_int) -> *mut GtkWidget;
    pub fn gtk_hbox_new(homogeneous: gboolean, spacing: c_int) -> *mut GtkWidget;
    pub fn gtk_box_pack_start(
        box_: *mut GtkWidget,
        child: *mut GtkWidget,
        expand: gboolean,
        fill: gboolean,
        padding: c_uint,
    );
    pub fn gtk_box_pack_end(
        box_: *mut GtkWidget,
        child: *mut GtkWidget,
        expand: gboolean,
        fill: gboolean,
        padding: c_uint,
    );

    // GtkNotebook (tabless instant page switcher)
    pub fn gtk_notebook_new() -> *mut GtkWidget;
    pub fn gtk_notebook_set_show_tabs(notebook: *mut GtkWidget, show_tabs: gboolean);
    pub fn gtk_notebook_set_show_border(notebook: *mut GtkWidget, show_border: gboolean);
    pub fn gtk_notebook_append_page(
        notebook: *mut GtkWidget,
        child: *mut GtkWidget,
        tab_label: *mut GtkWidget,
    ) -> c_int;
    pub fn gtk_notebook_set_current_page(notebook: *mut GtkWidget, page_num: c_int);
    pub fn gtk_notebook_get_current_page(notebook: *mut GtkWidget) -> c_int;

    // GtkDrawingArea
    pub fn gtk_drawing_area_new() -> *mut GtkWidget;

    // Native Widgets for Minimalist Settings / Login / Modals
    pub fn gtk_label_new(str_: *const c_char) -> *mut GtkWidget;
    pub fn gtk_label_set_text(label: *mut GtkWidget, str_: *const c_char);
    pub fn gtk_label_set_line_wrap(label: *mut GtkWidget, wrap: gboolean);
    pub fn gtk_misc_set_alignment(misc: *mut GtkWidget, xalign: f32, yalign: f32);

    pub fn gtk_entry_new() -> *mut GtkWidget;
    pub fn gtk_entry_set_text(entry: *mut GtkWidget, text: *const c_char);
    pub fn gtk_entry_get_text(entry: *mut GtkWidget) -> *const c_char;
    pub fn gtk_entry_set_visibility(entry: *mut GtkWidget, visible: gboolean);

    pub fn gtk_button_new_with_label(label: *const c_char) -> *mut GtkWidget;
    pub fn gtk_button_set_label(button: *mut GtkWidget, label: *const c_char);
    pub fn gtk_bin_get_child(bin: *mut GtkWidget) -> *mut GtkWidget;
    pub fn gtk_hseparator_new() -> *mut GtkWidget;

    // GtkDialog
    pub fn gtk_dialog_new() -> *mut GtkWidget;
    pub fn gtk_dialog_get_content_area(dialog: *mut GtkWidget) -> *mut GtkWidget;
    pub fn gtk_dialog_add_button(
        dialog: *mut GtkWidget,
        button_text: *const c_char,
        response_id: c_int,
    ) -> *mut GtkWidget;
    pub fn gtk_dialog_run(dialog: *mut GtkWidget) -> c_int;
}
