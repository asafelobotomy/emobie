// The Deleted tab: macros that were deleted, kept so they can be restored, and what removes them
// for good (uQ previews and confirms that).
import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.uq.settings
import "logic.js" as Logic

SettingsTab {
    id: tab

    required property var host
    readonly property var items: tab.host.ready ? tab.host.view.deleted : []

    title: "Deleted"
    iconName: "edit-delete"

    SettingsCard {
        title: "Deleted macros"
        iconName: "edit-delete"
        help: "emobie keeps the newest " + tab.host.view.deletedMax + " deleted macros. Restoring one puts it back as it was."
        actions: [
            QQC2.Button {
                objectName: "purge-all"
                text: "Delete all for good"
                visible: tab.items.length > 0
                enabled: tab.host.busy === ""
                onClicked: tab.host.run("deleted", "macro-purge-all", [])
            }
        ]

        Loading {
            visible: tab.host.loading
            mutedColor: tab.host.mutedColor
        }
        SettingRow {
            settingId: "deleted"
            label: "Deleted macros"
            highlight: tab.host.highlight
            visible: tab.host.ready
            Layout.fillWidth: true
            UqLabel {
                text: tab.items.length + " kept"
                color: tab.host.mutedColor
            }
        }
        UqLabel {
            objectName: "deleted-empty"
            visible: tab.host.ready && tab.items.length === 0
            text: "Nothing deleted. A macro you delete shows up here, and you can bring it back."
            color: tab.host.mutedColor
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }
        Repeater {
            model: tab.items
            RowLayout {
                id: line
                required property var modelData
                readonly property bool actionable: Logic.idOk(line.modelData.id)
                spacing: 8
                Layout.fillWidth: true

                ColumnLayout {
                    spacing: 0
                    Layout.fillWidth: true
                    UqLabel {
                        objectName: "deleted-trigger"
                        text: line.modelData.trigger
                        font.bold: true
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }
                    UqLabel {
                        text: Logic.oneLine(line.modelData.expansion, 70) + (line.modelData.deleted_at ? " · deleted " + Logic.whenText(line.modelData.deleted_at, Date.now()) : "")
                        color: tab.host.mutedColor
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }
                }
                QQC2.Button {
                    objectName: "restore"
                    text: "Restore"
                    enabled: line.actionable && tab.host.busy === ""
                    onClicked: tab.host.run("deleted:" + line.modelData.id, "macro-restore", [line.modelData.id])
                }
                QQC2.Button {
                    objectName: "purge"
                    flat: true
                    text: "Delete for good"
                    enabled: line.actionable && tab.host.busy === ""
                    onClicked: tab.host.run("deleted:" + line.modelData.id, "macro-purge", [line.modelData.id])
                }
            }
        }
        UqLabel {
            objectName: "deleted-note"
            visible: tab.host.noteStarting("deleted") !== ""
            text: tab.host.noteStarting("deleted")
            color: tab.host.errorColor
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }
    }
}
