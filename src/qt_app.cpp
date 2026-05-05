#include <algorithm>
#include <memory>

#include <QtWidgets/QApplication>
#include <QtWidgets/QAbstractItemView>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QDialog>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFileDialog>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QHeaderView>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QMainWindow>
#include <QtWidgets/QMenu>
#include <QtWidgets/QMenuBar>
#include <QtWidgets/QProgressBar>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QStyledItemDelegate>
#include <QtWidgets/QTableView>
#include <QtWidgets/QVBoxLayout>
#include <QtWidgets/QWidget>
#include <QtGui/QAction>
#include <QtGui/QActionGroup>
#include <QtGui/QClipboard>
#include <QtGui/QFont>
#include <QtGui/QFontDatabase>
#include <QtGui/QFontMetrics>
#include <QtGui/QIcon>
#include <QtGui/QKeySequence>
#include <QtGui/QPalette>
#include <QtCore/QItemSelectionModel>

#include "rapidchecksum/src/app_backend.cxxqt.h"

static int    s_argc    = 1;
static char   s_argv0[] = "rapidchecksum";
static char*  s_argv[]  = { s_argv0, nullptr };

static QApplication* s_app = nullptr;
static QMainWindow*  s_main_window = nullptr;

namespace {

constexpr int ROLE_IS_ERROR = 256;
constexpr int ROLE_VERIFY_STATUS = 258;

class ResultsTableDelegate final : public QStyledItemDelegate {
public:
    explicit ResultsTableDelegate(QObject* parent = nullptr)
        : QStyledItemDelegate(parent)
        , m_fixed_font(QFontDatabase::systemFont(QFontDatabase::FixedFont))
    {
    }

    void initStyleOption(QStyleOptionViewItem* option, const QModelIndex& index) const override
    {
        QStyledItemDelegate::initStyleOption(option, index);

        const bool is_selected = option->state & QStyle::State_Selected;
        const bool is_error = index.data(ROLE_IS_ERROR).toBool();
        const int verify_status = index.data(ROLE_VERIFY_STATUS).toInt();
        const int last_column = index.model()->columnCount(QModelIndex()) - 1;

        option->displayAlignment = Qt::AlignLeft | Qt::AlignVCenter;

        if (index.column() > 0 && index.column() < last_column) {
            option->font = m_fixed_font;
        }

        if (!is_selected && is_error) {
            const QColor error_background = QColor::fromRgbF(0.37, 0.07, 0.07, 1.0);
            option->backgroundBrush = error_background;
            option->palette.setColor(QPalette::Base, error_background);
            option->palette.setColor(QPalette::AlternateBase, error_background);
        }

        if (!is_selected && index.column() == last_column - 1) {
            if (verify_status == 1) {
                option->palette.setColor(QPalette::Text, QColor(QStringLiteral("#4caf50")));
            } else if (verify_status == 2) {
                option->palette.setColor(QPalette::Text, QColor(QStringLiteral("#f44336")));
            }
        }
    }

private:
    QFont m_fixed_font;
};

struct SortState {
    int column = -1;
    Qt::SortOrder order = Qt::AscendingOrder;
};

}

static QString widget_window_title(const AppBackend* backend)
{
    return QStringLiteral("RapidChecksum %1").arg(backend->getApp_version());
}

static int progress_value(float progress)
{
    auto scaled = static_cast<int>(progress * 1000.0f);
    return std::clamp(scaled, 0, 1000);
}

