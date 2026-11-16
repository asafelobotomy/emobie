// The Emoji and macros page for uQ Settings (uq_api = 3), laid out like Settings' own pages: three
// tabs of cards. Macros (yours, and add one), Deleted (restore, or delete for good) and Settings
// (everything emobie keeps in its preferences). No emoji are shown. Everything is a command of
// `emobie` (page.toml): reads follow its preferences file (watched), and what removes something
// for good is previewed and confirmed by uQ. Every piece of text goes through UqLabel.
import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.uq.settings
import "logic.js" as Logic

SettingsPage {
    id: page

    pageId: "emobie"

    property var ctx
    property var theme
    /// The [[setting]] row Settings search jumped to; the rows flash, and their tab opens.
    property string highlight

    // What was last read from `emobie status`, and what is in flight ("" when nothing is).
    property var view: Logic.blank("loading", "")
    property string busy: ""
    // Why the last change of an area or setting failed, by key.
    property var notes: ({})
    // Callbacks that arrive after the page is gone do nothing.
    property var life: ({ alive: true })
    Component.onDestruction: life.alive = false

    readonly property bool loading: view.status === "loading"
    readonly property bool ready: view.status === "ready"
    readonly property string mutedColor: colorOf("text_muted", "#707070")
    readonly property string errorColor: colorOf("error", "#c00000")
    // Both permissions are turned on in emobie itself; the page says where they stand.
    readonly property bool expandOn: Logic.row(view, "expandAsYouType").value === true

    function live(fn) {
        const life = page.life;
        return result => {
            if (life.alive)
                fn(result);
        };
    }

    function colorOf(key, fallback) {
        const c = page.ctx && page.ctx.theme && page.ctx.theme.colors;
        return (c && c[key]) || fallback;
    }

    function note(key) {
        return page.notes[key] || "";
    }

    // The first refusal among the notes whose key starts with `prefix` (one per macro, say).
    function noteStarting(prefix) {
        for (const k in page.notes)
            if (k.indexOf(prefix) === 0)
                return page.notes[k];
        return "";
    }

    function setNote(key, text) {
        const copy = {};
        for (const k in page.notes)
            copy[k] = page.notes[k];
        if (text === "")
            delete copy[key];
        else
            copy[key] = text;
        page.notes = copy;
    }

    // A watch and a write's re-read can overlap; the one that lost has nothing new to say.
    function skippable(result) {
        return result && result.error && /already running/i.test(String(result.error));
    }

    function onStatus(result) {
        if (page.skippable(result))
            return;
        page.view = Logic.readStatus(result);
    }

    function refresh() {
        if (page.ctx)
            page.ctx.command.run("status", [], page.live(page.onStatus));
    }

    onCtxChanged: {
        if (page.ctx)
            page.ctx.command.watch("status", page.live(page.onStatus));
    }

    // One of emobie's commands, then a fresh read. `key` names where a refusal is shown;
    // `after(env)` runs when it worked. A declined confirmation is not a failure.
    function run(key, id, args, after) {
        if (page.busy !== "" || !page.ctx)
            return;
        page.busy = key;
        page.setNote(key, "");
        page.ctx.command.run(id, args, page.live(result => {
            page.busy = "";
            if (!Logic.isCancelled(result)) {
                const env = Logic.envelope(result);
                if (!env.ok)
                    page.setNote(key, env.error);
                else if (after)
                    after(env);
            }
            page.refresh();
        }));
    }

    // One setting, as `emobie config set` takes it.
    function setPref(key, value) {
        page.run(key, "config-set", [key, String(value)]);
    }

    function resetPref(key) {
        page.run(key, "config-reset", [key]);
    }

    function openEmobie() {
        if (page.ctx)
            page.ctx.openApp("emobie.desktop", result => {
                if (result && result.error)
                    page.setNote("open", String(result.error).slice(0, 200));
            });
    }

    headerItems: [
        QQC2.Button {
            objectName: "open-emobie"
            text: "Open emobie"
            onClicked: page.openEmobie()
        }
    ]

    banners: [
        Banner {
            objectName: "banner-error"
            visible: page.view.status === "error"
            text: "Couldn't read emobie's settings: " + page.view.error
            buttonText: "Try again"
            buttonEnabled: page.busy === ""
            tone: page.errorColor
            onClicked: page.refresh()
        },
        Banner {
            objectName: "banner-new"
            visible: page.ready && page.view.saved === false
            text: "emobie hasn't saved anything yet, so these are its defaults. Open it once to get started."
            buttonText: "Open emobie"
            tone: page.mutedColor
            onClicked: page.openEmobie()
        },
        Banner {
            objectName: "banner-open"
            visible: page.note("open") !== ""
            text: "Couldn't open emobie: " + page.note("open")
            tone: page.errorColor
        }
    ]

    MacrosTab {
        host: page
    }
    DeletedTab {
        host: page
    }
    PrefsTab {
        host: page
    }
}
