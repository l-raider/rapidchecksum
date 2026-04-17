use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    CxxQtBuilder::new_qml_module(
        QmlModule::new("com.rapidchecksum.app").qml_file("qml/main.qml"),
    )
    .qt_module("Widgets")
    .cpp_file("src/qt_app.cpp")
    .files(["src/app_backend.rs"])
    .build();
}
