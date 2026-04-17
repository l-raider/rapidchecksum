#include <QtWidgets/QApplication>

static int    s_argc    = 1;
static char   s_argv0[] = "rapidchecksum";
static char*  s_argv[]  = { s_argv0, nullptr };

static QApplication* s_app = nullptr;

extern "C" {
    void qt_app_init()
    {
        if (!s_app) {
            s_app = new QApplication(s_argc, s_argv);
        }
    }

    int qt_app_exec()
    {
        return s_app ? s_app->exec() : 1;
    }
}
