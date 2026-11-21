//! 300 PPI Monochrome Paper-Book Typography & Cairo Drawing Primitives for KPW3 (1072x1448)
use std::f64::consts::PI;
use std::ffi::{c_void, CString};
use std::path::Path;

use crate::ffi::cairo::*;

pub const SCREEN_W: i32 = 1072;
pub const SCREEN_H: i32 = 1448;
pub const TOP_BAR_H: i32 = 84;
pub const BOTTOM_BAR_H: i32 = 128;
pub const DECK_FOOTER_H: i32 = 112;
pub const TIMER_SIZE: i32 = 54;

pub const FONT_FAMILY: &str = "Anthropic Sans Web, SonyReader Ming, Serif";

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub const fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self { x, y, w, h }
    }

    pub fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.x && px <= self.x + self.w && py >= self.y && py <= self.y + self.h
    }
}

/// Register local Anthropic Sans & Kindle SonyReader Ming fonts with Fontconfig so PangoCairo
/// renders the exact same paper-book typography as the card canvas.
pub fn register_app_fonts(views_dir: &Path) {
    let candidates = [
        views_dir.join("fonts/anthropic/AnthropicSans-Regular.ttf"),
        views_dir.join("fonts/anthropic/AnthropicSans-Italic.ttf"),
        Path::new("/mnt/us/fonts/SonyReader Ming.ttf").to_path_buf(),
    ];

    for p in &candidates {
        if p.exists() {
            if let Ok(c_path) = CString::new(p.to_string_lossy().as_bytes()) {
                unsafe {
                    let ok = FcConfigAppFontAddFile(std::ptr::null_mut(), c_path.as_ptr());
                    println!("[FONT] Fontconfig register {:?} -> {}", p, ok != 0);
                }
            }
        }
    }
}

/// Create an absolute-pixel-sized PangoFontDescription (DPI-independent on KPW3 X11!)
pub unsafe fn make_font_desc(px: f64, bold: bool) -> *mut PangoFontDescription {
    let desc = pango_font_description_new();
    let fam = CString::new(FONT_FAMILY).unwrap_or_else(|_| CString::new("Serif").unwrap());
    pango_font_description_set_family(desc, fam.as_ptr());
    pango_font_description_set_weight(
        desc,
        if bold {
            PANGO_WEIGHT_BOLD
        } else {
            PANGO_WEIGHT_NORMAL
        },
    );
    pango_font_description_set_absolute_size(desc, px * (PANGO_SCALE as f64));
    desc
}

pub unsafe fn fill_white(cr: *mut cairo_t) {
    cairo_set_source_rgb(cr, 1.0, 1.0, 1.0);
    cairo_paint(cr);
}

pub unsafe fn draw_hline(cr: *mut cairo_t, x1: f64, x2: f64, y: f64, width: f64, gray: f64) {
    cairo_set_source_rgb(cr, gray, gray, gray);
    cairo_set_line_width(cr, width);
    cairo_move_to(cr, x1, y);
    cairo_line_to(cr, x2, y);
    cairo_stroke(cr);
}

pub unsafe fn rounded_rect_path(cr: *mut cairo_t, r: Rect, radius: f64) {
    let rad = radius.min(r.w / 2.0).min(r.h / 2.0);
    cairo_new_path(cr);
    cairo_arc(cr, r.x + r.w - rad, r.y + rad, rad, -PI / 2.0, 0.0);
    cairo_arc(cr, r.x + r.w - rad, r.y + r.h - rad, rad, 0.0, PI / 2.0);
    cairo_arc(cr, r.x + rad, r.y + r.h - rad, rad, PI / 2.0, PI);
    cairo_arc(cr, r.x + rad, r.y + rad, rad, PI, 1.5 * PI);
    cairo_close_path(cr);
}

pub unsafe fn draw_pill_button(
    cr: *mut cairo_t,
    r: Rect,
    label: &str,
    font_px: f64,
    bold: bool,
    border_gray: f64,
    text_gray: f64,
    pressed: bool,
) {
    rounded_rect_path(cr, r, 8.0);
    if pressed {
        cairo_set_source_rgb(cr, 0.0, 0.0, 0.0);
        cairo_fill_preserve(cr);
        cairo_set_source_rgb(cr, 0.0, 0.0, 0.0);
    } else {
        cairo_set_source_rgb(cr, 1.0, 1.0, 1.0);
        cairo_fill_preserve(cr);
        cairo_set_source_rgb(cr, border_gray, border_gray, border_gray);
    }
    cairo_set_line_width(cr, 1.5);
    cairo_stroke(cr);

    let fg = if pressed { 1.0 } else { text_gray };
    draw_text_centered(cr, r, label, font_px, bold, fg);
}

