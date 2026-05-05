use cxx_qt_build::CxxQtBuilder;

fn main() {
    CxxQtBuilder::new()
        .qt_module("Widgets")
        .cpp_file("src/qt_app.cpp")
        .qrc("resources.qrc")
        .files(["src/app_backend.rs"])
        .build();
}
