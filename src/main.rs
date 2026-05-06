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
    fn qt_queue_startup_add(path: *const std::ffi::c_char);
    fn qt_process_startup_add();
}

fn main() {
    let mut startup_sfv_paths: Vec<std::ffi::CString> = Vec::new();
    let mut startup_add_paths: Vec<std::ffi::CString> = Vec::new();

    for arg in std::env::args_os().skip(1) {
        let path = std::path::PathBuf::from(&arg);
        let is_sfv = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("sfv"))
            .unwrap_or(false);
        if let Ok(cstr) = std::ffi::CString::new(path.to_string_lossy().into_owned()) {
            if is_sfv {
                startup_sfv_paths.push(cstr);
            } else {
                startup_add_paths.push(cstr);
            }
        }
    }

    // Create QApplication (Qt Widgets) so Qt.labs.platform dialogs work natively
    unsafe { qt_app_init() };

    unsafe { qt_show_main_window() };

    for path in &startup_sfv_paths {
        unsafe { qt_queue_startup_sfv(path.as_ptr()) };
    }
    unsafe { qt_process_startup_sfv() };

    for path in &startup_add_paths {
        unsafe { qt_queue_startup_add(path.as_ptr()) };
    }
    unsafe { qt_process_startup_add() };

        std::process::exit(unsafe { qt_app_exec() });
}
