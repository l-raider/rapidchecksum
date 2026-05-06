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
    fn qt_queue_startup_sfv(path: *const std::ffi::c_char);
    fn qt_process_startup_sfv();
}

fn main() {
    let startup_sfv_paths: Vec<_> = std::env::args_os()
        .skip(1)
        .filter_map(|arg| {
            let path = std::path::PathBuf::from(arg);
            let is_sfv = path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("sfv"))
                .unwrap_or(false);
            if !is_sfv {
                return None;
            }

            std::ffi::CString::new(path.to_string_lossy().into_owned()).ok()
        })
        .collect();

    // Create QApplication (Qt Widgets) so Qt.labs.platform dialogs work natively
    unsafe { qt_app_init() };

    unsafe { qt_show_main_window() };

    for path in &startup_sfv_paths {
        unsafe { qt_queue_startup_sfv(path.as_ptr()) };
    }
    unsafe { qt_process_startup_sfv() };

    unsafe { qt_app_exec() };
}
