#include <algorithm>
#include <functional>
#include <memory>
#include <vector>

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
#include <QtWidgets/QStatusBar>
#include <QtWidgets/QStyledItemDelegate>
#include <QtWidgets/QTableView>
#include <QtWidgets/QVBoxLayout>
#include <QtWidgets/QWidget>
#include <QtGui/QAction>
#include <QtGui/QActionGroup>
#include <QtGui/QClipboard>
#include <QtGui/QDragEnterEvent>
#include <QtGui/QDragMoveEvent>
#include <QtGui/QDropEvent>
#include <QtGui/QFont>
#include <QtGui/QFontDatabase>
#include <QtGui/QFontMetrics>
#include <QtGui/QIcon>
#include <QtGui/QKeySequence>
#include <QtGui/QPalette>
#include <QtCore/QEvent>
#include <QtCore/QItemSelectionModel>
#include <QtCore/QFileInfo>
#include <QtCore/QMimeData>
#include <QtCore/QObject>
#include <QtCore/QPointer>
#include <QtCore/QUrl>
#include <QtWidgets/QMessageBox>

#include "rapidchecksum/src/app_backend.cxxqt.h"

static int    s_argc    = 1;
static char   s_argv0[] = "rapidchecksum";
static char*  s_argv[]  = { s_argv0, nullptr };

static std::unique_ptr<QApplication> s_app;
static QPointer<QMainWindow> s_main_window;
static QPointer<AppBackend>  s_backend;
static QPointer<QTableView>  s_results_table;
static std::vector<QString> s_startup_sfv_paths;
static std::vector<QString> s_startup_add_paths;

static void apply_table_column_width_hints(QTableView* table_view);
static void add_files_to_results(AppBackend* backend, QTableView* table_view, const QStringList& files);

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
        const int verify_column = last_column - 1;

        option->displayAlignment = Qt::AlignLeft | Qt::AlignVCenter;

        if (index.column() >= 2 && index.column() < verify_column) {
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

class FileDropFilter final : public QObject {
public:
    FileDropFilter(AppBackend* backend, QTableView* table_view, QObject* parent = nullptr)
        : QObject(parent)
        , m_backend(backend)
        , m_table_view(table_view)
    {
    }

protected:
    bool eventFilter(QObject* watched, QEvent* event) override
    {
        switch (event->type()) {
        case QEvent::DragEnter:
            return handleDragEnter(static_cast<QDragEnterEvent*>(event));
        case QEvent::DragMove:
            return handleDragMove(static_cast<QDragMoveEvent*>(event));
        case QEvent::Drop:
            return handleDrop(static_cast<QDropEvent*>(event));
        default:
            break;
        }

        return QObject::eventFilter(watched, event);
    }

private:
    bool canAcceptDrop(const QMimeData* mime_data) const
    {
        if (!m_backend || m_backend->getIs_hashing() || !mime_data || !mime_data->hasUrls()) {
            return false;
        }

        for (const auto& url : mime_data->urls()) {
            if (!url.isLocalFile()) {
                continue;
            }

            const QFileInfo info(url.toLocalFile());
            if (info.isFile()) {
                return true;
            }
        }

        return false;
    }

    QStringList droppedFiles(const QMimeData* mime_data) const
    {
        QStringList files;
        if (!mime_data) {
            return files;
        }

        for (const auto& url : mime_data->urls()) {
            if (!url.isLocalFile()) {
                continue;
            }

            const QFileInfo info(url.toLocalFile());
            if (info.isFile()) {
                files.append(info.absoluteFilePath());
            }
        }

        return files;
    }

    bool handleDragEnter(QDragEnterEvent* event) const
    {
        if (!canAcceptDrop(event->mimeData())) {
            return false;
        }

        event->acceptProposedAction();
        return true;
    }

    bool handleDragMove(QDragMoveEvent* event) const
    {
        if (!canAcceptDrop(event->mimeData())) {
            return false;
        }

        event->acceptProposedAction();
        return true;
    }

    bool handleDrop(QDropEvent* event) const
    {
        if (!m_backend || m_backend->getIs_hashing()) {
            return false;
        }

        const auto files = droppedFiles(event->mimeData());
        if (files.isEmpty()) {
            return false;
        }

        m_backend->add_files(files);
        if (m_table_view) {
            apply_table_column_width_hints(m_table_view);
        }
        event->acceptProposedAction();
        return true;
    }

    QPointer<AppBackend> m_backend;
    QPointer<QTableView> m_table_view;
};

}

