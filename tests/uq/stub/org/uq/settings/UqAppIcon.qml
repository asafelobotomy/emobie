// Test stand-in for uQ Settings' UqAppIcon (uq_api = 2): no icon theme here, so it always shows
// the letter tile; `plainName` follows the same rule as the real one.
import QtQuick

Rectangle {
    id: icon

    property string name: ""
    property string label: ""
    property int size: 32
    readonly property bool plainName: /^[A-Za-z0-9_+-][A-Za-z0-9._+-]{0,127}$/.test(icon.name)
    readonly property bool found: false

    implicitWidth: icon.size
    implicitHeight: icon.size
    radius: Math.round(width * 0.22)
    color: "grey"
    Accessible.role: Accessible.Graphic
    Accessible.name: icon.label

    UqLabel {
        anchors.centerIn: parent
        text: icon.label.length > 0 ? icon.label.charAt(0).toUpperCase() : "?"
    }
}
