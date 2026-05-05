pub mod app_backend;
mod config;
mod fileio;
mod hasher;
mod model;
mod worker;

// Avoid conflict between \`mod hasher::sha1\` and the \`sha1\` crate
extern crate sha1 as sha1_crate;

use cxx_qt_lib::{QQmlApplicationEngine, QUrl};

extern "C" {
    fn qt_app_init();
    fn qt_show_main_window();
    fn qt_app_exec() -> i32;
}

fn main() {
    // Create QApplication (Qt Widgets) so Qt.labs.platform dialogs work natively
    unsafe { qt_app_init() };

    let use_widgets_ui = std::env::var("RAPIDCHECKSUM_UI")
        .map(|value| value.eq_ignore_ascii_case("widgets"))
        .unwrap_or(false);

    if use_widgets_ui {
        unsafe { qt_show_main_window() };
        unsafe { qt_app_exec() };
        return;
    }

    // Use org.kde.desktop style: native KDE look without Breeze QML bugs
    std::env::set_var("QT_QUICK_CONTROLS_STYLE", "org.kde.desktop");

    let mut engine = QQmlApplicationEngine::new();
    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from(
            "qrc:/qt/qml/com/rapidchecksum/app/qml/main.qml",
        ));
    }

    unsafe { qt_app_exec() };
}
