// Test stand-in for uQ Settings' ErrorCard: a title and the error text, both plain text.
import QtQuick
import QtQuick.Layouts

Rectangle {
    id: card

    property string title
    property string detail

    implicitWidth: 400
    implicitHeight: column.implicitHeight + 16
    border.width: 1
    border.color: "red"

    ColumnLayout {
        id: column
        anchors.fill: parent
        anchors.margins: 8

        UqLabel {
            text: card.title
            font.bold: true
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }
        UqLabel {
            text: card.detail
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }
    }
}