static QString widget_window_title(const AppBackend* backend)
{
    return QStringLiteral("RapidChecksum %1").arg(backend->getApp_version());
}

static constexpr qint64 BYTES_PER_MB = 1024LL * 1024;
static constexpr qint64 HASH_FILE_SIZE_WARN_THRESHOLD = 100 * BYTES_PER_MB;

/// Returns true if the file is below the size threshold, or if the user
/// explicitly confirms they want to open an unusually large file.
static bool confirm_large_hash_file(QWidget* parent, const QString& path)
{
    const qint64 size = QFileInfo(path).size();
    if (size <= HASH_FILE_SIZE_WARN_THRESHOLD) {
        return true;
    }

    const double size_mb = static_cast<double>(size) / static_cast<double>(BYTES_PER_MB);
    const QString message = QString(
        "The selected file is %1 MB, which is unusually large for a hash file.\n\n"
        "Hash files (such as .sfv) are plain text and are rarely larger than a "
        "few kilobytes. This file may not be a valid hash file, and attempting "
        "to parse it could consume a large amount of memory.\n\n"
        "Do you want to open it anyway?"
    ).arg(size_mb, 0, 'f', 1);

    return QMessageBox::warning(
        parent,
        QStringLiteral("Large File Warning"),
        message,
        QMessageBox::Yes | QMessageBox::No,
        QMessageBox::No
    ) == QMessageBox::Yes;
}

static int progress_value(float progress)
{
    auto scaled = static_cast<int>(progress * 1000.0f);
    return std::clamp(scaled, 0, 1000);
}

static QString representative_hash_text(int hex_length)
{
    if (hex_length <= 0) {
        return QString();
    }

    return QString(hex_length, QLatin1Char('0'));
}

static int padded_text_width(const QFontMetrics& metrics, const QString& text)
{
    return metrics.horizontalAdvance(text) + 18;
}

static int visible_header_section_count(QHeaderView* header)
{
    if (!header) {
        return 0;
    }

    int visible_count = 0;
    for (int section = 0; section < header->count(); ++section) {
        if (!header->isSectionHidden(section)) {
            ++visible_count;
        }
    }
    return visible_count;
}

