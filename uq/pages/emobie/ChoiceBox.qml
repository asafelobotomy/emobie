// A UqComboBox over [{text, value}] choices, where `value: null` means "not set". After a pick
// it shows `current` again until the page changes it, so a refused change snaps back.
import QtQuick
import org.uq.settings

UqComboBox {
    id: box

    property var choices: []
    property var current: null

    signal picked(var value)

    function indexOf(v) {
        const want = v === undefined ? null : v;
        for (let i = 0; i < box.choices.length; i++)
            if (box.choices[i].value === want)
                return i;
        return -1;
    }

    function sync() {
        box.currentIndex = box.indexOf(box.current);
    }

    model: box.choices.map(c => c.text)
    onModelChanged: later.restart()
    onCurrentChanged: later.restart()
    Component.onCompleted: box.sync()
    onActivated: index => {
        const v = box.choices[index] ? box.choices[index].value : null;
        later.restart();
        if (v !== (box.current === undefined ? null : box.current))
            box.picked(v);
    }
    // Runs sync() once things settle; dies with the control, unlike Qt.callLater.
    Timer {
        id: later
        interval: 0
        onTriggered: box.sync()
    }
}
