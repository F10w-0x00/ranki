//! Page 2: Minimalist Native GTK2 Settings & AnkiWeb Login View
//! - Uses real GtkEntry widgets in the upper half of the 1072x1448 screen so Kindle's
//!   native LIPC virtual keyboard (bottom ~520px) NEVER covers any input box.
//! - Radically minimal: AnkiWeb Login/Server, Font Size (28-54px), Math Engine (KaTeX/MathJax).
//!   Built-in 20s circular timer & 20-card auto GC16 refresh need zero settings clutter.
use std::ffi::{CStr, CString};
use std::ptr;

use crate::config::AppConfig;
use crate::ffi::cairo::*;
use crate::ffi::gtk::*;
use crate::ui::theme::*;

const BTN_SETTINGS_BACK: Rect = Rect::new(20.0, 16.0, 118.0, 52.0);

pub struct SettingsView {
    pub root: *mut GtkWidget,
    pub top_area: *mut GtkWidget,
    pub status_label: *mut GtkWidget,
    pub entry_user: *mut GtkWidget,
    pub entry_pass: *mut GtkWidget,
    pub entry_server: *mut GtkWidget,
    pub btn_login: *mut GtkWidget,
    pub btn_save_server: *mut GtkWidget,
    pub btn_logout: *mut GtkWidget,
    pub label_font_val: *mut GtkWidget,
    pub btn_font_down: *mut GtkWidget,
    pub btn_font_up: *mut GtkWidget,
    pub btn_math_engine: *mut GtkWidget,
    back_pressed: bool,
}

impl SettingsView {
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn new(
        top_expose: unsafe extern "C" fn(*mut GtkWidget, *mut GdkEventExpose, gpointer) -> gboolean,
        top_press: unsafe extern "C" fn(*mut GtkWidget, *mut GdkEventButton, gpointer) -> gboolean,
        top_release: unsafe extern "C" fn(*mut GtkWidget, *mut GdkEventButton, gpointer) -> gboolean,
        entry_focus_in: unsafe extern "C" fn(*mut GtkWidget, gpointer, gpointer) -> gboolean,
        entry_focus_out: unsafe extern "C" fn(*mut GtkWidget, gpointer, gpointer) -> gboolean,
        on_login_click: unsafe extern "C" fn(*mut GtkWidget, gpointer),
        on_save_server_click: unsafe extern "C" fn(*mut GtkWidget, gpointer),
        on_logout_click: unsafe extern "C" fn(*mut GtkWidget, gpointer),
        on_font_down_click: unsafe extern "C" fn(*mut GtkWidget, gpointer),
        on_font_up_click: unsafe extern "C" fn(*mut GtkWidget, gpointer),
        on_math_toggle_click: unsafe extern "C" fn(*mut GtkWidget, gpointer),
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

        // Upper-screen native form container (leaves lower ~550px clear for Kindle virtual keyboard!)
        let form = gtk_vbox_new(FALSE, 18);
        gtk_container_set_border_width(form, 44);
        gtk_widget_modify_bg(form, GTK_STATE_NORMAL, &white);

        let f_label = make_font_desc(25.0, true);
        let f_status = make_font_desc(24.0, false);
        let f_entry = make_font_desc(28.0, false);
        let f_btn = make_font_desc(24.0, true);

        let status_label = make_label("账号状态：检测中...", f_status);
        gtk_label_set_line_wrap(status_label, TRUE);
        gtk_box_pack_start(form, status_label, FALSE, FALSE, 4);

        let lbl_user = make_label("AnkiWeb 邮箱 (Email):", f_label);
        let entry_user = make_entry(f_entry, true, entry_focus_in, entry_focus_out);
        gtk_box_pack_start(form, lbl_user, FALSE, FALSE, 0);
        gtk_box_pack_start(form, entry_user, FALSE, FALSE, 0);

        let lbl_pass = make_label("密码 (仅换取安全 Token，明文不落盘):", f_label);
        let entry_pass = make_entry(f_entry, false, entry_focus_in, entry_focus_out);
        gtk_box_pack_start(form, lbl_pass, FALSE, FALSE, 0);
        gtk_box_pack_start(form, entry_pass, FALSE, FALSE, 0);

        let lbl_srv = make_label("自定义同步服务器 (留空使用官方 AnkiWeb):", f_label);
        let entry_server = make_entry(f_entry, true, entry_focus_in, entry_focus_out);
        gtk_box_pack_start(form, lbl_srv, FALSE, FALSE, 0);
        gtk_box_pack_start(form, entry_server, FALSE, FALSE, 0);

        // Account action buttons
        let acct_row = gtk_hbox_new(TRUE, 16);
        let btn_login = make_button("登录并保存 Token", f_btn, on_login_click);
        let btn_save_server = make_button("保存服务器", f_btn, on_save_server_click);
        let btn_logout = make_button("清除登录", f_btn, on_logout_click);
        gtk_box_pack_start(acct_row, btn_login, TRUE, TRUE, 0);
        gtk_box_pack_start(acct_row, btn_save_server, TRUE, TRUE, 0);
        gtk_box_pack_start(acct_row, btn_logout, TRUE, TRUE, 0);
        gtk_box_pack_start(form, acct_row, FALSE, FALSE, 8);

        let sep = gtk_hseparator_new();
        gtk_box_pack_start(form, sep, FALSE, FALSE, 12);

        // Minimalist Reading Settings: Font Size (28-54px)
        let font_row = gtk_hbox_new(FALSE, 18);
        let lbl_font = make_label("卡片正文字号 (300 PPI):", f_label);
        let btn_font_down = make_button("  A-  ", f_btn, on_font_down_click);
        let label_font_val = make_label("42 px", f_label);
        gtk_misc_set_alignment(label_font_val, 0.5, 0.5);
        gtk_widget_set_size_request(label_font_val, 110, 56);
        let btn_font_up = make_button("  A+  ", f_btn, on_font_up_click);
        gtk_box_pack_start(font_row, lbl_font, TRUE, TRUE, 0);
        gtk_box_pack_start(font_row, btn_font_down, FALSE, FALSE, 0);
        gtk_box_pack_start(font_row, label_font_val, FALSE, FALSE, 0);
        gtk_box_pack_start(font_row, btn_font_up, FALSE, FALSE, 0);
        gtk_box_pack_start(form, font_row, FALSE, FALSE, 4);

        // Math engine switch (KaTeX primary / MathJax fallback)
        let math_row = gtk_hbox_new(FALSE, 18);
        let lbl_math = make_label("公式渲染引擎:", f_label);
        let btn_math_engine = make_button("KaTeX 极速 (推荐)", f_btn, on_math_toggle_click);
        gtk_widget_set_size_request(btn_math_engine, 320, 60);
        gtk_box_pack_start(math_row, lbl_math, TRUE, TRUE, 0);
        gtk_box_pack_end(math_row, btn_math_engine, FALSE, FALSE, 0);
        gtk_box_pack_start(form, math_row, FALSE, FALSE, 4);

        pango_font_description_free(f_label);
        pango_font_description_free(f_status);
        pango_font_description_free(f_entry);
        pango_font_description_free(f_btn);

        gtk_box_pack_start(root, form, FALSE, FALSE, 0);

        Self {
            root,
            top_area,
            status_label,
            entry_user,
            entry_pass,
            entry_server,
            btn_login,
            btn_save_server,
            btn_logout,
            label_font_val,
            btn_font_down,
            btn_font_up,
            btn_math_engine,
            back_pressed: false,
        }
    }