static QString file_count_text(int file_count)
{
    return QStringLiteral("%1 file%2").arg(file_count).arg(file_count == 1 ? QString() : QStringLiteral("s"));
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

    auto* backend = qobject_cast<AppBackend*>(model);
    if (!backend) {
        return;
    }

    const int last_column = column_count - 1;
    const int verify_column = last_column - 1;
    const QFont fixed_font = QFontDatabase::systemFont(QFontDatabase::FixedFont);
    const QFontMetrics fixed_metrics(fixed_font);
    const QFontMetrics default_metrics(table_view->font());

    const QString filepath_header = model->headerData(0, Qt::Horizontal, Qt::DisplayRole).toString();
    const QString filename_header = model->headerData(1, Qt::Horizontal, Qt::DisplayRole).toString();
    const int filepath_width = std::max(padded_text_width(default_metrics, filepath_header), 320);
    const int filename_width = std::max(padded_text_width(default_metrics, filename_header), 220);
    table_view->setColumnWidth(0, std::max(table_view->columnWidth(0), filepath_width));
    table_view->setColumnWidth(1, std::max(table_view->columnWidth(1), filename_width));

    for (int column = 2; column < verify_column; ++column) {
        const QString header = model->headerData(column, Qt::Horizontal, Qt::DisplayRole).toString();
        int desired_width = padded_text_width(fixed_metrics, header);

        const QString sample = representative_hash_text(backend->hash_hex_length_for_column(column));
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

static void add_files_to_results(AppBackend* backend, QTableView* table_view, const QStringList& files)
{
    if (!backend || files.isEmpty()) {
        return;
    }

    backend->add_files(files);
    if (table_view) {
        apply_table_column_width_hints(table_view);
    }
}

static void persist_hidden_column_settings(AppBackend* backend, QTableView* table_view)
{
    auto* header = table_view ? table_view->horizontalHeader() : nullptr;
    if (!backend || !header) {
        return;
    }

    QStringList hidden_keys;
    for (int section = 0; section < header->count(); ++section) {
        if (!header->isSectionHidden(section)) {
            continue;
        }

        const QString key = backend->column_visibility_key(section);
        if (!key.isEmpty()) {
            hidden_keys.append(key);
        }
    }

    backend->set_hidden_column_keys(hidden_keys);
}

static void apply_hidden_column_settings(AppBackend* backend, QTableView* table_view)
{
    auto* header = table_view ? table_view->horizontalHeader() : nullptr;
    if (!backend || !header) {
        return;
    }

    const QStringList hidden_keys = backend->hidden_column_keys();
    for (int section = 0; section < header->count(); ++section) {
        const QString key = backend->column_visibility_key(section);
        header->setSectionHidden(section, !key.isEmpty() && hidden_keys.contains(key));
    }

    if (visible_header_section_count(header) == 0 && header->count() > 0) {
        const int fallback_section = header->count() > 1 ? 1 : 0;
        header->setSectionHidden(fallback_section, false);
        persist_hidden_column_settings(backend, table_view);
    }

    apply_table_column_width_hints(table_view);
}

extern "C" {
    void qt_app_init()
    {
        if (!s_app) {
            s_app = std::make_unique<QApplication>(s_argc, s_argv);
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
        auto* window = s_main_window.data();
        auto* central_widget = new QWidget(window);
        auto* main_layout = new QVBoxLayout(central_widget);
        auto* toolbar_layout = new QHBoxLayout();
        auto* start_button = new QPushButton(QStringLiteral("Start Hashing"));
        auto* cancel_button = new QPushButton(QStringLiteral("Cancel"));
        auto* clear_button = new QPushButton(QStringLiteral("Clear List"));
        auto* remove_button = new QPushButton(QStringLiteral("Remove Selected"));
        auto* rename_button = new QPushButton(QStringLiteral("Rename Files"));
        auto* file_progress = new QProgressBar();
        auto* global_progress = new QProgressBar();
        auto* status_label = new QLabel();
        auto* file_count_label = new QLabel();
        auto* table_view = new QTableView();
        auto* backend = new AppBackend(central_widget);
        auto* file_drop_filter = new FileDropFilter(backend, table_view, window);
        auto* open_files_action = new QAction(QStringLiteral("Open Files..."), window);
        auto* open_hash_file_action = new QAction(QStringLiteral("Open Hash File..."), window);
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
        open_hash_file_action->setShortcut(QKeySequence(QStringLiteral("Ctrl+Shift+O")));
        open_folder_action->setShortcut(QKeySequence(QStringLiteral("Ctrl+L")));
        remove_selected_action->setShortcut(QKeySequence(QStringLiteral("Delete")));
        exit_action->setShortcut(QKeySequence(QStringLiteral("Ctrl+Q")));

        window->addAction(open_files_action);
        window->addAction(open_hash_file_action);
        window->addAction(open_folder_action);
        window->addAction(remove_selected_action);
        window->addAction(exit_action);

        main_layout->setContentsMargins(6, 6, 6, 6);
        main_layout->setSpacing(4);

        toolbar_layout->setSpacing(4);
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
        table_view->setSelectionMode(QAbstractItemView::ExtendedSelection);
        table_view->setAlternatingRowColors(true);
        table_view->setContextMenuPolicy(Qt::CustomContextMenu);
        table_view->setItemDelegate(new ResultsTableDelegate(table_view));
        table_view->horizontalHeader()->setSectionResizeMode(QHeaderView::Interactive);
        table_view->horizontalHeader()->setStretchLastSection(true);
        table_view->horizontalHeader()->setSectionsClickable(true);
        table_view->horizontalHeader()->setContextMenuPolicy(Qt::CustomContextMenu);
        table_view->horizontalHeader()->setSortIndicatorShown(true);
        table_view->horizontalHeader()->setSortIndicator(-1, Qt::AscendingOrder);
        table_view->verticalHeader()->setVisible(false);
        table_view->verticalHeader()->setDefaultSectionSize(28);
        window->setAcceptDrops(true);
        central_widget->setAcceptDrops(true);
        table_view->setAcceptDrops(true);
        table_view->viewport()->setAcceptDrops(true);
        window->installEventFilter(file_drop_filter);
        central_widget->installEventFilter(file_drop_filter);
        table_view->installEventFilter(file_drop_filter);
        table_view->viewport()->installEventFilter(file_drop_filter);
        auto* selection_model = table_view->selectionModel();
        auto sync_action_state = std::make_shared<std::function<void()>>();
        const QPointer<AppBackend> backend_ptr(backend);
        const QPointer<QProgressBar> file_progress_ptr(file_progress);
        const QPointer<QProgressBar> global_progress_ptr(global_progress);
        const QPointer<QLabel> status_label_ptr(status_label);
        const QPointer<QLabel> file_count_label_ptr(file_count_label);
        const QPointer<QItemSelectionModel> selection_model_ptr(selection_model);
        const QPointer<QPushButton> start_button_ptr(start_button);
        const QPointer<QPushButton> cancel_button_ptr(cancel_button);
        const QPointer<QPushButton> clear_button_ptr(clear_button);
        const QPointer<QPushButton> remove_button_ptr(remove_button);
        const QPointer<QPushButton> rename_button_ptr(rename_button);
        const QPointer<QAction> open_files_action_ptr(open_files_action);
        const QPointer<QAction> open_hash_file_action_ptr(open_hash_file_action);
        const QPointer<QAction> open_folder_action_ptr(open_folder_action);
        const QPointer<QAction> remove_selected_action_ptr(remove_selected_action);
        const QPointer<QAction> hash_algorithms_action_ptr(hash_algorithms_action);
        const QPointer<QMenu> hash_casing_menu_ptr(hash_casing_menu);
        const QPointer<QAction> file_renaming_action_ptr(file_renaming_action);

        auto sync_progress_state = [backend_ptr,
                                    file_progress_ptr,
                                    global_progress_ptr,
                                    status_label_ptr,
                                    file_count_label_ptr]() {
            if (!backend_ptr || !file_progress_ptr || !global_progress_ptr || !status_label_ptr || !file_count_label_ptr) {
                return;
            }

            const bool is_hashing = backend_ptr->getIs_hashing();

            file_progress_ptr->setVisible(is_hashing);
            global_progress_ptr->setVisible(is_hashing);
            file_progress_ptr->setValue(progress_value(backend_ptr->getFile_progress()));
            global_progress_ptr->setValue(progress_value(backend_ptr->getGlobal_progress()));
            status_label_ptr->setText(backend_ptr->getStatus_text());
            file_count_label_ptr->setText(file_count_text(backend_ptr->getFile_count()));
        };

        auto open_files = [backend, table_view, window](bool) {
            const auto files = QFileDialog::getOpenFileNames(
                window,
                QStringLiteral("Select files to hash"));
            add_files_to_results(backend, table_view, files);
        };

        auto open_hash_file = [backend, table_view, window](bool) {
            const auto path = QFileDialog::getOpenFileName(
                window,
                QStringLiteral("Open SFV file"),
                QString(),
                QStringLiteral("SFV Files (*.sfv);;All Files (*)"));
            if (!path.isEmpty() && confirm_large_hash_file(window, path)) {
                backend->load_hash_file(path);
                apply_table_column_width_hints(table_view);
            }
        };

        auto save_hash_file = [backend, table_view](const QString& algorithm_id) {
            const auto path = QFileDialog::getSaveFileName(
                table_view,
                QStringLiteral("Save hash file"),
                QString(),
                algorithm_id == QStringLiteral("crc32")
                    ? QStringLiteral("SFV Files (*.sfv);;All Files (*)")
                    : QStringLiteral("All Files (*)"));
            if (!path.isEmpty()) {
                QString final_path = path;
                if (algorithm_id == QStringLiteral("crc32") && QFileInfo(final_path).suffix().isEmpty()) {
                    final_path += QStringLiteral(".sfv");
                }
                backend->save_hash_file(algorithm_id, final_path);
            }
        };

        auto populate_save_hash_menu = [backend, save_hash_file](QMenu* menu) {
            menu->clear();
            const auto algorithm_ids = backend->all_hash_algorithm_ids();
            for (int idx = 0; idx < algorithm_ids.size(); ++idx) {
                const QString algorithm_id = algorithm_ids.at(idx);
                if (!backend->is_hash_algorithm_enabled(algorithm_id)) {
                    continue;
                }

                auto* action = menu->addAction(backend->hash_algorithm_save_label(algorithm_id));
                QObject::connect(action, &QAction::triggered, menu, [save_hash_file, algorithm_id]() {
                    save_hash_file(algorithm_id);
                });
            }
            menu->setEnabled(!menu->actions().isEmpty());
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
            auto* algorithms_layout = new QGridLayout();
            auto* buttons = new QDialogButtonBox(
                QDialogButtonBox::Ok | QDialogButtonBox::Cancel,
                &dialog);
            struct AlgorithmCheckbox {
                QString id;
                QCheckBox* checkbox;
            };
            std::vector<AlgorithmCheckbox> algorithm_checkboxes;
            const auto algorithm_ids = backend->all_hash_algorithm_ids();
            const int column_count = algorithm_ids.size() > 8 ? 2 : 1;

            for (int index = 0; index < algorithm_ids.size(); ++index) {
                const QString algorithm_id = algorithm_ids.at(index);
                auto* checkbox = new QCheckBox(backend->hash_algorithm_name(algorithm_id), &dialog);
                checkbox->setChecked(backend->is_hash_algorithm_enabled(algorithm_id));
                algorithms_layout->addWidget(checkbox, index / column_count, index % column_count);
                algorithm_checkboxes.push_back({algorithm_id, checkbox});
            }

            layout->addLayout(algorithms_layout);
            layout->addWidget(buttons);

            QObject::connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
            QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);

            if (dialog.exec() == QDialog::Accepted) {
                for (const auto& entry : algorithm_checkboxes) {
                    backend->set_hash_algorithm_enabled(entry.id, entry.checkbox->isChecked());
                }
                backend->apply_settings();
                apply_table_column_width_hints(table_view);
            }
        };

        auto show_file_renaming_dialog = [backend, window, fixed_font, sync_action_state](bool) {
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

            const RenameTagRow base_tags[] = {
                {"%FILENAME%", "Original filename (without extension)"},
                {"%FILEEXT%", "File extension (without dot)"},
            };

            int row = 0;
            for (const auto& tag : base_tags) {
                auto* tag_label = new QLabel(QString::fromLatin1(tag.tag), &dialog);
                auto* description_label = new QLabel(QString::fromLatin1(tag.description), &dialog);
                tag_label->setFont(fixed_font);
                tags_layout->addWidget(tag_label, row, 0);
                tags_layout->addWidget(description_label, row, 1);
                ++row;
            }

            const auto algorithm_ids = backend->all_hash_algorithm_ids();
            for (int index = 0; index < algorithm_ids.size(); ++index) {
                const QString algorithm_id = algorithm_ids.at(index);
                auto* tag_label = new QLabel(backend->hash_algorithm_tag(algorithm_id), &dialog);
                auto* description_label = new QLabel(
                    backend->hash_algorithm_name(algorithm_id) + QStringLiteral(" hash"),
                    &dialog);
                tag_label->setFont(fixed_font);
                tags_layout->addWidget(tag_label, row, 0);
                tags_layout->addWidget(description_label, row, 1);
                ++row;
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
                if (*sync_action_state) {
                    (*sync_action_state)();
                }
            }
        };

        auto apply_hash_casing = [backend, table_view, uppercase_hash_action](bool) {
            backend->setSetting_hash_uppercase(uppercase_hash_action->isChecked());
            backend->apply_settings();
            apply_table_column_width_hints(table_view);
        };

        auto confirm_rename_files = [backend, window, sync_action_state](bool) {
            const QString preview = backend->get_rename_preview();
            if (preview.isEmpty()) {
                QMessageBox::information(
                    window,
                    QStringLiteral("Rename Files"),
                    QStringLiteral("No hashed files are currently eligible to be renamed."));
                if (*sync_action_state) {
                    (*sync_action_state)();
                }
                return;
            }

            QDialog dialog(window);
            dialog.setWindowTitle(QStringLiteral("Rename Files"));
            dialog.setModal(true);
            dialog.setMinimumWidth(460);

            auto* layout = new QVBoxLayout(&dialog);
            auto* description = new QLabel(
                QStringLiteral("This will permanently rename all hashed files on disk according to the current rename pattern."),
                &dialog);
            auto* preview_title = new QLabel(QStringLiteral("Preview (first file):"), &dialog);
            auto* preview_label = new QLabel(preview, &dialog);
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
                if (*sync_action_state) {
                    (*sync_action_state)();
                }
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
            table_view->horizontalHeader(),
            &QHeaderView::customContextMenuRequested,
            table_view,
            [table_view, backend](const QPoint& pos) {
                auto* header = table_view->horizontalHeader();
                auto* model = table_view->model();
                if (!header || !model) {
                    return;
                }

                QMenu menu(header);
                const int column_count = model->columnCount(QModelIndex());

                auto* reset_columns_action = menu.addAction(QStringLiteral("Reset Columns"));
                reset_columns_action->setEnabled(visible_header_section_count(header) < column_count);
                QObject::connect(reset_columns_action, &QAction::triggered, &menu, [table_view, header, backend]() {
                    for (int section = 0; section < header->count(); ++section) {
                        header->setSectionHidden(section, false);
                    }
                    persist_hidden_column_settings(backend, table_view);
                    apply_table_column_width_hints(table_view);
                });
                menu.addSeparator();

                for (int section = 0; section < column_count; ++section) {
                    const QString label = model->headerData(section, Qt::Horizontal, Qt::DisplayRole).toString();
                    if (label.isEmpty()) {
                        continue;
                    }

                    auto* action = menu.addAction(label);
                    action->setCheckable(true);
                    action->setChecked(!header->isSectionHidden(section));

                    QObject::connect(action, &QAction::triggered, &menu, [table_view, header, backend, action, section](bool checked) {
                        const int visible_count = visible_header_section_count(header);
                        if (!checked && !header->isSectionHidden(section) && visible_count <= 1) {
                            action->setChecked(true);
                            return;
                        }

                        header->setSectionHidden(section, !checked);
                        persist_hidden_column_settings(backend, table_view);
                        if (checked) {
                            apply_table_column_width_hints(table_view);
                        }
                    });
                }

                menu.exec(header->viewport()->mapToGlobal(pos));
            });

        QObject::connect(
            table_view,
            &QTableView::customContextMenuRequested,
            table_view,
            [table_view, backend, selection_model, populate_save_hash_menu](const QPoint& pos) {
                const QModelIndex index = table_view->indexAt(pos);
                if (!index.isValid()) {
                    return;
                }

                if (!selection_model->isSelected(index)) {
                    selection_model->setCurrentIndex(
                        index,
                        QItemSelectionModel::ClearAndSelect | QItemSelectionModel::Rows);
                } else {
                    selection_model->setCurrentIndex(index, QItemSelectionModel::NoUpdate);
                }

                QMenu menu(table_view);
                auto* copy_file_path_action = menu.addAction(QStringLiteral("Copy File Path"));
                QObject::connect(copy_file_path_action, &QAction::triggered, &menu, [table_view]() {
                    auto* backend = qobject_cast<AppBackend*>(table_view->model());
                    if (!backend) {
                        return;
                    }
                    backend->copy_filepath();
                });

                auto* copy_hash_menu = menu.addMenu(QStringLiteral("Copy Hash"));
                const auto algorithm_ids = backend->all_hash_algorithm_ids();
                for (int idx = 0; idx < algorithm_ids.size(); ++idx) {
                    const QString algorithm_id = algorithm_ids.at(idx);
                    if (!backend->is_hash_algorithm_enabled(algorithm_id)) {
                        continue;
                    }

                    auto* action = copy_hash_menu->addAction(backend->hash_algorithm_name(algorithm_id));
                    QObject::connect(action, &QAction::triggered, &menu, [table_view, algorithm_id]() {
                        auto* backend = qobject_cast<AppBackend*>(table_view->model());
                        if (!backend) {
                            return;
                        }
                        backend->copy_hash(algorithm_id);
                    });
                }

                auto* open_folder_action_menu = menu.addAction(QStringLiteral("Open Containing Folder"));
                QObject::connect(open_folder_action_menu, &QAction::triggered, &menu, [table_view]() {
                    auto* backend = qobject_cast<AppBackend*>(table_view->model());
                    if (!backend) {
                        return;
                    }
                    backend->open_folder();
                });

                menu.addSeparator();
                auto* save_hash_menu = menu.addMenu(QStringLiteral("Save Hash File"));
                populate_save_hash_menu(save_hash_menu);

                menu.exec(table_view->viewport()->mapToGlobal(pos));
            });

        QObject::connect(open_files_action, &QAction::triggered, window, open_files);
        QObject::connect(open_hash_file_action, &QAction::triggered, window, open_hash_file);
        QObject::connect(open_folder_action, &QAction::triggered, window, open_folder);
        QObject::connect(hash_algorithms_action, &QAction::triggered, window, show_hash_algorithms_dialog);
        QObject::connect(file_renaming_action, &QAction::triggered, window, show_file_renaming_dialog);
        QObject::connect(uppercase_hash_action, &QAction::triggered, window, apply_hash_casing);
        QObject::connect(lowercase_hash_action, &QAction::triggered, window, apply_hash_casing);
        QObject::connect(exit_action, &QAction::triggered, window, [](bool) {
            QApplication::quit();
        });

        QObject::connect(start_button, &QPushButton::clicked, backend, [backend](bool) {
            backend->start_hashing();
        });
        QObject::connect(cancel_button, &QPushButton::clicked, backend, [backend](bool) {
            backend->cancel_hashing();
        });
        QObject::connect(clear_button, &QPushButton::clicked, backend, [backend](bool) {
            backend->clear_list();
        });
        auto remove_selected_rows = [backend, table_view](bool) {
            const auto selected = table_view->selectionModel()->selectedRows();
            std::vector<int> rows;
            rows.reserve(static_cast<size_t>(selected.size()));
            for (const auto& idx : selected) {
                rows.push_back(idx.row());
            }
            std::sort(rows.begin(), rows.end(), std::greater<int>());
            for (int row : rows) {
                backend->remove_row_at(row);
            }
        };
        QObject::connect(remove_button, &QPushButton::clicked, backend, remove_selected_rows);
        QObject::connect(remove_selected_action, &QAction::triggered, backend, remove_selected_rows);
        QObject::connect(rename_button, &QPushButton::clicked, window, confirm_rename_files);

        auto* file_menu = window->menuBar()->addMenu(QStringLiteral("File"));
        auto* save_hash_menu_bar = new QMenu(QStringLiteral("Save Hash File"), file_menu);
        file_menu->addAction(open_files_action);
        file_menu->addAction(open_hash_file_action);
        file_menu->addAction(open_folder_action);
        file_menu->addMenu(save_hash_menu_bar);
        file_menu->addSeparator();
        file_menu->addAction(exit_action);
        QObject::connect(save_hash_menu_bar, &QMenu::aboutToShow, window, [populate_save_hash_menu, save_hash_menu_bar]() {
            populate_save_hash_menu(save_hash_menu_bar);
        });

        auto* settings_menu = window->menuBar()->addMenu(QStringLiteral("Settings"));
        settings_menu->addAction(hash_algorithms_action);
        settings_menu->addMenu(hash_casing_menu);
        settings_menu->addAction(file_renaming_action);

        *sync_action_state = [backend_ptr,
                              selection_model_ptr,
                              start_button_ptr,
                              cancel_button_ptr,
                              clear_button_ptr,
                              remove_button_ptr,
                              rename_button_ptr,
                              open_files_action_ptr,
                              open_hash_file_action_ptr,
                              open_folder_action_ptr,
                              save_hash_menu_bar = QPointer<QMenu>(save_hash_menu_bar),
                              remove_selected_action_ptr,
                              hash_algorithms_action_ptr,
                              hash_casing_menu_ptr,
                              file_renaming_action_ptr]() {
            if (!backend_ptr || !selection_model_ptr || !start_button_ptr || !cancel_button_ptr || !clear_button_ptr
                || !remove_button_ptr || !rename_button_ptr || !open_files_action_ptr || !open_hash_file_action_ptr
                || !open_folder_action_ptr || !save_hash_menu_bar || !remove_selected_action_ptr
                || !hash_algorithms_action_ptr || !hash_casing_menu_ptr || !file_renaming_action_ptr) {
                return;
            }

            const bool is_hashing = backend_ptr->getIs_hashing();
            const bool has_files = backend_ptr->getFile_count() > 0;
            const bool has_selected_rows = !selection_model_ptr->selectedRows().isEmpty();
            const bool has_rename_candidates = has_files && !backend_ptr->get_rename_preview().isEmpty();

            start_button_ptr->setEnabled(!is_hashing && has_files);
            cancel_button_ptr->setEnabled(is_hashing);
            clear_button_ptr->setEnabled(!is_hashing && has_files);
            remove_button_ptr->setEnabled(!is_hashing && has_selected_rows);
            rename_button_ptr->setEnabled(!is_hashing && has_rename_candidates);
            open_files_action_ptr->setEnabled(!is_hashing);
            open_hash_file_action_ptr->setEnabled(!is_hashing);
            open_folder_action_ptr->setEnabled(!is_hashing);
            save_hash_menu_bar->setEnabled(!is_hashing && has_files);
            remove_selected_action_ptr->setEnabled(!is_hashing && has_selected_rows);
            hash_algorithms_action_ptr->setEnabled(!is_hashing);
            hash_casing_menu_ptr->setEnabled(!is_hashing);
            file_renaming_action_ptr->setEnabled(!is_hashing);
        };

        QObject::connect(backend, &AppBackend::is_hashingChanged, central_widget, [sync_action_state]() {
            if (*sync_action_state) {
                (*sync_action_state)();
            }
        });
        QObject::connect(backend, &AppBackend::file_countChanged, central_widget, [sync_action_state]() {
            if (*sync_action_state) {
                (*sync_action_state)();
            }
        });
        QObject::connect(selection_model, &QItemSelectionModel::selectionChanged, central_widget, [sync_action_state](const auto&, const auto&) {
            if (*sync_action_state) {
                (*sync_action_state)();
            }
        });
        QObject::connect(backend, &QAbstractItemModel::dataChanged, central_widget, [sync_action_state](const auto&, const auto&, const auto&) {
            if (*sync_action_state) {
                (*sync_action_state)();
            }
        });
        QObject::connect(backend, &QAbstractItemModel::modelReset, central_widget, [table_view, backend, sync_action_state]() {
            apply_hidden_column_settings(backend, table_view);
            if (*sync_action_state) {
                (*sync_action_state)();
            }
        });

        QObject::connect(backend, &AppBackend::is_hashingChanged, central_widget, sync_progress_state);
        QObject::connect(backend, &AppBackend::file_progressChanged, central_widget, sync_progress_state);
        QObject::connect(backend, &AppBackend::global_progressChanged, central_widget, sync_progress_state);
        QObject::connect(backend, &AppBackend::status_textChanged, central_widget, sync_progress_state);
        QObject::connect(backend, &AppBackend::file_countChanged, central_widget, sync_progress_state);

        main_layout->addLayout(toolbar_layout);
        main_layout->addWidget(file_progress);
        main_layout->addWidget(global_progress);
        main_layout->addWidget(table_view, 1);

        window->statusBar()->addWidget(status_label, 1);
        window->statusBar()->addPermanentWidget(file_count_label);

        if (*sync_action_state) {
            (*sync_action_state)();
        }
        sync_progress_state();
        apply_hidden_column_settings(backend, table_view);

    s_backend = backend;
    s_results_table = table_view;

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

    void qt_queue_startup_sfv(const char* path)
    {
        if (!path) {
            return;
        }

        s_startup_sfv_paths.push_back(QString::fromUtf8(path));
    }

    void qt_process_startup_sfv()
    {
        if (!s_backend) {
            return;
        }

        for (const auto& path : s_startup_sfv_paths) {
            if (confirm_large_hash_file(s_main_window, path)) {
                s_backend->load_hash_file(path);
            }
        }
        s_startup_sfv_paths.clear();

        if (s_results_table) {
            apply_table_column_width_hints(s_results_table);
        }
    }

    void qt_queue_startup_add(const char* path)
    {
        if (!path) {
            return;
        }

        s_startup_add_paths.push_back(QString::fromUtf8(path));
    }

    void qt_process_startup_add()
    {
        if (!s_backend || s_startup_add_paths.empty()) {
            return;
        }

        QStringList paths;
        paths.reserve(static_cast<qsizetype>(s_startup_add_paths.size()));
        for (const auto& path : s_startup_add_paths) {
            paths.append(path);
        }
        add_files_to_results(s_backend, s_results_table, paths);
        s_startup_add_paths.clear();
    }
}
