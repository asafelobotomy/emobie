// The Macros tab: your macros (switch and delete each), and a form to add one.
import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.uq.settings
import "logic.js" as Logic

SettingsTab {
    id: tab

    required property var host
    property string problem: ""
    readonly property int off: tab.host.view.macros.filter(m => m.enabled === false).length

    title: "Macros"
    iconName: "accessories-text-editor"

    function add() {
        const why = Logic.triggerProblem(trigger.text, tab.host.view.macros) || Logic.expansionProblem(expansion.text);
        tab.problem = why;
        if (why !== "")
            return;
        tab.host.run("add", "macro-add", [trigger.text.trim(), expansion.text], () => {
            trigger.text = "";
            expansion.text = "";
        });
    }

    SettingsCard {
        title: "Your macros"
        iconName: "accessories-text-editor"
        help: "A macro replaces its trigger with its expansion. Deleting one keeps it in Deleted until you remove it for good."

        Loading {
            visible: tab.host.loading
            mutedColor: tab.host.mutedColor
        }
        SettingRow {
            settingId: "macros"
            label: "Macros"
            highlight: tab.host.highlight
            visible: tab.host.ready
            Layout.fillWidth: true
            UqLabel {
                objectName: "macros-count"
                text: Logic.counted(tab.host.view.macros.length, "macro", "macros") + (tab.off > 0 ? ", " + tab.off + " off" : "")
                color: tab.host.mutedColor
            }
        }
        UqLabel {
            objectName: "macros-empty"
            visible: tab.host.ready && tab.host.view.macros.length === 0
            text: "No macros yet. Add one below."
            color: tab.host.mutedColor
            Layout.fillWidth: true
        }
        Repeater {
            model: tab.host.ready ? tab.host.view.macros : []
            MacroRow {
                required property var modelData
                host: tab.host
                entry: modelData
            }
        }
        UqLabel {
            objectName: "macro-note"
            visible: tab.host.noteStarting("macro:") !== ""
            text: tab.host.noteStarting("macro:")
            color: tab.host.errorColor
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }
    }

    SettingsCard {
        title: "Add a macro"
        iconName: "list-add"
        help: "Longer or multi-line text, hotkeys and fonts are set in emobie."

        SettingRow {
            settingId: "add-macro"
            label: "Trigger"
            highlight: tab.host.highlight
            QQC2.TextField {
                id: trigger
                objectName: "add-trigger"
                placeholderText: ".sig"
                maximumLength: 256
                implicitWidth: 220
                onAccepted: expansion.forceActiveFocus()
            }
        }
        SettingRow {
            label: "Expands to"
            QQC2.TextField {
                id: expansion
                objectName: "add-expansion"
                placeholderText: "Best regards"
                maximumLength: 1000
                implicitWidth: 220
                onAccepted: tab.add()
            }
        }
        RowLayout {
            Layout.fillWidth: true
            UqLabel {
                objectName: "add-problem"
                text: tab.problem !== "" ? tab.problem : tab.host.note("add")
                color: tab.host.errorColor
                wrapMode: Text.Wrap
                Layout.fillWidth: true
            }
            QQC2.Button {
                objectName: "add-button"
                text: "Add"
                enabled: tab.host.busy === "" && trigger.text.trim() !== "" && expansion.text !== ""
                onClicked: tab.add()
            }
        }
    }

    SettingsCard {
        title: "Expanding as you type"
        iconName: "input-keyboard"
        help: "Turning this on needs a one-time permission, so it is done in emobie."

        UqLabel {
            objectName: "expand-state"
            text: !tab.host.ready ? "" : (tab.host.expandOn
                ? "On: a macro's trigger is replaced as you type it, in any app. Change when it expands under Settings."
                : "Off: macros are copied from emobie's window. Turn on Expand as you type in emobie to have triggers replaced wherever you type.")
            color: tab.host.mutedColor
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }
        QQC2.Button {
            visible: tab.host.ready && !tab.host.expandOn
            text: "Open emobie"
            onClicked: tab.host.openEmobie()
        }
    }
}
