// Test stand-in for uQ Settings' SettingRow: a label on the left, the row's controls on the
// right. `highlight` equal to `settingId` makes it flash in Settings; here it only sets
// `flashing`.
import QtQuick
import QtQuick.Layouts

Item {
    id: row

    property string settingId
    property string label
    property string highlight
    readonly property bool flashing: highlight !== "" && highlight === settingId
    default property alias content: slot.data

    implicitWidth: layout.implicitWidth
    implicitHeight: layout.implicitHeight

    RowLayout {
        id: layout
        anchors.fill: parent

        UqLabel {
            text: row.label
            Layout.fillWidth: true
        }
        RowLayout {
            id: slot
        }
    }
}
