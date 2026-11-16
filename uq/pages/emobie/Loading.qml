// What a card shows while emobie's answer is on its way: a bar that moves, not a bare word.
import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.uq.settings

ColumnLayout {
    property string text: "Reading emobie…"
    property color mutedColor: "grey"

    spacing: 6
    Layout.fillWidth: true

    QQC2.ProgressBar {
        objectName: "loading-bar"
        indeterminate: true
        Layout.fillWidth: true
    }
    UqLabel {
        text: parent.text
        color: parent.mutedColor
        Layout.fillWidth: true
    }
}
