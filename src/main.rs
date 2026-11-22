//! RAnki Route B Binary Entry Point
mod ffi;
mod ui;

pub(crate) use ranki::{anki, config, i18n, logic};

fn main() {
    ui::run_application();
}
