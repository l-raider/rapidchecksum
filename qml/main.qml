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

    // ─── Settings dialog ─────────────────────────────────────────────────
    Dialog {
        id: settingsDialog
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
            MenuItem { text: "CRC32";  onTriggered: AppBackend.copy_hash(0) }
            MenuItem { text: "MD5";    onTriggered: AppBackend.copy_hash(1) }
            MenuItem { text: "SHA1";   onTriggered: AppBackend.copy_hash(2) }
            MenuItem { text: "SHA256"; onTriggered: AppBackend.copy_hash(3) }
            MenuItem { text: "SHA512"; onTriggered: AppBackend.copy_hash(4) }
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
                onTriggered: { saveHashDialog.hashAlgo = 0; saveHashDialog.open() }
            }
            MenuItem {
                text: "MD5"
                onTriggered: { saveHashDialog.hashAlgo = 1; saveHashDialog.open() }
            }
            MenuItem {
                text: "SHA1"
                onTriggered: { saveHashDialog.hashAlgo = 2; saveHashDialog.open() }
            }
            MenuItem {
                text: "SHA256"
                onTriggered: { saveHashDialog.hashAlgo = 3; saveHashDialog.open() }
            }
            MenuItem {
                text: "SHA512"
                onTriggered: { saveHashDialog.hashAlgo = 4; saveHashDialog.open() }
            }
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
                text: "Open Files…"
                onClicked: openFilesDialog.open()
            }
            Button {
                text: "Start Hashing"
                enabled: !AppBackend.is_hashing
                onClicked: AppBackend.start_hashing()
            }
            Button {
                text: "Cancel"
                enabled: AppBackend.is_hashing
                onClicked: AppBackend.cancel_hashing()
            }
            Button {
                text: "Clear List"
                enabled: !AppBackend.is_hashing
                onClicked: AppBackend.clear_list()
            }
            Button {
                text: "Remove Selected"
                enabled: !AppBackend.is_hashing && AppBackend.selected_row >= 0
                onClicked: AppBackend.remove_selected()
            }
            Item { Layout.fillWidth: true }
            Button {
                text: "Settings…"
                onClicked: {
                    chkCrc32.checked  = AppBackend.setting_crc32
                    chkMd5.checked    = AppBackend.setting_md5
                    chkSha1.checked   = AppBackend.setting_sha1
                    chkSha256.checked = AppBackend.setting_sha256
                    chkSha512.checked = AppBackend.setting_sha512
                    settingsDialog.open()
                }
            }
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

        // Split: file list (left) + result panel (right)
        SplitView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            orientation: Qt.Horizontal

            // ── File list ────────────────────────────────────────────────
            Item {
                id: fileListItem
                SplitView.minimumWidth: 300
                SplitView.fillWidth: true

                // Populate header model from visible columns
                function refreshHeaders() {
                    headerModel.clear()
                    headerModel.append({ "colName": "Filename" })
                    var vc = AppBackend.visible_columns()
                    for (var i = 0; i < vc.length; i++) {
                        headerModel.append({ "colName": vc[i] })
                    }
                    headerModel.append({ "colName": "Info" })
                    tableView.forceLayout()
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
                            required property int column
                            implicitWidth: column === 0 ? 220 : (column === headerView.columns - 1 ? 110 : 130)
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
                                    visible: column === headerView.sortColumn
                                    text: headerView.sortAscending ? "▲" : "▼"
                                    color: palette.buttonText
                                    font.pixelSize: 9
                                }
                            }

                            MouseArea {
                                // Leave room for the resize handle on the right
                                anchors { fill: parent; rightMargin: 8 }
                                onClicked: {
                                    if (headerView.sortColumn === column) {
                                        headerView.sortAscending = !headerView.sortAscending
                                    } else {
                                        headerView.sortColumn = column
                                        headerView.sortAscending = true
                                    }
                                    AppBackend.sort_by(column, headerView.sortAscending)
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

                        delegate: Rectangle {
                            required property int row
                            required property int column
                            implicitWidth: column === 0 ? 220 : (column === tableView.columns - 1 ? 110 : 130)
                            implicitHeight: 28
                            color: model.isSelected ? palette.highlight
                                 : model.isError    ? Qt.rgba(0.37, 0.07, 0.07, 1)
                                 : (row % 2 === 0   ? palette.base : palette.alternateBase)

                            Text {
                                anchors { fill: parent; leftMargin: 6; rightMargin: 4 }
                                text: model.display
                                color: model.isSelected ? palette.highlightedText : palette.text
                                elide: Text.ElideRight
                                verticalAlignment: Text.AlignVCenter
                                font.family: (column > 0 && column < tableView.columns - 1) ? "Monospace" : ""
                                font.pixelSize: 13
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

            // ── Result panel (right) ─────────────────────────────────────
            Pane {
                SplitView.minimumWidth: 220
                SplitView.preferredWidth: 280

                ScrollView {
                    anchors.fill: parent
                    ColumnLayout {
                        width: parent.width
                        spacing: 6

                        Label { text: "Selected File"; font.bold: true }

                        Label { text: "Filename:"; font.bold: true }
                        TextEdit {
                            text: AppBackend.result_filename
                            readOnly: true
                            wrapMode: TextEdit.Wrap
                            Layout.fillWidth: true
                        }

                        Label { text: "CRC32:"; font.bold: true; visible: AppBackend.setting_crc32 }
                        TextEdit {
                            visible: AppBackend.setting_crc32
                            text: AppBackend.result_crc32
                            readOnly: true
                            font.family: "monospace"
                            Layout.fillWidth: true
                        }

                        Label { text: "MD5:"; font.bold: true; visible: AppBackend.setting_md5 }
                        TextEdit {
                            visible: AppBackend.setting_md5
                            text: AppBackend.result_md5
                            readOnly: true
                            font.family: "monospace"
                            Layout.fillWidth: true
                        }

                        Label { text: "SHA1:"; font.bold: true; visible: AppBackend.setting_sha1 }
                        TextEdit {
                            visible: AppBackend.setting_sha1
                            text: AppBackend.result_sha1
                            readOnly: true
                            font.family: "monospace"
                            Layout.fillWidth: true
                        }

                        Label { text: "SHA256:"; font.bold: true; visible: AppBackend.setting_sha256 }
                        TextEdit {
                            visible: AppBackend.setting_sha256
                            text: AppBackend.result_sha256
                            readOnly: true
                            font.family: "monospace"
                            Layout.fillWidth: true
                        }

                        Label { text: "SHA512:"; font.bold: true; visible: AppBackend.setting_sha512 }
                        TextEdit {
                            visible: AppBackend.setting_sha512
                            text: AppBackend.result_sha512
                            readOnly: true
                            font.family: "monospace"
                            Layout.fillWidth: true
                            wrapMode: TextEdit.Wrap
                        }

                        Label { text: "Info:"; font.bold: true }
                        TextEdit {
                            text: AppBackend.result_info
                            readOnly: true
                            wrapMode: TextEdit.Wrap
                            Layout.fillWidth: true
                        }
                    }
                }
            }
        }
    }
}
