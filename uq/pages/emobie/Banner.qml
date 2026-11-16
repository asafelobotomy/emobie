// A page-wide message under the title: what is wrong or going on, with the button that deals
// with it. Text and button label are fixed strings or from the page's logic; plain text.
import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.uq.settings

Rectangle {
    id: banner

    property string text
    property string buttonText
    property color tone: "grey"
    property bool buttonEnabled: true
    readonly property alias button: action

    signal clicked

    implicitHeight: row.implicitHeight + 16
    color: Qt.rgba(banner.tone.r, banner.tone.g, banner.tone.b, 0.14)
    border.width: 1
    border.color: banner.tone
    radius: 6
    Layout.fillWidth: true

    RowLayout {
        id: row
        anchors.fill: parent
        anchors.margins: 8
        spacing: 8
        UqLabel {
            text: banner.text
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }
        QQC2.Button {
            id: action
            visible: banner.buttonText !== ""
            text: banner.buttonText
            enabled: banner.buttonEnabled
            onClicked: banner.clicked()
        }
    }
}
