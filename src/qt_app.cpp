#include <QtWidgets/QApplication>
#include <QtGui/QClipboard>
#include <QtGui/QIcon>

static int    s_argc    = 1;
static char   s_argv0[] = "rapidchecksum";
static char*  s_argv[]  = { s_argv0, nullptr };

static QApplication* s_app = nullptr;

extern "C" {
    void qt_app_init()
    {
        if (!s_app) {
            s_app = new QApplication(s_argc, s_argv);
            s_app->setWindowIcon(QIcon(":/icons/hicolor/256x256/apps/io.github.l_raider.rapidchecksum.png"));
        }
    }

    int qt_app_exec()
    {
        return s_app ? s_app->exec() : 1;
    }

    void qt_set_clipboard(const char* text)
    {
        if (s_app) {
            QApplication::clipboard()->setText(QString::fromUtf8(text));
        }
    }
}
