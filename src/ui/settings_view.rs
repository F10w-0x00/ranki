//! Page 2: Minimalist Native GTK2 Settings, AnkiWeb Login & Language View
//! - Keeps 3 real GtkEntry widgets in the upper half of the 1072x1448 screen so Kindle's
//!   native LIPC virtual keyboard (bottom ~520px) NEVER covers any input box.
//! - Renders ALL action / font-size / language controls in pure-white PangoCairo (zero gray
//!   GTK2 button theme dithering, zero synthetic fake-bold blur on 300 PPI Carta E-ink!).
use std::ffi::{CStr, CString};
use std::ptr;

use crate::config::AppConfig;
use crate::ffi::cairo::*;
use crate::ffi::gtk::*;
use crate::i18n::UiLang;
use crate::ui::theme::*;

const BTN_SETTINGS_BACK: Rect = Rect::new(20.0, 16.0, 134.0, 52.0);

const CTRL_W: i32 = 984;
const CTRL_H: i32 = 300;

// PangoCairo control hitboxes inside controls_area (984 x 300)
const BTN_LOGIN: Rect = Rect::new(0.0, 8.0, 316.0, 68.0);
const BTN_SAVE_SRV: Rect = Rect::new(334.0, 8.0, 316.0, 68.0);
const BTN_LOGOUT: Rect = Rect::new(668.0, 8.0, 316.0, 68.0);

const BTN_FONT_DOWN: Rect = Rect::new(608.0, 122.0, 116.0, 66.0);
const RECT_FONT_VAL: Rect = Rect::new(730.0, 122.0, 132.0, 66.0);
const BTN_FONT_UP: Rect = Rect::new(868.0, 122.0, 116.0, 66.0);

