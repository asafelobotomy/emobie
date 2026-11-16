// Test stand-in for uQ Settings' ValueSlider: `chosen` fires 400 ms after the last move.
import QtQuick
import QtQuick.Controls as QQC2

SettingRow {
    id: root

    property real value: 0
    property real from: 0
    property real to: 1
    property real stepSize: 0.05
    property string lowLabel: ""
    property string highLabel: ""
    property bool busy: false
    signal chosen(real value)
    readonly property alias slider: sl

    UqLabel {
        text: root.lowLabel
        visible: root.lowLabel !== ""
    }
    QQC2.Slider {
        id: sl
        from: root.from
        to: root.to
        stepSize: root.stepSize
        value: root.value
        enabled: root.enabled
        Accessible.name: root.label
        onMoved: settle.restart()
    }
    UqLabel {
        text: root.highLabel
        visible: root.highLabel !== ""
    }
    Timer {
        id: settle
        interval: 400
        onTriggered: root.chosen(Math.round(sl.value * 100) / 100)
    }
    onBusyChanged: {
        if (!busy)
            sl.value = Qt.binding(() => root.value);
    }
}
