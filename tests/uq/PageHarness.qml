// The Emoji and macros page in a Loader with a FakeContext, and helpers to find what it shows and
// what it asked for. Answers come from fixtures/: the real output of the emobie commands
// (src-tauri/src/cli/tests.rs checks they still are). Run with QML_XHR_ALLOW_FILE_READ=1.
import QtQuick

Item {
    id: harness
    anchors.fill: parent

    readonly property url pageUrl: Qt.resolvedUrl("../../uq/pages/emobie/Page.qml")
    readonly property alias fake: fake
    readonly property alias loader: loader
    readonly property var page: loader.item
    // The commands that change something.
    readonly property var writeIds: ["config-set", "config-reset", "excluded-add", "excluded-remove", "macro-add", "macro-enable", "macro-disable", "macro-remove", "macro-restore", "macro-purge", "macro-purge-all"]

    function fixture(name) {
        const xhr = new XMLHttpRequest();
        xhr.open("GET", Qt.resolvedUrl("fixtures/" + name), false);
        xhr.send();
        return JSON.parse(xhr.responseText);
    }

    function ok(summary, over) {
        return { code: 0, output: Object.assign({ ok: true, summary: summary || "Saved", path: "/p" }, over || {}) };
    }

    // `emobie` refusing: a non-zero exit with the JSON error on stdout.
    function refused(name) {
        return { code: 1, error: "exit 1", output: fixture(name) };
    }

    function arrange(over) {
        const o = over || {};
        fake.confirms = true;
        fake.confirmed = ["macro-remove", "macro-purge", "macro-purge-all"];
        fake.commands = Object.assign({
            "status": { code: 0, output: fixture("status.json") },
            "macro-remove-plan": { code: 0, output: fixture("plan-remove.json") },
            "macro-purge-plan": { code: 0, output: fixture("plan-purge.json") },
            "macro-purge-all-plan": { code: 0, output: fixture("plan-purge-all.json") },
            "config-set": ok("Saved"),
            "config-reset": ok("Saved"),
            "excluded-add": ok("Saved"),
            "excluded-remove": ok("Saved"),
            "macro-add": ok("Added"),
            "macro-enable": ok("Turned on"),
            "macro-disable": ok("Turned off"),
            "macro-remove": ok("Deleted"),
            "macro-restore": ok("Restored"),
            "macro-purge": ok("Removed"),
            "macro-purge-all": ok("Removed")
        }, o.commands || {});
    }

    // Every request the page made, by kind.
    function runs(id) {
        return fake.calls.filter(c => c[0] === "run" && (id === undefined || c[1] === id));
    }
    function writes() {
        return fake.calls.filter(c => c[0] === "run" && harness.writeIds.indexOf(c[1]) >= 0);
    }

    function walk(item, test) {
        if (test(item))
            return item;
        for (let i = 0; i < item.children.length; ++i) {
            const hit = walk(item.children[i], test);
            if (hit)
                return hit;
        }
        return null;
    }
    function findLabel(text) {
        return walk(loader.item, it => it.text === text && it.textFormat !== undefined);
    }
    function findButton(text) {
        return walk(loader.item, it => it.text === text && it.clicked !== undefined);
    }
    function findNamed(name) {
        return walk(loader.item, it => it.objectName === name);
    }
    // Every item with this objectName, in order.
    function all(name) {
        const out = [];
        const visit = item => {
            if (item.objectName === name)
                out.push(item);
            for (let i = 0; i < item.children.length; ++i)
                visit(item.children[i]);
        };
        visit(loader.item);
        return out;
    }
    // Opens a tab of the page, as the person would: by its button.
    function tab(title) {
        const button = findNamed("tab-" + title);
        if (button === null)
            throw new Error("no tab " + title);
        button.clicked();
    }

    FakeContext {
        id: fake
    }

    Loader {
        id: loader
        anchors.fill: parent
    }
}