const BTN_LANG_EN: Rect = Rect::new(608.0, 212.0, 180.0, 66.0);
const BTN_LANG_ZH: Rect = Rect::new(804.0, 212.0, 180.0, 66.0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsAction {
    None,
    Login,
    SaveServer,
    Logout,
    FontDown,
    FontUp,
    SetLang(UiLang),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CtrlPress {
    None,
    Login,
    SaveServer,
    Logout,
    FontDown,
    FontUp,
    LangEn,
    LangZh,
}

pub struct SettingsView {
    pub root: *mut GtkWidget,
    pub top_area: *mut GtkWidget,
    pub status_label: *mut GtkWidget,
    pub lbl_user: *mut GtkWidget,
    pub lbl_pass: *mut GtkWidget,
    pub lbl_srv: *mut GtkWidget,
    pub entry_user: *mut GtkWidget,
    pub entry_pass: *mut GtkWidget,
    pub entry_server: *mut GtkWidget,
    pub controls_area: *mut GtkWidget,

    pub font_size_px: u32,
    pub lang: UiLang,
    back_pressed: bool,
    ctrl_pressed: CtrlPress,
}

impl SettingsView {
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn new(
        top_expose: unsafe extern "C" fn(*mut GtkWidget, *mut GdkEventExpose, gpointer) -> gboolean,
        top_press: unsafe extern "C" fn(*mut GtkWidget, *mut GdkEventButton, gpointer) -> gboolean,
        top_release: unsafe extern "C" fn(*mut GtkWidget, *mut GdkEventButton, gpointer) -> gboolean,
        entry_focus_in: unsafe extern "C" fn(*mut GtkWidget, gpointer, gpointer) -> gboolean,
        entry_focus_out: unsafe extern "C" fn(*mut GtkWidget, gpointer, gpointer) -> gboolean,
        ctrl_expose: unsafe extern "C" fn(*mut GtkWidget, *mut GdkEventExpose, gpointer) -> gboolean,
        ctrl_press: unsafe extern "C" fn(*mut GtkWidget, *mut GdkEventButton, gpointer) -> gboolean,
        ctrl_release: unsafe extern "C" fn(*mut GtkWidget, *mut GdkEventButton, gpointer) -> gboolean,
    ) -> Self {
        let root = gtk_vbox_new(FALSE, 0);
        let white = GdkColor {
            pixel: 0,
            red: 0xFFFF,
            green: 0xFFFF,
            blue: 0xFFFF,
        };
        gtk_widget_modify_bg(root, GTK_STATE_NORMAL, &white);

        // Top 84px header
        let top_area = gtk_drawing_area_new();
        gtk_widget_set_size_request(top_area, SCREEN_W, TOP_BAR_H);
        gtk_widget_add_events(top_area, GDK_BUTTON_PRESS_MASK | GDK_BUTTON_RELEASE_MASK);
        connect_raw(top_area, "expose-event", top_expose as usize);
        connect_raw(top_area, "button-press-event", top_press as usize);
        connect_raw(top_area, "button-release-event", top_release as usize);
        gtk_box_pack_start(root, top_area, FALSE, FALSE, 0);

        // Upper-screen native GtkEntry form (keeps all 3 inputs well above bottom ~520px LIPC keyboard)
        let form = gtk_vbox_new(FALSE, 14);
        gtk_container_set_border_width(form, 44);
        gtk_widget_modify_bg(form, GTK_STATE_NORMAL, &white);

        // Use Regular weight on labels so Kindle FreeType never applies synthetic fake-bold blur to CJK
        let f_label = make_font_desc(26.0, false);
        let f_status = make_font_desc(24.0, false);
        let f_entry = make_font_desc(28.0, false);

        let lang = UiLang::default();
        let status_label = make_label(lang.status_signed_out(), f_status);
        gtk_label_set_line_wrap(status_label, TRUE);
        gtk_box_pack_start(form, status_label, FALSE, FALSE, 4);

        let lbl_user = make_label(lang.lbl_email(), f_label);
        let entry_user = make_entry(f_entry, true, entry_focus_in, entry_focus_out);
        gtk_box_pack_start(form, lbl_user, FALSE, FALSE, 0);
        gtk_box_pack_start(form, entry_user, FALSE, FALSE, 0);

        let lbl_pass = make_label(lang.lbl_password(), f_label);
        let entry_pass = make_entry(f_entry, false, entry_focus_in, entry_focus_out);
        gtk_box_pack_start(form, lbl_pass, FALSE, FALSE, 0);
        gtk_box_pack_start(form, entry_pass, FALSE, FALSE, 0);

        let lbl_srv = make_label(lang.lbl_server(), f_label);
        let entry_server = make_entry(f_entry, true, entry_focus_in, entry_focus_out);
        gtk_box_pack_start(form, lbl_srv, FALSE, FALSE, 0);
        gtk_box_pack_start(form, entry_server, FALSE, FALSE, 0);

        // Pure-white PangoCairo controls (Account buttons + Font size + Interface language)
        let controls_area = gtk_drawing_area_new();
        gtk_widget_set_size_request(controls_area, CTRL_W, CTRL_H);
        gtk_widget_add_events(
            controls_area,
            GDK_BUTTON_PRESS_MASK | GDK_BUTTON_RELEASE_MASK,
        );
        connect_raw(controls_area, "expose-event", ctrl_expose as usize);
        connect_raw(controls_area, "button-press-event", ctrl_press as usize);
        connect_raw(controls_area, "button-release-event", ctrl_release as usize);
        gtk_box_pack_start(form, controls_area, FALSE, FALSE, 10);

        pango_font_description_free(f_label);
        pango_font_description_free(f_status);
        pango_font_description_free(f_entry);

        gtk_box_pack_start(root, form, FALSE, FALSE, 0);

        Self {
            root,
            top_area,
            status_label,
            lbl_user,
            lbl_pass,
            lbl_srv,
            entry_user,
            entry_pass,
            entry_server,
            controls_area,
            font_size_px: 42,
            lang,
            back_pressed: false,
            ctrl_pressed: CtrlPress::None,
        }
    }

    pub unsafe fn sync_from_config(&mut self, cfg: &AppConfig, custom_status: Option<&str>) {
        self.lang = cfg.language;
        self.font_size_px = cfg.font_size_px;
        set_label_text(self.lbl_user, cfg.language.lbl_email());
        set_label_text(self.lbl_pass, cfg.language.lbl_password());
        set_label_text(self.lbl_srv, cfg.language.lbl_server());
        self.set_status_for_config(cfg, custom_status);
        set_entry_text(self.entry_server, &cfg.sync_server);
        gtk_widget_queue_draw(self.top_area);
        gtk_widget_queue_draw(self.controls_area);
    }

    pub unsafe fn set_status_for_config(&self, cfg: &AppConfig, custom_status: Option<&str>) {
        let status_text = match custom_status {
            Some(msg) => msg.to_string(),
            None if !cfg.hkey.trim().is_empty() => cfg.language.status_signed_in().to_string(),
            None => cfg.language.status_signed_out().to_string(),
        };
        set_label_text(self.status_label, &status_text);
    }

    pub unsafe fn sync_reading_controls(&mut self, cfg: &AppConfig) {
        self.font_size_px = cfg.font_size_px;
        self.lang = cfg.language;
        gtk_widget_queue_draw(self.controls_area);
    }

    pub unsafe fn read_credentials(&self) -> (String, String, String) {
        (
            get_entry_text(self.entry_user),
            get_entry_text(self.entry_pass),
            get_entry_text(self.entry_server),
        )
    }

    pub unsafe fn clear_password_entry(&self) {
        set_entry_text(self.entry_pass, "");
    }

    pub unsafe fn draw_top(&self, cr: *mut cairo_t) {
        fill_white(cr);
        draw_hline(cr, 0.0, SCREEN_W as f64, (TOP_BAR_H as f64) - 0.75, 1.5, 0.0);
        let btn_bold = !self.lang.is_cjk();
        draw_pill_button(
            cr,
            BTN_SETTINGS_BACK,
            self.lang.btn_back(),
            24.0,
            btn_bold,
            0.0,
            0.0,
            self.back_pressed,
        );
        draw_text_centered(
            cr,
            Rect::new(200.0, 0.0, 672.0, TOP_BAR_H as f64),
            self.lang.settings_title(),
            30.0,
            btn_bold,
            0.0,
        );
    }

    pub unsafe fn on_top_press(&mut self, x: f64, y: f64) {
        self.back_pressed = BTN_SETTINGS_BACK.contains(x, y);
        if self.back_pressed {
            gtk_widget_queue_draw(self.top_area);
        }
    }

    pub unsafe fn on_top_release(&mut self, x: f64, y: f64) -> bool {
        let hit = self.back_pressed && BTN_SETTINGS_BACK.contains(x, y);
        self.back_pressed = false;
        gtk_widget_queue_draw(self.top_area);
        hit
    }

    pub unsafe fn draw_controls(&self, cr: *mut cairo_t) {
        fill_white(cr);

        let btn_bold = !self.lang.is_cjk();

        // Row 1: Account Action Pills (pure white paper pills, crisp black border, zero gray dither)
        draw_pill_button(
            cr,
            BTN_LOGIN,
            self.lang.btn_sign_in(),
            26.0,
            btn_bold,
            0.0,
            0.0,
            self.ctrl_pressed == CtrlPress::Login,
        );
        draw_pill_button(
            cr,
            BTN_SAVE_SRV,
            self.lang.btn_save_server(),
            26.0,
            false,
            0.35,
            0.0,
            self.ctrl_pressed == CtrlPress::SaveServer,
        );
        draw_pill_button(
            cr,
            BTN_LOGOUT,
            self.lang.btn_sign_out(),
            26.0,
            false,
            0.55,
            0.15,
            self.ctrl_pressed == CtrlPress::Logout,
        );

        // Divider
        draw_hline(cr, 0.0, CTRL_W as f64, 98.0, 1.5, 0.78);

        // Row 2: Card Font Size (300 PPI)
        draw_text_left(
            cr,
            Rect::new(0.0, 122.0, 580.0, 66.0),
            self.lang.lbl_font_size(),
            27.0,
            false,
            0.0,
            true,
        );
        draw_pill_button(
            cr,
            BTN_FONT_DOWN,
            "A-",
            28.0,
            true,
            0.0,
            0.0,
            self.ctrl_pressed == CtrlPress::FontDown,
        );
        let font_txt = format!("{} px", self.font_size_px);
        draw_text_centered_for(
            cr,
            RECT_FONT_VAL,
            &font_txt,
            FONT_FAMILY_MONO,
            27.0,
            true,
            0.0,
        );
        draw_pill_button(
            cr,
            BTN_FONT_UP,
            "A+",
            28.0,
            true,
            0.0,
            0.0,
            self.ctrl_pressed == CtrlPress::FontUp,
        );

        // Row 3: Interface Language (English default / 中文)
        draw_text_left(
            cr,
            Rect::new(0.0, 212.0, 580.0, 66.0),
            self.lang.lbl_language(),
            27.0,
            false,
            0.0,
            true,
        );
        let en_active = self.lang == UiLang::En || self.ctrl_pressed == CtrlPress::LangEn;
        let zh_active = self.lang == UiLang::Zh || self.ctrl_pressed == CtrlPress::LangZh;
        draw_pill_button(cr, BTN_LANG_EN, "English", 26.0, true, 0.0, 0.0, en_active);
        draw_pill_button(cr, BTN_LANG_ZH, "中文", 27.0, false, 0.0, 0.0, zh_active);
    }

    pub unsafe fn on_ctrl_press(&mut self, x: f64, y: f64) {
        self.ctrl_pressed = if BTN_LOGIN.contains(x, y) {
            CtrlPress::Login
        } else if BTN_SAVE_SRV.contains(x, y) {
            CtrlPress::SaveServer
        } else if BTN_LOGOUT.contains(x, y) {
            CtrlPress::Logout
        } else if BTN_FONT_DOWN.contains(x, y) {
            CtrlPress::FontDown
        } else if BTN_FONT_UP.contains(x, y) {
            CtrlPress::FontUp
        } else if BTN_LANG_EN.contains(x, y) {
            CtrlPress::LangEn
        } else if BTN_LANG_ZH.contains(x, y) {
            CtrlPress::LangZh
        } else {
            CtrlPress::None
        };
        if self.ctrl_pressed != CtrlPress::None {
            gtk_widget_queue_draw(self.controls_area);
        }
    }

    pub unsafe fn on_ctrl_release(&mut self, x: f64, y: f64) -> SettingsAction {
        let was = self.ctrl_pressed;
        self.ctrl_pressed = CtrlPress::None;
        gtk_widget_queue_draw(self.controls_area);

        match was {
            CtrlPress::Login if BTN_LOGIN.contains(x, y) => SettingsAction::Login,
            CtrlPress::SaveServer if BTN_SAVE_SRV.contains(x, y) => SettingsAction::SaveServer,
            CtrlPress::Logout if BTN_LOGOUT.contains(x, y) => SettingsAction::Logout,
            CtrlPress::FontDown if BTN_FONT_DOWN.contains(x, y) => SettingsAction::FontDown,
            CtrlPress::FontUp if BTN_FONT_UP.contains(x, y) => SettingsAction::FontUp,
            CtrlPress::LangEn if BTN_LANG_EN.contains(x, y) => SettingsAction::SetLang(UiLang::En),
            CtrlPress::LangZh if BTN_LANG_ZH.contains(x, y) => SettingsAction::SetLang(UiLang::Zh),
            _ => SettingsAction::None,
        }
    }
}

unsafe fn make_label(text: &str, font: *mut PangoFontDescription) -> *mut GtkWidget {
    let c = CString::new(text).unwrap_or_else(|_| CString::new("").unwrap());
    let w = gtk_label_new(c.as_ptr());
    gtk_misc_set_alignment(w, 0.0, 0.5);
    gtk_widget_modify_font(w, font as *mut std::ffi::c_void);
    w
}

unsafe fn make_entry(
    font: *mut PangoFontDescription,
    visible: bool,
    focus_in: unsafe extern "C" fn(*mut GtkWidget, gpointer, gpointer) -> gboolean,
    focus_out: unsafe extern "C" fn(*mut GtkWidget, gpointer, gpointer) -> gboolean,
) -> *mut GtkWidget {
    let e = gtk_entry_new();
    gtk_widget_set_size_request(e, -1, 60);
    gtk_widget_modify_font(e, font as *mut std::ffi::c_void);
    gtk_entry_set_visibility(e, if visible { TRUE } else { FALSE });
    connect_raw(e, "focus-in-event", focus_in as usize);
    connect_raw(e, "focus-out-event", focus_out as usize);
    e
}

pub unsafe fn set_label_text(label: *mut GtkWidget, text: &str) {
    if let Ok(c) = CString::new(text) {
        gtk_label_set_text(label, c.as_ptr());
    }
}

pub unsafe fn set_entry_text(entry: *mut GtkWidget, text: &str) {
    if let Ok(c) = CString::new(text) {
        gtk_entry_set_text(entry, c.as_ptr());
    }
}

pub unsafe fn get_entry_text(entry: *mut GtkWidget) -> String {
    let ptr = gtk_entry_get_text(entry);
    if ptr.is_null() {
        return String::new();
    }
    CStr::from_ptr(ptr).to_string_lossy().into_owned()
}

unsafe fn connect_raw(widget: *mut GtkWidget, sig_name: &str, handler: usize) {
    let sig = CString::new(sig_name).unwrap();
    g_signal_connect_data(
        widget as gpointer,
        sig.as_ptr(),
        Some(std::mem::transmute(handler)),
        ptr::null_mut(),
        None,
        0,
    );
}
