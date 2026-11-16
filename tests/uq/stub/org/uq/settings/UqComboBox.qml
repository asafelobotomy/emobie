// Test stand-in for uQ Settings' UqComboBox: a ComboBox whose value and rows are UqLabels.
import QtQuick
import QtQuick.Controls as QQC2

QQC2.ComboBox {
    id: combo

    displayText: ""
    contentItem: UqLabel {
        text: combo.currentText
        elide: Text.ElideRight
        verticalAlignment: Text.AlignVCenter
    }
    delegate: QQC2.ItemDelegate {
        required property int index
        required property var modelData
        width: combo.width
        highlighted: combo.highlightedIndex === index
        contentItem: UqLabel {
            text: modelData
        }
    }
}
