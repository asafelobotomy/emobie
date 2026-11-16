// Test stand-in for uQ Settings' SettingsPage (uq_api = 2): the page's title and icon with a
// row of tab buttons, then the current SettingsTab. A page with one shown tab has no buttons.
import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts

FocusScope {
    id: page

    property string pageId: ""
    property string title: ""
    property string iconName: ""
    property alias headerItems: headerExtra.data
    property alias banners: bannerColumn.data
    default property alias tabs: stack.data
    readonly property alias currentIndex: stack.currentIndex
    readonly property var tabList: {
        const out = [];
        for (let i = 0; i < stack.children.length; i++) {
            const c = stack.children[i];
            if (c && c.isSettingsTab === true)
                out.push(c);
        }
        return out;
    }
    readonly property var shownTabs: page.tabList.filter(t => t.available)

    function show(tab) {
        const i = page.tabList.indexOf(tab);
        if (i >= 0)
            stack.currentIndex = i;
    }

    Component.onCompleted: {
        for (const t of page.tabList)
            t.activateHook = page.show;
    }

    ColumnLayout {
        anchors.fill: parent

        RowLayout {
            UqLabel {
                text: page.title
                font.bold: true
            }
            Repeater {
                model: page.shownTabs.length > 1 ? page.shownTabs : []
                QQC2.ToolButton {
                    required property var modelData
                    objectName: "tab-" + modelData.title
                    text: modelData.title
                    checkable: true
                    checked: page.tabList[stack.currentIndex] === modelData
                    onClicked: page.show(modelData)
                }
            }
            RowLayout {
                id: headerExtra
            }
        }
        ColumnLayout {
            id: bannerColumn
            Layout.fillWidth: true
        }
        StackLayout {
            id: stack
            Layout.fillWidth: true
            Layout.fillHeight: true
        }
    }
}
