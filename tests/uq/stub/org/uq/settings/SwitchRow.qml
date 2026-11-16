// Test stand-in for uQ Settings' SwitchRow: a label and a switch that shows `value` again after
// a flip, until the page changes `value`.
import QtQuick
import QtQuick.Controls as QQC2

SettingRow {
    id: root

    property bool value: false
    signal chosen(bool value)
    readonly property alias toggle: sw

    QQC2.Switch {
        id: sw
        checked: root.value
        enabled: root.enabled
        Accessible.name: root.label
        onToggled: {
            root.chosen(sw.checked);
            sw.checked = Qt.binding(() => root.value);
        }
    }
}
