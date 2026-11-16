// The Settings tab: what emobie keeps in its preferences, followed live from its file. The two
// settings that need a one-time permission are shown, and turned on in emobie.
import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.uq.settings
import "logic.js" as Logic

SettingsTab {
    id: tab

    required property var host
    readonly property var apps: Logic.row(tab.host.view, "expandExcludedApps").value || []
    property string appProblem: ""

    title: "Settings"
    iconName: "configure"

    function addApp() {
        const why = Logic.appProblem(appName.text, tab.apps);
        tab.appProblem = why;
        if (why !== "")
            return;
        tab.host.run("expandExcludedApps", "excluded-add", [appName.text.trim()], () => {
            appName.text = "";
        });
    }

    SettingsCard {
        title: "Window"
        iconName: "preferences-desktop-theme"
        Loading {
            visible: tab.host.loading
            mutedColor: tab.host.mutedColor
        }
        SettingLine { host: tab.host; rowKey: "theme"; settingId: "theme" }
        SettingLine { host: tab.host; rowKey: "showTitleBar" }
        SettingLine { host: tab.host; rowKey: "allowMultipleInstances" }
    }

    SettingsCard {
        title: "Starting and opening"
        iconName: "system-run"
        help: "emobie applies these while it runs, and when it next starts if it isn't running."
        SettingLine { host: tab.host; rowKey: "hotkey"; settingId: "hotkey" }
        SettingLine { host: tab.host; rowKey: "launchOnStartup"; settingId: "launchOnStartup" }
        SettingLine { host: tab.host; rowKey: "startMinimizedToTray" }
        SettingLine { host: tab.host; rowKey: "checkUpdatesOnStartup" }
    }

    SettingsCard {
        title: "Macros and pasting"
        iconName: "input-keyboard"
        SettingLine { host: tab.host; rowKey: "expandTriggerMode"; settingId: "expandTriggerMode" }
        SettingLine { host: tab.host; rowKey: "expandKeepTriggerSpace" }
        SettingLine { host: tab.host; rowKey: "pasteChordOverride"; settingId: "pasteChordOverride" }
        SettingLine { host: tab.host; rowKey: "expandRestoreClipboard" }
    }

    SettingsCard {
        id: permissions
        title: "Typing for you"
        iconName: "security-medium"
        help: "These need a one-time permission to type into other apps, so you turn them on in emobie."

        Repeater {
            model: [["autoPasteOnCopy", "Paste after copying"], ["expandAsYouType", "Expand as you type"]]
            SettingRow {
                id: permRow
                required property var modelData
                settingId: permRow.modelData[0] === "autoPasteOnCopy" ? "permissions" : ""
                label: permRow.modelData[1]
                highlight: tab.host.highlight
                UqLabel {
                    objectName: "perm-" + permRow.modelData[0]
                    text: !tab.host.ready ? "" : (Logic.row(tab.host.view, permRow.modelData[0]).value === true ? "On" : "Off")
                    color: tab.host.mutedColor
                }
            }
        }
        QQC2.Button {
            objectName: "perm-open"
            text: "Change in emobie"
            onClicked: tab.host.openEmobie()
        }
    }

    SettingsCard {
        title: "Apps where macros never expand"
        iconName: "dialog-password"
        help: "Matched against the focused app's class, in part and ignoring case. Password managers are listed by default."

        UqLabel {
            objectName: "apps-empty"
            visible: tab.host.ready && tab.apps.length === 0
            text: "No apps listed, so macros can expand anywhere."
            color: tab.host.mutedColor
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }
        Repeater {
            model: tab.host.ready ? tab.apps : []
            RowLayout {
                id: appLine
                required property string modelData
                Layout.fillWidth: true
                UqLabel {
                    text: appLine.modelData
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                }
                QQC2.Button {
                    objectName: "app-remove"
                    flat: true
                    text: "Remove"
                    enabled: tab.host.busy === ""
                    onClicked: tab.host.run("expandExcludedApps", "excluded-remove", [appLine.modelData])
                }
            }
        }
        SettingRow {
            settingId: "excluded"
            label: "Add an app"
            highlight: tab.host.highlight
            QQC2.TextField {
                id: appName
                objectName: "app-name"
                placeholderText: "keepassxc"
                maximumLength: 128
                implicitWidth: 180
                onAccepted: tab.addApp()
            }
            QQC2.Button {
                objectName: "app-add"
                text: "Add"
                enabled: tab.host.busy === "" && appName.text.trim() !== ""
                onClicked: tab.addApp()
            }
        }
        UqLabel {
            objectName: "app-problem"
            visible: text !== ""
            text: tab.appProblem !== "" ? tab.appProblem : tab.host.note("expandExcludedApps")
            color: tab.host.errorColor
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }
        QQC2.Button {
            objectName: "apps-reset"
            visible: Logic.row(tab.host.view, "expandExcludedApps").saved === true
            text: "Back to the default list"
            enabled: tab.host.busy === ""
            onClicked: tab.host.resetPref("expandExcludedApps")
        }
    }

    SettingsCard {
        title: "Emoji palette"
        iconName: "view-grid"
        help: "How the palette itself looks and what it remembers. The emoji are in emobie."
        SettingLine { host: tab.host; rowKey: "emojiSize" }
        SettingLine { host: tab.host; rowKey: "sortBy" }
        SettingLine { host: tab.host; rowKey: "skinTone" }
        SettingLine { host: tab.host; rowKey: "recentMax" }
        SettingLine { host: tab.host; rowKey: "favoriteEmojiMacros" }
        SettingLine { host: tab.host; rowKey: "emoticonStyle" }
    }
}
