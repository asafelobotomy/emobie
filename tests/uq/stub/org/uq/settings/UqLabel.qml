// Test stand-in for uQ Settings' UqLabel: a Label that never interprets markup.
import QtQuick
import QtQuick.Controls as QQC2

QQC2.Label {
    textFormat: Text.PlainText
    Accessible.name: text
}
