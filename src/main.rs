//! RAnki Route B Binary Entry Point
mod ffi;
mod ui;

pub(crate) use ranki::{anki, config, logic};

fn main() {
    ui::run_application();
}
