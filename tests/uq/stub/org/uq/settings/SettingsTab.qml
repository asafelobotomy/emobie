// Test stand-in for uQ Settings' SettingsTab (uq_api = 2): one tab of a SettingsPage, its cards
// in a column (Settings lays them out in two columns on a wide window).
import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts

QQC2.ScrollView {
    id: tab

    property string title: ""
    property string iconName: ""
    property bool available: true
    readonly property bool isSettingsTab: true
    property var activateHook: null
    default property alias content: grid.data

    function activate() {
        if (tab.activateHook)
            tab.activateHook(tab);
    }

    GridLayout {
        id: grid
        width: tab.availableWidth
        columns: 1
    }
}
