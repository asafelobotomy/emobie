// Test stand-in for uQ Settings' HelpTip (uq_api = 2): a "?" whose tooltip is `tip`; hidden
// when `tip` is empty.
import QtQuick
import QtQuick.Controls as QQC2

QQC2.ToolButton {
    id: help

    property string tip: ""

    visible: help.tip !== ""
    text: "?"
    QQC2.ToolTip.visible: help.hovered
    QQC2.ToolTip.text: help.tip
}
