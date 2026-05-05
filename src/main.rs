pub mod app_backend;
mod config;
mod fileio;
mod hasher;
mod model;
mod worker;

// Avoid conflict between `mod hasher::sha1` and the `sha1` crate
extern crate sha1 as sha1_crate;

extern "C" {
    fn qt_app_init();
    fn qt_show_main_window();
    fn qt_app_exec() -> i32;
}

fn main() {
    // Create QApplication (Qt Widgets) so Qt.labs.platform dialogs work natively
    unsafe { qt_app_init() };

    unsafe { qt_show_main_window() };

    unsafe { qt_app_exec() };
}