static QString representative_hash_text(const QString& column_name)
{
    if (column_name == QStringLiteral("CRC32")) {
        return QStringLiteral("01234567");
    }
    if (column_name == QStringLiteral("MD5")) {
        return QStringLiteral("0123456789abcdef0123456789abcdef");
    }
    if (column_name == QStringLiteral("SHA1")) {
        return QStringLiteral("0123456789abcdef0123456789abcdef01234567");
    }
    if (column_name == QStringLiteral("SHA256")) {
        return QStringLiteral("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
    }
    if (column_name == QStringLiteral("SHA512")) {
        return QStringLiteral(
            "0123456789abcdef0123456789abcdef"
            "0123456789abcdef0123456789abcdef"
            "0123456789abcdef0123456789abcdef"
            "0123456789abcdef0123456789abcdef");
    }

    return QString();
}

static int padded_text_width(const QFontMetrics& metrics, const QString& text)
{
    return metrics.horizontalAdvance(text) + 18;
}

static void apply_table_column_width_hints(QTableView* table_view)
{
    auto* model = table_view->model();
    if (!model) {
        return;
    }

    const int column_count = model->columnCount(QModelIndex());
    if (column_count <= 0) {
        return;
    }

    const int last_column = column_count - 1;
    const int verify_column = last_column - 1;
    const QFont fixed_font = QFontDatabase::systemFont(QFontDatabase::FixedFont);
    const QFontMetrics fixed_metrics(fixed_font);
    const QFontMetrics default_metrics(table_view->font());

    for (int column = 1; column < verify_column; ++column) {
        const QString header = model->headerData(column, Qt::Horizontal, Qt::DisplayRole).toString();
        int desired_width = padded_text_width(fixed_metrics, header);

        const QString sample = representative_hash_text(header);
        if (!sample.isEmpty()) {
            desired_width = std::max(desired_width, padded_text_width(fixed_metrics, sample));
        }

        table_view->setColumnWidth(column, std::max(table_view->columnWidth(column), desired_width));
    }

    if (verify_column >= 0) {
        const QString verify_header = model->headerData(verify_column, Qt::Horizontal, Qt::DisplayRole).toString();
        const int verify_width = std::max(
            padded_text_width(default_metrics, verify_header),
            padded_text_width(default_metrics, QStringLiteral("Mismatch")));
        table_view->setColumnWidth(verify_column, std::max(table_view->columnWidth(verify_column), verify_width));
    }
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

        s_main_window = new QMainWindow();
        auto* window = s_main_window;
        auto* central_widget = new QWidget(window);
        auto* main_layout = new QVBoxLayout(central_widget);
        auto* toolbar_layout = new QHBoxLayout();
        auto* open_files_button = new QPushButton(QStringLiteral("Open Files..."));
        auto* open_folder_button = new QPushButton(QStringLiteral("Open Folder..."));
        auto* start_button = new QPushButton(QStringLiteral("Start Hashing"));
        auto* cancel_button = new QPushButton(QStringLiteral("Cancel"));
        auto* clear_button = new QPushButton(QStringLiteral("Clear List"));
        auto* remove_button = new QPushButton(QStringLiteral("Remove Selected"));
        auto* rename_button = new QPushButton(QStringLiteral("Rename Files"));
        auto* file_progress = new QProgressBar();
        auto* global_progress = new QProgressBar();
        auto* status_label = new QLabel();
        auto* table_view = new QTableView();
        auto* backend = new AppBackend(central_widget);
        auto* open_files_action = new QAction(QStringLiteral("Open Files..."), window);
        auto* open_folder_action = new QAction(QStringLiteral("Open Folder..."), window);
        auto* remove_selected_action = new QAction(QStringLiteral("Remove Selected"), window);
        auto* exit_action = new QAction(QStringLiteral("Exit"), window);
        auto* hash_algorithms_action = new QAction(QStringLiteral("Hash Algorithms..."), window);
        auto* file_renaming_action = new QAction(QStringLiteral("File Renaming..."), window);
        auto* hash_casing_menu = new QMenu(QStringLiteral("Hash Casing"), window);
        auto* uppercase_hash_action = new QAction(QStringLiteral("Upper Case"), hash_casing_menu);
        auto* lowercase_hash_action = new QAction(QStringLiteral("Lower Case"), hash_casing_menu);
        auto* hash_casing_group = new QActionGroup(window);
        QFont fixed_font = QFontDatabase::systemFont(QFontDatabase::FixedFont);
        auto sort_state = std::make_shared<SortState>();

        uppercase_hash_action->setCheckable(true);
        lowercase_hash_action->setCheckable(true);
        hash_casing_group->setExclusive(true);
        hash_casing_group->addAction(uppercase_hash_action);
        hash_casing_group->addAction(lowercase_hash_action);
        uppercase_hash_action->setChecked(backend->getSetting_hash_uppercase());
        lowercase_hash_action->setChecked(!backend->getSetting_hash_uppercase());
        hash_casing_menu->addAction(uppercase_hash_action);
        hash_casing_menu->addAction(lowercase_hash_action);

        open_files_action->setShortcut(QKeySequence(QStringLiteral("Ctrl+O")));
        open_folder_action->setShortcut(QKeySequence(QStringLiteral("Ctrl+L")));
        remove_selected_action->setShortcut(QKeySequence(QStringLiteral("Delete")));
        exit_action->setShortcut(QKeySequence(QStringLiteral("Ctrl+Q")));

        window->addAction(open_files_action);
        window->addAction(open_folder_action);
        window->addAction(remove_selected_action);
        window->addAction(exit_action);

        main_layout->setContentsMargins(6, 6, 6, 6);
        main_layout->setSpacing(4);

        toolbar_layout->setSpacing(4);
        toolbar_layout->addWidget(open_files_button);
        toolbar_layout->addWidget(open_folder_button);
        toolbar_layout->addWidget(start_button);
        toolbar_layout->addWidget(cancel_button);
        toolbar_layout->addWidget(clear_button);
        toolbar_layout->addWidget(remove_button);
        toolbar_layout->addWidget(rename_button);
        toolbar_layout->addStretch();

        file_progress->setRange(0, 1000);
        file_progress->setTextVisible(false);
        global_progress->setRange(0, 1000);
        global_progress->setTextVisible(false);
        status_label->setTextInteractionFlags(Qt::TextSelectableByMouse);

        table_view->setModel(backend);
        table_view->setSelectionBehavior(QAbstractItemView::SelectRows);
        table_view->setSelectionMode(QAbstractItemView::SingleSelection);
        table_view->setAlternatingRowColors(true);
        table_view->setContextMenuPolicy(Qt::CustomContextMenu);
        table_view->setItemDelegate(new ResultsTableDelegate(table_view));
        table_view->horizontalHeader()->setSectionResizeMode(QHeaderView::Interactive);
        table_view->horizontalHeader()->setStretchLastSection(true);
        table_view->horizontalHeader()->setSectionsClickable(true);
        table_view->horizontalHeader()->setSortIndicatorShown(true);
        table_view->horizontalHeader()->setSortIndicator(-1, Qt::AscendingOrder);
        table_view->verticalHeader()->setVisible(false);
        table_view->verticalHeader()->setDefaultSectionSize(28);

        auto open_files = [backend, table_view, window](bool) {
            const auto files = QFileDialog::getOpenFileNames(
                window,
                QStringLiteral("Select files to hash"));
            if (!files.isEmpty()) {
                backend->add_files(files);
                apply_table_column_width_hints(table_view);
            }
        };

        auto open_folder = [backend, table_view, window](bool) {
            const auto folder = QFileDialog::getExistingDirectory(
                window,
                QStringLiteral("Select folder to add"));
            if (!folder.isEmpty()) {
                backend->add_folder(folder);
                apply_table_column_width_hints(table_view);
            }
        };

        auto show_hash_algorithms_dialog = [backend, table_view, window](bool) {
            QDialog dialog(window);
            dialog.setWindowTitle(QStringLiteral("Settings - Hash Algorithms"));
            dialog.setModal(true);

            auto* layout = new QVBoxLayout(&dialog);
            auto* crc32 = new QCheckBox(QStringLiteral("CRC32"), &dialog);
            auto* md5 = new QCheckBox(QStringLiteral("MD5"), &dialog);
            auto* sha1 = new QCheckBox(QStringLiteral("SHA1"), &dialog);
            auto* sha256 = new QCheckBox(QStringLiteral("SHA256"), &dialog);
            auto* sha512 = new QCheckBox(QStringLiteral("SHA512"), &dialog);
            auto* buttons = new QDialogButtonBox(
                QDialogButtonBox::Ok | QDialogButtonBox::Cancel,
                &dialog);

            crc32->setChecked(backend->getSetting_crc32());
            md5->setChecked(backend->getSetting_md5());
            sha1->setChecked(backend->getSetting_sha1());
            sha256->setChecked(backend->getSetting_sha256());
            sha512->setChecked(backend->getSetting_sha512());

            layout->addWidget(crc32);
            layout->addWidget(md5);
            layout->addWidget(sha1);
            layout->addWidget(sha256);
            layout->addWidget(sha512);
            layout->addWidget(buttons);

            QObject::connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
            QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);

            if (dialog.exec() == QDialog::Accepted) {
                backend->setSetting_crc32(crc32->isChecked());
                backend->setSetting_md5(md5->isChecked());
                backend->setSetting_sha1(sha1->isChecked());
                backend->setSetting_sha256(sha256->isChecked());
                backend->setSetting_sha512(sha512->isChecked());
                backend->apply_settings();
                apply_table_column_width_hints(table_view);
            }
        };

        auto show_file_renaming_dialog = [backend, window, fixed_font](bool) {
            QDialog dialog(window);
            dialog.setWindowTitle(QStringLiteral("Settings - File Renaming"));
            dialog.setModal(true);
            dialog.resize(480, dialog.sizeHint().height());

            auto* layout = new QVBoxLayout(&dialog);
            auto* pattern_label = new QLabel(QStringLiteral("Rename pattern:"), &dialog);
            auto* pattern_edit = new QLineEdit(backend->getSetting_rename_pattern(), &dialog);
            auto* tags_label = new QLabel(QStringLiteral("Available tags:"), &dialog);
            auto* tags_layout = new QGridLayout();
            auto* example_label = new QLabel(
                QStringLiteral("Example: %FILENAME%_%CRC%.%FILEEXT%"),
                &dialog);
            auto* buttons = new QDialogButtonBox(
                QDialogButtonBox::Ok | QDialogButtonBox::Cancel,
                &dialog);

            tags_label->setStyleSheet(QStringLiteral("font-weight: bold;"));
            example_label->setStyleSheet(QStringLiteral("font-style: italic; color: palette(mid);"));

            struct RenameTagRow {
                const char* tag;
                const char* description;
            };

            const RenameTagRow rename_tags[] = {
                {"%FILENAME%", "Original filename (without extension)"},
                {"%FILEEXT%", "File extension (without dot)"},
                {"%CRC%", "CRC32 hash"},
                {"%MD5%", "MD5 hash"},
                {"%SHA1%", "SHA1 hash"},
                {"%SHA256%", "SHA256 hash"},
                {"%SHA512%", "SHA512 hash"},
            };

            for (int row = 0; row < static_cast<int>(std::size(rename_tags)); ++row) {
                auto* tag_label = new QLabel(QString::fromLatin1(rename_tags[row].tag), &dialog);
                auto* description_label = new QLabel(QString::fromLatin1(rename_tags[row].description), &dialog);
                tag_label->setFont(fixed_font);
                tags_layout->addWidget(tag_label, row, 0);
                tags_layout->addWidget(description_label, row, 1);
            }

            layout->addWidget(pattern_label);
            layout->addWidget(pattern_edit);
            layout->addWidget(tags_label);
            layout->addLayout(tags_layout);
            layout->addWidget(example_label);
            layout->addWidget(buttons);

            QObject::connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
            QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);

            if (dialog.exec() == QDialog::Accepted) {
                backend->setSetting_rename_pattern(pattern_edit->text());
                backend->apply_rename_settings();
            }
        };

        auto apply_hash_casing = [backend, table_view, uppercase_hash_action](bool) {
            backend->setSetting_hash_uppercase(uppercase_hash_action->isChecked());
            backend->apply_settings();
            apply_table_column_width_hints(table_view);
        };

        auto confirm_rename_files = [backend, window](bool) {
            QDialog dialog(window);
            dialog.setWindowTitle(QStringLiteral("Rename Files"));
            dialog.setModal(true);
            dialog.setMinimumWidth(460);

            auto* layout = new QVBoxLayout(&dialog);
            auto* description = new QLabel(
                QStringLiteral("This will permanently rename all hashed files on disk according to the current rename pattern."),
                &dialog);
            auto* preview_title = new QLabel(QStringLiteral("Preview (first file):"), &dialog);
            auto* preview_label = new QLabel(backend->get_rename_preview(), &dialog);
            auto* confirm_checkbox = new QCheckBox(
                QStringLiteral("I confirm I want to rename these files"),
                &dialog);
            auto* buttons = new QDialogButtonBox(&dialog);
            auto* rename_confirm_button = buttons->addButton(
                QStringLiteral("Rename"),
                QDialogButtonBox::AcceptRole);
            buttons->addButton(QDialogButtonBox::Cancel);

            description->setWordWrap(true);
            preview_title->setStyleSheet(QStringLiteral("font-weight: bold;"));
            preview_label->setWordWrap(true);
            preview_label->setTextInteractionFlags(Qt::TextSelectableByMouse);
            preview_title->setVisible(!preview_label->text().isEmpty());
            preview_label->setVisible(!preview_label->text().isEmpty());
            rename_confirm_button->setEnabled(false);

            layout->addWidget(description);
            layout->addWidget(preview_title);
            layout->addWidget(preview_label);
            layout->addWidget(confirm_checkbox);
            layout->addWidget(buttons);

            dialog.adjustSize();

            QObject::connect(confirm_checkbox, &QCheckBox::toggled, rename_confirm_button, &QPushButton::setEnabled);
            QObject::connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
            QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);

            if (dialog.exec() == QDialog::Accepted) {
                backend->rename_files();
            }
        };

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
            [table_view, backend, sort_state](int section) {
                auto* header = table_view->horizontalHeader();

                if (sort_state->column == section) {
                    sort_state->order = sort_state->order == Qt::AscendingOrder
                        ? Qt::DescendingOrder
                        : Qt::AscendingOrder;
                } else {
                    sort_state->column = section;
                    sort_state->order = Qt::AscendingOrder;
                }

                backend->sort_by(section, sort_state->order == Qt::AscendingOrder);
                header->setSortIndicator(sort_state->column, sort_state->order);
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

                auto* open_folder_action_menu = menu.addAction(QStringLiteral("Open Containing Folder"));
                QObject::connect(open_folder_action_menu, &QAction::triggered, &menu, [backend]() {
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

        QObject::connect(open_files_action, &QAction::triggered, window, open_files);
        QObject::connect(open_folder_action, &QAction::triggered, window, open_folder);
        QObject::connect(hash_algorithms_action, &QAction::triggered, window, show_hash_algorithms_dialog);
        QObject::connect(file_renaming_action, &QAction::triggered, window, show_file_renaming_dialog);
        QObject::connect(uppercase_hash_action, &QAction::triggered, window, apply_hash_casing);
        QObject::connect(lowercase_hash_action, &QAction::triggered, window, apply_hash_casing);
        QObject::connect(exit_action, &QAction::triggered, window, [](bool) {
            QApplication::quit();
        });

        QObject::connect(open_files_button, &QPushButton::clicked, window, open_files);
        QObject::connect(open_folder_button, &QPushButton::clicked, window, open_folder);
        QObject::connect(start_button, &QPushButton::clicked, backend, [backend](bool) {
            backend->start_hashing();
        });
        QObject::connect(cancel_button, &QPushButton::clicked, backend, [backend](bool) {
            backend->cancel_hashing();
        });
        QObject::connect(clear_button, &QPushButton::clicked, backend, [backend](bool) {
            backend->clear_list();
        });
        QObject::connect(remove_button, &QPushButton::clicked, backend, [backend](bool) {
            backend->remove_selected();
        });
        QObject::connect(remove_selected_action, &QAction::triggered, backend, [backend](bool) {
            backend->remove_selected();
        });
        QObject::connect(rename_button, &QPushButton::clicked, window, confirm_rename_files);

        auto* file_menu = window->menuBar()->addMenu(QStringLiteral("File"));
        file_menu->addAction(open_files_action);
        file_menu->addAction(open_folder_action);
        file_menu->addSeparator();
        file_menu->addAction(exit_action);

        auto* settings_menu = window->menuBar()->addMenu(QStringLiteral("Settings"));
        settings_menu->addAction(hash_algorithms_action);
        settings_menu->addMenu(hash_casing_menu);
        settings_menu->addAction(file_renaming_action);

        auto sync_widget_state = [backend,
                                  open_files_button,
                                  open_folder_button,
                                  start_button,
                                  cancel_button,
                                  clear_button,
                                  remove_button,
                                  rename_button,
                                  open_files_action,
                                  open_folder_action,
                                  remove_selected_action,
                                  hash_algorithms_action,
                                  hash_casing_menu,
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
            rename_button->setEnabled(!is_hashing && has_files);
            open_files_action->setEnabled(!is_hashing);
            open_folder_action->setEnabled(!is_hashing);
            remove_selected_action->setEnabled(!is_hashing && has_selection);
            hash_algorithms_action->setEnabled(!is_hashing);
            hash_casing_menu->setEnabled(!is_hashing);

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
        apply_table_column_width_hints(table_view);

        window->setWindowTitle(widget_window_title(backend));
        window->resize(1000, 700);
        window->setCentralWidget(central_widget);
        window->show();
    }

    void qt_set_clipboard(const char* text)
    {
        if (s_app) {
            QApplication::clipboard()->setText(QString::fromUtf8(text));
        }
    }
}
