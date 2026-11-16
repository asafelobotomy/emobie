// One setting of `emobie status`' config: its words, a control that fits its type, and why emobie
// refused the last change. A change runs `emobie config set`; the row shows emobie's value again
// until the page reads the change back, so a refused change snaps back.
import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.uq.settings
import "logic.js" as Logic

ColumnLayout {
    id: line

    required property var host
    required property string rowKey
    property string settingId: ""

    readonly property var row: Logic.row(line.host.view, line.rowKey)
    readonly property var meta: Logic.metaOf(line.rowKey)
    readonly property bool free: line.host.busy === ""
    readonly property string note: line.host.note(line.rowKey)

    Layout.fillWidth: true
    spacing: 2

    SwitchRow {
        objectName: line.row.type === "bool" ? "pref-" + line.rowKey : ""
        visible: line.row.type === "bool"
        settingId: line.settingId
        label: line.meta.label
        highlight: line.host.highlight
        value: line.row.value === true
        enabled: line.free
        onChosen: v => line.host.setPref(line.rowKey, v ? "on" : "off")
        Layout.fillWidth: true
    }

    SettingRow {
        visible: line.row.type === "enum"
        settingId: line.settingId
        label: line.meta.label
        highlight: line.host.highlight
        Layout.fillWidth: true
        ChoiceBox {
            objectName: line.row.type === "enum" ? "pref-" + line.rowKey : ""
            choices: (line.meta.choices || []).map(c => ({ text: c[1], value: c[0] }))
            current: line.row.value
            enabled: line.free
            onPicked: v => line.host.setPref(line.rowKey, v)
        }
    }

    ValueSlider {
        objectName: line.row.type === "int" ? "pref-" + line.rowKey : ""
        visible: line.row.type === "int"
        settingId: line.settingId
        label: line.meta.label
        highlight: line.host.highlight
        from: line.row.min === undefined ? 0 : line.row.min
        to: line.row.max === undefined ? 1 : line.row.max
        stepSize: 1
        lowLabel: line.meta.lowLabel || ""
        highLabel: line.meta.highLabel || ""
        value: typeof line.row.value === "number" ? line.row.value : 0
        busy: !line.free
        onChosen: v => line.host.setPref(line.rowKey, Math.round(v))
        Layout.fillWidth: true
    }

    SettingRow {
        visible: line.row.type === "hotkey"
        settingId: line.settingId
        label: line.meta.label
        highlight: line.host.highlight
        Layout.fillWidth: true
        QQC2.TextField {
            id: keys
            objectName: line.row.type === "hotkey" ? "pref-" + line.rowKey : ""
            text: typeof line.row.value === "string" ? line.row.value : ""
            maximumLength: 64
            implicitWidth: 200
            enabled: line.free
            onAccepted: line.host.setPref(line.rowKey, keys.text.trim())
            // A refused change shows what emobie keeps.
            Connections {
                target: line
                function onRowChanged() {
                    keys.text = typeof line.row.value === "string" ? line.row.value : "";
                }
                function onNoteChanged() {
                    keys.text = typeof line.row.value === "string" ? line.row.value : "";
                }
            }
        }
        QQC2.Button {
            objectName: "set-" + line.rowKey
            text: "Set"
            enabled: line.free && keys.text.trim() !== "" && keys.text.trim() !== line.row.value
            onClicked: line.host.setPref(line.rowKey, keys.text.trim())
        }
        QQC2.Button {
            objectName: "reset-" + line.rowKey
            flat: true
            text: "Default"
            visible: line.row.saved === true && line.row.value !== line.row.default
            enabled: line.free
            onClicked: line.host.resetPref(line.rowKey)
        }
    }

    UqLabel {
        visible: line.meta.help !== "" && line.row.type !== ""
        text: line.meta.help
        color: line.host.mutedColor
        wrapMode: Text.Wrap
        Layout.fillWidth: true
    }
    UqLabel {
        objectName: "note-" + line.rowKey
        visible: line.note !== ""
        text: line.note
        color: line.host.errorColor
        wrapMode: Text.Wrap
        Layout.fillWidth: true
    }
}