    pub unsafe fn sync_from_config(&self, cfg: &AppConfig, custom_status: Option<&str>) {
        self.set_status_for_config(cfg, custom_status);
        set_entry_text(self.entry_server, &cfg.sync_server);
        self.sync_reading_controls(cfg);
    }

    pub unsafe fn set_status_for_config(&self, cfg: &AppConfig, custom_status: Option<&str>) {
        let status_text = match custom_status {
            Some(msg) => msg.to_string(),
            None if !cfg.hkey.trim().is_empty() => {
                "账号状态：已登录 AnkiWeb（安全 Token 已保存，可直接同步）".to_string()
            }
            None => "账号状态：未登录（请输入邮箱与密码登录）".to_string(),
        };
        set_label_text(self.status_label, &status_text);
    }

    pub unsafe fn sync_reading_controls(&self, cfg: &AppConfig) {
        set_label_text(self.label_font_val, &format!("{} px", cfg.font_size_px));

        let engine_label = if cfg.math_engine == "mathjax" {
            "MathJax 2.7.9 兼容"
        } else {
            "KaTeX 极速 (推荐)"
        };
        set_button_label(self.btn_math_engine, engine_label);
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
        draw_pill_button(
            cr,
            BTN_SETTINGS_BACK,
            "< 返回",
            24.0,
            true,
            0.0,
            0.0,
            self.back_pressed,
        );
        draw_text_centered(
            cr,
            Rect::new(200.0, 0.0, 672.0, TOP_BAR_H as f64),
            "账号与阅读设置",
            30.0,
            true,
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
    gtk_widget_set_size_request(e, -1, 62);
    gtk_widget_modify_font(e, font as *mut std::ffi::c_void);
    gtk_entry_set_visibility(e, if visible { TRUE } else { FALSE });
    connect_raw(e, "focus-in-event", focus_in as usize);
    connect_raw(e, "focus-out-event", focus_out as usize);
    e
}

unsafe fn make_button(
    text: &str,
    font: *mut PangoFontDescription,
    on_click: unsafe extern "C" fn(*mut GtkWidget, gpointer),
) -> *mut GtkWidget {
    let c = CString::new(text).unwrap_or_else(|_| CString::new("").unwrap());
    let b = gtk_button_new_with_label(c.as_ptr());
    gtk_widget_set_size_request(b, -1, 62);
    gtk_widget_modify_font(b, font as *mut std::ffi::c_void);
    let child = gtk_bin_get_child(b);
    if !child.is_null() {
        gtk_widget_modify_font(child, font as *mut std::ffi::c_void);
    }
    connect_raw(b, "clicked", on_click as usize);
    b
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

pub unsafe fn set_button_label(button: *mut GtkWidget, text: &str) {
    if let Ok(c) = CString::new(text) {
        gtk_button_set_label(button, c.as_ptr());
    }
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
