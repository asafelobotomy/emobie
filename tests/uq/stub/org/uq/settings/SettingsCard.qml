// Test stand-in for uQ Settings' SettingsCard (uq_api = 2): a titled group of rows, with
// optional buttons beside the title and a one-line `help`.
import QtQuick
import QtQuick.Layouts

Rectangle {
    id: card

    property string title: ""
    property string iconName: ""
    property string help: ""
    property bool wide: false
    property alias actions: actionsRow.data
    default property alias content: body.data

    Layout.fillWidth: true
    implicitWidth: col.implicitWidth + 16
    implicitHeight: col.implicitHeight + 16
    border.width: 1
    border.color: "grey"

    ColumnLayout {
        id: col
        anchors.fill: parent
        anchors.margins: 8

        RowLayout {
            UqLabel {
                text: card.title
                font.bold: true
                Layout.fillWidth: true
            }
            HelpTip {
                tip: card.help
            }
            RowLayout {
                id: actionsRow
            }
        }
        ColumnLayout {
            id: body
            Layout.fillWidth: true
        }
    }
}
