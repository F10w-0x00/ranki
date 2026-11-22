//! Thin Cairo, Pango, PangoCairo, and Fontconfig C-FFI bindings
use std::ffi::{c_char, c_double, c_int, c_void};
use super::gtk::GdkWindow;

pub enum cairo_t {}
pub enum PangoLayout {}
pub enum PangoFontDescription {}

pub const CAIRO_ANTIALIAS_DEFAULT: c_int = 0;
pub const CAIRO_ANTIALIAS_NONE: c_int = 1;
pub const CAIRO_ANTIALIAS_GRAY: c_int = 2;

pub const PANGO_ELLIPSIZE_NONE: c_int = 0;
pub const PANGO_ELLIPSIZE_END: c_int = 3;

pub const PANGO_WRAP_WORD: c_int = 0;
pub const PANGO_WRAP_CHAR: c_int = 1;
pub const PANGO_WRAP_WORD_CHAR: c_int = 2;

pub const PANGO_ALIGN_LEFT: c_int = 0;
pub const PANGO_ALIGN_CENTER: c_int = 1;
pub const PANGO_ALIGN_RIGHT: c_int = 2;

pub const PANGO_WEIGHT_NORMAL: c_int = 400;
pub const PANGO_WEIGHT_BOLD: c_int = 700;

pub const PANGO_SCALE: c_int = 1024;

extern "C" {
    // GDK -> Cairo
    pub fn gdk_cairo_create(drawable: *mut GdkWindow) -> *mut cairo_t;

    // Cairo Core Drawing
    pub fn cairo_destroy(cr: *mut cairo_t);
    pub fn cairo_save(cr: *mut cairo_t);
    pub fn cairo_restore(cr: *mut cairo_t);
    pub fn cairo_set_antialias(cr: *mut cairo_t, antialias: c_int);
    pub fn cairo_set_source_rgb(cr: *mut cairo_t, r: c_double, g: c_double, b: c_double);
    pub fn cairo_set_line_width(cr: *mut cairo_t, width: c_double);
    pub fn cairo_paint(cr: *mut cairo_t);
    pub fn cairo_new_path(cr: *mut cairo_t);
    pub fn cairo_move_to(cr: *mut cairo_t, x: c_double, y: c_double);
    pub fn cairo_line_to(cr: *mut cairo_t, x: c_double, y: c_double);
    pub fn cairo_rectangle(
        cr: *mut cairo_t,
        x: c_double,
        y: c_double,
        width: c_double,
        height: c_double,
    );
    pub fn cairo_arc(
        cr: *mut cairo_t,
        xc: c_double,
        yc: c_double,
        radius: c_double,
        angle1: c_double,
        angle2: c_double,
    );
    pub fn cairo_close_path(cr: *mut cairo_t);
    pub fn cairo_fill(cr: *mut cairo_t);
    pub fn cairo_stroke(cr: *mut cairo_t);
    pub fn cairo_fill_preserve(cr: *mut cairo_t);

    // PangoCairo Text Rendering
    pub fn pango_cairo_create_layout(cr: *mut cairo_t) -> *mut PangoLayout;
    pub fn pango_cairo_show_layout(cr: *mut cairo_t, layout: *mut PangoLayout);

    // Pango Layout & Font Description
    pub fn pango_font_description_new() -> *mut PangoFontDescription;
    pub fn pango_font_description_set_family(
        desc: *mut PangoFontDescription,
        family: *const c_char,
    );
    pub fn pango_font_description_set_weight(desc: *mut PangoFontDescription, weight: c_int);
    pub fn pango_font_description_set_absolute_size(
        desc: *mut PangoFontDescription,
        size: c_double,
    );
    pub fn pango_font_description_free(desc: *mut PangoFontDescription);

    pub fn pango_layout_set_font_description(
        layout: *mut PangoLayout,
        desc: *const PangoFontDescription,
    );
    pub fn pango_layout_set_text(layout: *mut PangoLayout, text: *const c_char, length: c_int);
    pub fn pango_layout_set_width(layout: *mut PangoLayout, width: c_int);
    pub fn pango_layout_set_wrap(layout: *mut PangoLayout, wrap: c_int);
    pub fn pango_layout_set_ellipsize(layout: *mut PangoLayout, ellipsize: c_int);
    pub fn pango_layout_set_alignment(layout: *mut PangoLayout, alignment: c_int);
    pub fn pango_layout_get_pixel_size(
        layout: *mut PangoLayout,
        width: *mut c_int,
        height: *mut c_int,
    );
    pub fn g_object_unref(object: *mut c_void);

    // Fontconfig app-local font registration (Anthropic Sans + SonyReader Ming)
    pub fn FcConfigAppFontAddFile(config: *mut c_void, file: *const c_char) -> c_int;
}
