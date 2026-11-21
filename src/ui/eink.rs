//! Kindle PW3 Hardware Integration:
//! - Native Lab126 LIPC virtual keyboard (com.lab126.keyboard open/close)
//! - Hardware GC16 full-screen e-ink refresh (/usr/sbin/eips -f "" with safe fallback)
use std::path::Path;
use std::process::Command;

pub const APP_ID: &str = "ranki.ranki.ranki";

pub fn open_keyboard() {
    if !Path::new("/usr/bin/lipc-set-prop").exists() {
        return;
    }
    println!("[EINK] Opening Kindle virtual keyboard via LIPC...");
    let arg = format!("{}:abc:1", APP_ID);
    let _ = Command::new("/usr/bin/lipc-set-prop")
        .args(["-s", "com.lab126.keyboard", "open", &arg])
        .spawn();
}

pub fn close_keyboard() {
    if !Path::new("/usr/bin/lipc-set-prop").exists() {
        return;
    }
    println!("[EINK] Closing Kindle virtual keyboard via LIPC...");
    let _ = Command::new("/usr/bin/lipc-set-prop")
        .args(["-s", "com.lab126.keyboard", "close", APP_ID])
        .spawn();
}

/// Trigger true hardware GC16 full-screen waveform flash to clear e-ink ghosting.
pub fn trigger_full_refresh() {
    if Path::new("/usr/sbin/eips").exists() {
        println!("[EINK] Triggering hardware GC16 full refresh via eips -f...");
        let _ = Command::new("/usr/sbin/eips")
            .args(["-f", ""])
            .spawn();
        return;
    }
    if Path::new("/usr/bin/lipc-send-event").exists() {
        println!("[EINK] Fallback full refresh via lipc-send-event...");
        let _ = Command::new("/usr/bin/lipc-send-event")
            .args(["com.lab126.pillow", "refreshScreen"])
            .spawn();
    }
}
