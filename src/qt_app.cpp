#include <algorithm>

#include <QtWidgets/QApplication>
#include <QtWidgets/QAbstractItemView>
#include <QtWidgets/QFileDialog>
#include <QtWidgets/QHeaderView>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QMainWindow>
#include <QtWidgets/QMenu>
#include <QtWidgets/QProgressBar>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QTableView>
#include <QtWidgets/QVBoxLayout>
#include <QtWidgets/QWidget>
#include <QtGui/QClipboard>
#include <QtGui/QIcon>
#include <QtCore/QItemSelectionModel>

#include "rapidchecksum/src/app_backend.cxxqt.h"

static int    s_argc    = 1;
static char   s_argv0[] = "rapidchecksum";
static char*  s_argv[]  = { s_argv0, nullptr };

static QApplication* s_app = nullptr;
static QMainWindow*  s_main_window = nullptr;

static QString widget_window_title(const AppBackend* backend)
{
    return QStringLiteral("RapidChecksum %1").arg(backend->getApp_version());
}

static int progress_value(float progress)
{
    auto scaled = static_cast<int>(progress * 1000.0f);
    return std::clamp(scaled, 0, 1000);
}

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

    void qt_show_main_window()
    {
        if (s_main_window) {
            s_main_window->show();
            s_main_window->raise();
            s_main_window->activateWindow();
            return;
        }

        auto* central_widget = new QWidget();
        auto* main_layout = new QVBoxLayout(central_widget);
        auto* toolbar_layout = new QHBoxLayout();
        auto* open_files_button = new QPushButton(QStringLiteral("Open Files..."));
        auto* open_folder_button = new QPushButton(QStringLiteral("Open Folder..."));
        auto* start_button = new QPushButton(QStringLiteral("Start Hashing"));
        auto* cancel_button = new QPushButton(QStringLiteral("Cancel"));
        auto* clear_button = new QPushButton(QStringLiteral("Clear List"));
        auto* remove_button = new QPushButton(QStringLiteral("Remove Selected"));
        auto* file_progress = new QProgressBar();
        auto* global_progress = new QProgressBar();
        auto* status_label = new QLabel();
        auto* table_view = new QTableView();
        auto* backend = new AppBackend(central_widget);

        main_layout->setContentsMargins(6, 6, 6, 6);
        main_layout->setSpacing(4);

        toolbar_layout->setSpacing(4);
        toolbar_layout->addWidget(open_files_button);
        toolbar_layout->addWidget(open_folder_button);
        toolbar_layout->addWidget(start_button);
        toolbar_layout->addWidget(cancel_button);
        toolbar_layout->addWidget(clear_button);
        toolbar_layout->addWidget(remove_button);
        toolbar_layout->addStretch();

        file_progress->setRange(0, 1000);
        file_progress->setTextVisible(false);
        global_progress->setRange(0, 1000);
        global_progress->setTextVisible(false);
        status_label->setTextInteractionFlags(Qt::TextSelectableByMouse);

        table_view->setModel(backend);
        table_view->setSelectionBehavior(QAbstractItemView::SelectRows);
        table_view->setSelectionMode(QAbstractItemView::SingleSelection);
        table_view->setAlternatingRowColors(false);
        table_view->setContextMenuPolicy(Qt::CustomContextMenu);
        table_view->horizontalHeader()->setSectionResizeMode(QHeaderView::Interactive);
        table_view->horizontalHeader()->setStretchLastSection(true);
        table_view->horizontalHeader()->setSectionsClickable(true);
        table_view->horizontalHeader()->setSortIndicatorShown(true);
        table_view->verticalHeader()->setVisible(false);

        QObject::connect(
            table_view->selectionModel(),
            &QItemSelectionModel::currentRowChanged,
            table_view,
            [backend](const QModelIndex& current, const QModelIndex&) {
                backend->select_row(current.isValid() ? current.row() : -1);
            });

        QObject::connect(
            table_view->horizontalHeader(),
            &QHeaderView::sectionClicked,
            table_view,
            [table_view, backend](int section) {
                auto* header = table_view->horizontalHeader();
                auto order = Qt::AscendingOrder;
                if (header->sortIndicatorSection() == section
                    && header->sortIndicatorOrder() == Qt::AscendingOrder) {
                    order = Qt::DescendingOrder;
                }

                header->setSortIndicator(section, order);
                backend->sort_by(section, order == Qt::AscendingOrder);
            });

        QObject::connect(
            table_view,
            &QTableView::customContextMenuRequested,
            table_view,
            [table_view, backend](const QPoint& pos) {
                const QModelIndex index = table_view->indexAt(pos);
                if (!index.isValid()) {
                    return;
                }

                table_view->selectionModel()->setCurrentIndex(
                    index,
                    QItemSelectionModel::ClearAndSelect | QItemSelectionModel::Rows);

                QMenu menu(table_view);
                auto* copy_file_path_action = menu.addAction(QStringLiteral("Copy File Path"));
                QObject::connect(copy_file_path_action, &QAction::triggered, &menu, [backend]() {
                    backend->copy_filepath();
                });

                auto* copy_hash_menu = menu.addMenu(QStringLiteral("Copy Hash"));
                struct HashAction {
                    const char* label;
                    int algo;
                    bool enabled;
                };
                const HashAction hash_actions[] = {
                    {"CRC32", 0, backend->getSetting_crc32()},
                    {"MD5", 1, backend->getSetting_md5()},
                    {"SHA1", 2, backend->getSetting_sha1()},
                    {"SHA256", 3, backend->getSetting_sha256()},
                    {"SHA512", 4, backend->getSetting_sha512()},
                };

                for (const auto& action_def : hash_actions) {
                    if (!action_def.enabled) {
                        continue;
                    }

                    auto* action = copy_hash_menu->addAction(QString::fromLatin1(action_def.label));
                    QObject::connect(action, &QAction::triggered, &menu, [backend, algo = action_def.algo]() {
                        backend->copy_hash(algo);
                    });
                }

                auto* open_folder_action = menu.addAction(QStringLiteral("Open Containing Folder"));
                QObject::connect(open_folder_action, &QAction::triggered, &menu, [backend]() {
                    backend->open_folder();
                });

                menu.addSeparator();
                auto* save_hash_menu = menu.addMenu(QStringLiteral("Save Hash File"));
                const HashAction save_actions[] = {
                    {"CRC32 / SFV", 0, backend->getSetting_crc32()},
                    {"MD5", 1, backend->getSetting_md5()},
                    {"SHA1", 2, backend->getSetting_sha1()},
                    {"SHA256", 3, backend->getSetting_sha256()},
                    {"SHA512", 4, backend->getSetting_sha512()},
                };

                for (const auto& action_def : save_actions) {
                    if (!action_def.enabled) {
                        continue;
                    }

                    auto* action = save_hash_menu->addAction(QString::fromLatin1(action_def.label));
                    QObject::connect(
                        action,
                        &QAction::triggered,
                        &menu,
                        [backend, table_view, algo = action_def.algo]() {
                            const auto path = QFileDialog::getSaveFileName(
                                table_view,
                                QStringLiteral("Save hash file"));
                            if (!path.isEmpty()) {
                                backend->save_hash_file(algo, path);
                            }
                        });
                }

                menu.exec(table_view->viewport()->mapToGlobal(pos));
            });

        QObject::connect(
            open_files_button,
            &QPushButton::clicked,
            table_view,
            [backend, central_widget]() {
                const auto files = QFileDialog::getOpenFileNames(
                    central_widget,
                    QStringLiteral("Select files to hash"));
                if (!files.isEmpty()) {
                    backend->add_files(files);
                }
            });

        QObject::connect(
            open_folder_button,
            &QPushButton::clicked,
            table_view,
            [backend, central_widget]() {
                const auto folder = QFileDialog::getExistingDirectory(
                    central_widget,
                    QStringLiteral("Select folder to add"));
                if (!folder.isEmpty()) {
                    backend->add_folder(folder);
                }
            });

        QObject::connect(start_button, &QPushButton::clicked, backend, [backend]() {
            backend->start_hashing();
        });
        QObject::connect(cancel_button, &QPushButton::clicked, backend, [backend]() {
            backend->cancel_hashing();
        });
        QObject::connect(clear_button, &QPushButton::clicked, backend, [backend]() {
            backend->clear_list();
        });
        QObject::connect(remove_button, &QPushButton::clicked, backend, [backend]() {
            backend->remove_selected();
        });

        auto sync_widget_state = [backend,
                                  open_files_button,
                                  open_folder_button,
                                  start_button,
                                  cancel_button,
                                  clear_button,
                                  remove_button,
                                  file_progress,
                                  global_progress,
                                  status_label]() {
            const bool is_hashing = backend->getIs_hashing();
            const bool has_files = backend->getFile_count() > 0;
            const bool has_selection = backend->getSelected_row() >= 0;

            open_files_button->setEnabled(!is_hashing);
            open_folder_button->setEnabled(!is_hashing);
            start_button->setEnabled(!is_hashing && has_files);
            cancel_button->setEnabled(is_hashing);
            clear_button->setEnabled(!is_hashing && has_files);
            remove_button->setEnabled(!is_hashing && has_selection);

            file_progress->setVisible(is_hashing);
            global_progress->setVisible(is_hashing);
            file_progress->setValue(progress_value(backend->getFile_progress()));
            global_progress->setValue(progress_value(backend->getGlobal_progress()));
            status_label->setText(backend->getStatus_text());
        };

        QObject::connect(backend, &AppBackend::is_hashingChanged, central_widget, sync_widget_state);
        QObject::connect(backend, &AppBackend::file_countChanged, central_widget, sync_widget_state);
        QObject::connect(backend, &AppBackend::selected_rowChanged, central_widget, sync_widget_state);
        QObject::connect(backend, &AppBackend::file_progressChanged, central_widget, sync_widget_state);
        QObject::connect(backend, &AppBackend::global_progressChanged, central_widget, sync_widget_state);
        QObject::connect(backend, &AppBackend::status_textChanged, central_widget, sync_widget_state);

        main_layout->addLayout(toolbar_layout);
        main_layout->addWidget(file_progress);
        main_layout->addWidget(global_progress);
        main_layout->addWidget(status_label);
        main_layout->addWidget(table_view, 1);

        sync_widget_state();

        s_main_window = new QMainWindow();
        s_main_window->setWindowTitle(widget_window_title(backend));
        s_main_window->resize(1000, 700);
        s_main_window->setCentralWidget(central_widget);
        s_main_window->show();
    }

    void qt_set_clipboard(const char* text)
    {
        if (s_app) {
            QApplication::clipboard()->setText(QString::fromUtf8(text));
        }
    }
}