/// Draw UTF-8 text horizontally centered and vertically centered inside `r`.
pub unsafe fn draw_text_centered(
    cr: *mut cairo_t,
    r: Rect,
    text: &str,
    px: f64,
    bold: bool,
    gray: f64,
) {
    let c_text = match CString::new(text) {
        Ok(s) => s,
        Err(_) => return,
    };
    let layout = pango_cairo_create_layout(cr);
    let desc = make_font_desc(px, bold);
    pango_layout_set_font_description(layout, desc);
    pango_layout_set_text(layout, c_text.as_ptr(), -1);
    pango_layout_set_width(layout, ((r.w - 8.0).max(16.0) as i32) * PANGO_SCALE);
    pango_layout_set_ellipsize(layout, PANGO_ELLIPSIZE_END);
    pango_layout_set_alignment(layout, PANGO_ALIGN_CENTER);

    let mut tw: i32 = 0;
    let mut th: i32 = 0;
    pango_layout_get_pixel_size(layout, &mut tw, &mut th);

    let draw_x = r.x + 4.0;
    let draw_y = r.y + (r.h - th as f64) / 2.0;

    cairo_set_source_rgb(cr, gray, gray, gray);
    cairo_move_to(cr, draw_x, draw_y);
    pango_cairo_show_layout(cr, layout);

    pango_font_description_free(desc);
    g_object_unref(layout as *mut c_void);
}

/// Draw left-aligned text vertically centered inside `r`, returning pixel width drawn.
pub unsafe fn draw_text_left(
    cr: *mut cairo_t,
    r: Rect,
    text: &str,
    px: f64,
    bold: bool,
    gray: f64,
    ellipsize: bool,
) -> (i32, i32) {
    let c_text = match CString::new(text) {
        Ok(s) => s,
        Err(_) => return (0, 0),
    };
    let layout = pango_cairo_create_layout(cr);
    let desc = make_font_desc(px, bold);
    pango_layout_set_font_description(layout, desc);
    pango_layout_set_text(layout, c_text.as_ptr(), -1);
    if ellipsize {
        pango_layout_set_width(layout, (r.w.max(16.0) as i32) * PANGO_SCALE);
        pango_layout_set_ellipsize(layout, PANGO_ELLIPSIZE_END);
    }
    pango_layout_set_alignment(layout, PANGO_ALIGN_LEFT);

    let mut tw: i32 = 0;
    let mut th: i32 = 0;
    pango_layout_get_pixel_size(layout, &mut tw, &mut th);

    let draw_y = r.y + (r.h - th as f64) / 2.0;
    cairo_set_source_rgb(cr, gray, gray, gray);
    cairo_move_to(cr, r.x, draw_y);
    pango_cairo_show_layout(cr, layout);

    pango_font_description_free(desc);
    g_object_unref(layout as *mut c_void);
    (tw, th)
}

/// Draw right-aligned text vertically centered inside `r`.
pub unsafe fn draw_text_right(
    cr: *mut cairo_t,
    r: Rect,
    text: &str,
    px: f64,
    bold: bool,
    gray: f64,
) {
    let c_text = match CString::new(text) {
        Ok(s) => s,
        Err(_) => return,
    };
    let layout = pango_cairo_create_layout(cr);
    let desc = make_font_desc(px, bold);
    pango_layout_set_font_description(layout, desc);
    pango_layout_set_text(layout, c_text.as_ptr(), -1);
    pango_layout_set_width(layout, (r.w.max(16.0) as i32) * PANGO_SCALE);
    pango_layout_set_alignment(layout, PANGO_ALIGN_RIGHT);

    let mut tw: i32 = 0;
    let mut th: i32 = 0;
    pango_layout_get_pixel_size(layout, &mut tw, &mut th);

    let draw_y = r.y + (r.h - th as f64) / 2.0;
    cairo_set_source_rgb(cr, gray, gray, gray);
    cairo_move_to(cr, r.x, draw_y);
    pango_cairo_show_layout(cr, layout);

    pango_font_description_free(desc);
    g_object_unref(layout as *mut c_void);
}
