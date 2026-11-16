// One macro: its trigger and what it expands to, its hotkey, an on/off switch and Delete.
// The trigger, expansion and hotkey are shown as plain text; Delete asks uQ to preview and confirm.
import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.uq.settings
import "logic.js" as Logic

RowLayout {
    id: line

    required property var host
    /// An entry of `emobie status`' macros: {id, trigger, expansion, hotkey, enabled}.
    required property var entry
    readonly property bool actionable: Logic.idOk(line.entry.id)
    readonly property bool working: line.host.busy === "macro:" + line.entry.id

    spacing: 8
    Layout.fillWidth: true

    ColumnLayout {
        spacing: 0
        Layout.fillWidth: true
        UqLabel {
            objectName: "macro-trigger"
            text: line.entry.trigger
            font.bold: true
            elide: Text.ElideRight
            opacity: line.entry.enabled ? 1 : 0.55
            Layout.fillWidth: true
        }
        UqLabel {
            objectName: "macro-expansion"
            text: Logic.oneLine(line.entry.expansion, 90)
            color: line.host.mutedColor
            elide: Text.ElideRight
            Layout.fillWidth: true
        }
    }
    UqLabel {
        visible: line.entry.hotkey !== null && line.entry.hotkey !== undefined
        text: Logic.hotkeyText(line.entry.hotkey)
        color: line.host.mutedColor
    }
    QQC2.Switch {
        id: macroSwitch
        objectName: "macro-switch"
        checked: line.entry.enabled
        enabled: line.actionable && line.host.busy === ""
        onToggled: line.host.run("macro:" + line.entry.id, checked ? "macro-enable" : "macro-disable", [line.entry.id])
        // A refused change snaps the switch back to what emobie says.
        Connections {
            target: line
            function onEntryChanged() {
                macroSwitch.checked = line.entry.enabled;
            }
        }
    }
    QQC2.Button {
        objectName: "macro-delete"
        flat: true
        text: "Delete"
        enabled: line.actionable && line.host.busy === ""
        onClicked: line.host.run("macro:" + line.entry.id, "macro-remove", [line.entry.id])
    }
}
