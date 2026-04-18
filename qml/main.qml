import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import Qt.labs.platform 1.1 as Platform
import com.rapidchecksum.app 1.0

ApplicationWindow {
    id: root
    title: "RapidChecksum"
    width: 1000
    height: 700
    visible: true

    // ─── Menu bar ─────────────────────────────────────────────────────────
    menuBar: MenuBar {
        Menu {
            title: "File"
            MenuItem {
                text: "Open Files…"
                enabled: !AppBackend.is_hashing
                onTriggered: openFilesDialog.open()
            }
            MenuItem {
                text: "Open Folder…"
                enabled: !AppBackend.is_hashing
                onTriggered: openFolderDialog.open()
            }
            MenuSeparator {}
            MenuItem {
                text: "Exit"
                onTriggered: Qt.quit()
            }
        }
        Menu {
            title: "Settings"
            MenuItem {
                text: "Hash Algorithms…"
                onTriggered: {
                    chkCrc32.checked  = AppBackend.setting_crc32
                    chkMd5.checked    = AppBackend.setting_md5
                    chkSha1.checked   = AppBackend.setting_sha1
                    chkSha256.checked = AppBackend.setting_sha256
                    chkSha512.checked = AppBackend.setting_sha512
                    hashAlgorithmsDialog.open()
                }
            }
            MenuItem {
                text: "File Renaming…"
                onTriggered: {
                    renamePatternField.text = AppBackend.setting_rename_pattern
                    fileRenamingDialog.open()
                }
            }
        }
    }

    Shortcut { sequence: "Ctrl+O"; enabled: !AppBackend.is_hashing; onActivated: openFilesDialog.open() }
    Shortcut { sequence: "Ctrl+L"; enabled: !AppBackend.is_hashing; onActivated: openFolderDialog.open() }
    Shortcut { sequence: "Delete"; enabled: !AppBackend.is_hashing && AppBackend.selected_row >= 0; onActivated: AppBackend.remove_selected() }
    Shortcut { sequence: "Ctrl+Q"; onActivated: Qt.quit() }

    // ─── File dialogs ─────────────────────────────────────────────────────
    Platform.FileDialog {
        id: openFilesDialog
        title: "Select files to hash"
        fileMode: Platform.FileDialog.OpenFiles
        onAccepted: {
            var paths = []
            for (var i = 0; i < files.length; i++) {
                var p = decodeURIComponent(files[i].toString())
                if (p.startsWith("file://")) p = p.substring(7)
                paths.push(p)
            }
            AppBackend.add_files(paths)
        }
    }

    Platform.FolderDialog {
        id: openFolderDialog
        title: "Select folder to add"
        onAccepted: {
            var p = decodeURIComponent(folder.toString())
            if (p.startsWith("file://")) p = p.substring(7)
            AppBackend.add_folder(p)
        }
    }

    Platform.FileDialog {
        id: saveHashDialog
        title: "Save hash file"
        fileMode: Platform.FileDialog.SaveFile
        property int hashAlgo: 0
        onAccepted: {
            var p = decodeURIComponent(file.toString())
            if (p.startsWith("file://")) p = p.substring(7)
            AppBackend.save_hash_file(hashAlgo, p)
        }
    }

    // ─── Hash Algorithms dialog ───────────────────────────────────────────
    Dialog {
        id: hashAlgorithmsDialog
        title: "Settings — Hash Algorithms"
        modal: true
        anchors.centerIn: parent
        standardButtons: Dialog.Ok | Dialog.Cancel

        ColumnLayout {
            spacing: 8
            CheckBox {
                id: chkCrc32
                text: "CRC32"
                checked: AppBackend.setting_crc32
            }
            CheckBox {
                id: chkMd5
                text: "MD5"
                checked: AppBackend.setting_md5
            }
            CheckBox {
                id: chkSha1
                text: "SHA1"
                checked: AppBackend.setting_sha1
            }
            CheckBox {
                id: chkSha256
                text: "SHA256"
                checked: AppBackend.setting_sha256
            }
            CheckBox {
                id: chkSha512
                text: "SHA512"
                checked: AppBackend.setting_sha512
            }
        }

        onAccepted: {
            AppBackend.setting_crc32  = chkCrc32.checked
            AppBackend.setting_md5    = chkMd5.checked
            AppBackend.setting_sha1   = chkSha1.checked
            AppBackend.setting_sha256 = chkSha256.checked
            AppBackend.setting_sha512 = chkSha512.checked
            AppBackend.apply_settings()
            fileListItem.refreshHeaders()
        }
    }

    // ─── Context menu ─────────────────────────────────────────────────────
    Menu {
        id: contextMenu
        MenuItem {
            text: "Copy File Path"
            onTriggered: AppBackend.copy_filepath()
        }
        Menu {
            title: "Copy Hash"
            MenuItem { text: "CRC32";  visible: AppBackend.setting_crc32;  onTriggered: AppBackend.copy_hash(0) }
            MenuItem { text: "MD5";    visible: AppBackend.setting_md5;    onTriggered: AppBackend.copy_hash(1) }
            MenuItem { text: "SHA1";   visible: AppBackend.setting_sha1;   onTriggered: AppBackend.copy_hash(2) }
            MenuItem { text: "SHA256"; visible: AppBackend.setting_sha256; onTriggered: AppBackend.copy_hash(3) }
            MenuItem { text: "SHA512"; visible: AppBackend.setting_sha512; onTriggered: AppBackend.copy_hash(4) }
        }
        MenuItem {
            text: "Open Containing Folder"
            onTriggered: AppBackend.open_folder()
        }
        MenuSeparator {}
        Menu {
            title: "Save Hash File"
            MenuItem {
                text: "CRC32 / SFV"
                visible: AppBackend.setting_crc32
                onTriggered: { saveHashDialog.hashAlgo = 0; saveHashDialog.open() }
            }
            MenuItem {
                text: "MD5"
                visible: AppBackend.setting_md5
                onTriggered: { saveHashDialog.hashAlgo = 1; saveHashDialog.open() }
            }
            MenuItem {
                text: "SHA1"
                visible: AppBackend.setting_sha1
                onTriggered: { saveHashDialog.hashAlgo = 2; saveHashDialog.open() }
            }
            MenuItem {
                text: "SHA256"
                visible: AppBackend.setting_sha256
                onTriggered: { saveHashDialog.hashAlgo = 3; saveHashDialog.open() }
            }
            MenuItem {
                text: "SHA512"
                visible: AppBackend.setting_sha512
                onTriggered: { saveHashDialog.hashAlgo = 4; saveHashDialog.open() }
            }
        }
    }

    // ─── File Renaming dialog ─────────────────────────────────────────────
    Dialog {
        id: fileRenamingDialog
        title: "Settings — File Renaming"
        modal: true
        anchors.centerIn: parent
        implicitWidth: 480
        standardButtons: Dialog.Ok | Dialog.Cancel

        ColumnLayout {
            spacing: 8
            anchors.left: parent.left
            anchors.right: parent.right

            Label { text: "Rename pattern:" }
            TextField {
                id: renamePatternField
                Layout.fillWidth: true
                placeholderText: "%FILENAME%.%FILEEXT%"
            }

            Label { text: "Available tags:"; font.bold: true }

            GridLayout {
                columns: 2
                columnSpacing: 12
                rowSpacing: 3
                Layout.fillWidth: true

                Label { text: "%FILENAME%"; font.family: "monospace" }
                Label { text: "Original filename (without extension)" }

                Label { text: "%FILEEXT%"; font.family: "monospace" }
                Label { text: "File extension (without dot)" }

                Label { text: "%CRC%"; font.family: "monospace" }
                Label { text: "CRC32 hash" }

                Label { text: "%MD5%"; font.family: "monospace" }
                Label { text: "MD5 hash" }

                Label { text: "%SHA1%"; font.family: "monospace" }
                Label { text: "SHA1 hash" }

                Label { text: "%SHA256%"; font.family: "monospace" }
                Label { text: "SHA256 hash" }

                Label { text: "%SHA512%"; font.family: "monospace" }
                Label { text: "SHA512 hash" }
            }

            Label {
                text: "Example: %FILENAME%_%CRC%.%FILEEXT%"
                font.italic: true
                color: palette.mid
            }
        }

        onAccepted: {
            AppBackend.setting_rename_pattern = renamePatternField.text
            AppBackend.apply_rename_settings()
        }
    }

    // ─── Main layout ──────────────────────────────────────────────────────
    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 6
        spacing: 4

        // Toolbar
        RowLayout {
            spacing: 4
            Button {
                text: "Start Hashing"
                enabled: !AppBackend.is_hashing && AppBackend.file_count > 0
                onClicked: AppBackend.start_hashing()
            }
            Button {
                text: "Cancel"
                enabled: AppBackend.is_hashing
                onClicked: AppBackend.cancel_hashing()
            }
            Button {
                text: "Clear List"
                enabled: !AppBackend.is_hashing && AppBackend.file_count > 0
                onClicked: AppBackend.clear_list()
            }
            Button {
                text: "Remove Selected"
                enabled: !AppBackend.is_hashing && AppBackend.selected_row >= 0
                onClicked: AppBackend.remove_selected()
            }
            Button {
                text: "Rename Files"
                enabled: !AppBackend.is_hashing && AppBackend.file_count > 0
                onClicked: AppBackend.rename_files()
            }
            Item { Layout.fillWidth: true }
        }

        // Progress bars
        ProgressBar {
            Layout.fillWidth: true
            value: AppBackend.file_progress
            visible: AppBackend.is_hashing
        }
        ProgressBar {
            Layout.fillWidth: true
            value: AppBackend.global_progress
            visible: AppBackend.is_hashing
        }

        // Status bar
        Label {
            text: AppBackend.status_text
            Layout.fillWidth: true
            elide: Text.ElideRight
        }

        // ── File list ────────────────────────────────────────────────────
        Item {
            id: fileListItem
            Layout.fillWidth: true
            Layout.fillHeight: true

            // Populate header model from visible columns
            function refreshHeaders() {
                headerModel.clear()
                var idx = 0
                headerModel.append({ "colName": "Filename", "colIdx": idx++ })
                var vc = AppBackend.visible_columns()
                for (var i = 0; i < vc.length; i++) {
                    headerModel.append({ "colName": vc[i], "colIdx": idx++ })
                }
                headerModel.append({ "colName": "Verify", "colIdx": idx++ })
                headerModel.append({ "colName": "Info", "colIdx": idx++ })
                Qt.callLater(initColumnWidths)
            }

            function initColumnWidths() {
                var w = tableView.width
                var cols = tableView.columns
                if (cols <= 0 || w <= 0) return
                var filenameW = 3.0
                var infoW     = 1.5
                var verifyW   = 1.2
                var hashW     = 2.0
                var hashCols  = Math.max(0, cols - 3)
                var total     = filenameW + infoW + verifyW + hashCols * hashW
                for (var i = 0; i < cols; i++) {
                    if (i === 0)             tableView.setColumnWidth(i, w * filenameW / total)
                    else if (i === cols - 1) tableView.setColumnWidth(i, w * infoW / total)
                    else if (i === cols - 2) tableView.setColumnWidth(i, w * verifyW / total)
                    else                     tableView.setColumnWidth(i, w * hashW / total)
                }
            }

            // Stretch the last column so it always reaches the right edge
            function stretchLastColumn() {
                var cols = tableView.columns
                var w = tableView.width
                if (cols <= 0 || w <= 0) return
                var used = 0
                for (var i = 0; i < cols - 1; i++)
                    used += tableView.columnWidth(i)
                var remaining = w - used
                if (remaining > 50)
                    tableView.setColumnWidth(cols - 1, remaining)
            }

            Component.onCompleted: refreshHeaders()

            ListModel { id: headerModel }

            ColumnLayout {
                anchors.fill: parent
                spacing: 0

                // Sortable, resizable column header
                HorizontalHeaderView {
                    id: headerView
                    Layout.fillWidth: true
                    syncView: tableView
                    resizableColumns: true
                    model: headerModel

                    property int sortColumn: -1
                    property bool sortAscending: true

                    delegate: Rectangle {
                        implicitWidth: 100
                        implicitHeight: 26
                        color: palette.button

                        // Right-edge column divider
                        Rectangle {
                            anchors.right: parent.right
                            width: 1
                            height: parent.height
                            color: palette.mid
                        }

                        RowLayout {
                            anchors { fill: parent; leftMargin: 5; rightMargin: 5 }
                            spacing: 2
                            Label {
                                Layout.fillWidth: true
                                text: model.colName
                                font.bold: true
                                color: palette.buttonText
                                elide: Text.ElideRight
                            }
                            Text {
                                visible: model.colIdx === headerView.sortColumn
                                text: headerView.sortAscending ? "▲" : "▼"
                                color: palette.buttonText
                                font.pixelSize: 9
                            }
                        }

                        MouseArea {
                            // Leave room for the resize handle on the right
                            anchors { fill: parent; rightMargin: 8 }
                            onClicked: {
                                var col = model.colIdx
                                if (headerView.sortColumn === col) {
                                    headerView.sortAscending = !headerView.sortAscending
                                } else {
                                    headerView.sortColumn = col
                                    headerView.sortAscending = true
                                }
                                AppBackend.sort_by(col, headerView.sortAscending)
                            }
                        }
                    }
                }

                // File table
                TableView {
                    id: tableView
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    model: AppBackend
                    clip: true
                    ScrollBar.vertical: ScrollBar {}
                    ScrollBar.horizontal: ScrollBar {}

                    onWidthChanged: Qt.callLater(fileListItem.stretchLastColumn)
                    onColumnsChanged: Qt.callLater(fileListItem.initColumnWidths)

                    delegate: Rectangle {
                        required property int row
                        required property int column
                        implicitWidth: 100
                        implicitHeight: 28
                        color: model.isSelected ? palette.highlight
                             : model.isError    ? Qt.rgba(0.37, 0.07, 0.07, 1)
                             : (row % 2 === 0   ? palette.base : palette.alternateBase)

                        Text {
                            anchors { fill: parent; leftMargin: 6; rightMargin: 4 }
                            text: model.display
                            color: model.isSelected ? palette.highlightedText
                                 : (column === tableView.columns - 2 && model.verifyStatus === 1) ? "#4caf50"
                                 : (column === tableView.columns - 2 && model.verifyStatus === 2) ? "#f44336"
                                 : palette.text
                            elide: Text.ElideRight
                            verticalAlignment: Text.AlignVCenter
                            font.family: (column > 0 && column < tableView.columns - 1) ? "monospace" : Qt.application.font.family
                        }

                        MouseArea {
                            anchors.fill: parent
                            acceptedButtons: Qt.LeftButton | Qt.RightButton
                            onClicked: function(mouse) {
                                AppBackend.select_row(row)
                                if (mouse.button === Qt.RightButton) contextMenu.popup()
                            }
                            onPressAndHold: {
                                AppBackend.select_row(row)
                                contextMenu.popup()
                            }
                        }
                    }
                }
            }
        }
    }
}

