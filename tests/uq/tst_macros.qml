// The Macros and Deleted tabs offscreen, with the stub org.uq.settings module and a fake ctx:
//   QML_XHR_ALLOW_FILE_READ=1 QT_QPA_PLATFORM=offscreen qmltestrunner -import tests/uq/stub -input tests/uq
import QtQuick
import QtTest

Item {
    id: root
    width: 700
    height: 600

    PageHarness {
        id: h
    }

    TestCase {
        name: "MacrosAndDeleted"
        when: windowShown

        function load(over) {
            h.fake.calls = [];
            h.arrange(over);
            h.loader.source = "";
            h.loader.source = h.pageUrl;
            verify(h.loader.status === Loader.Ready, "the page loads");
            h.loader.item.ctx = h.fake.ctx;
            tryVerify(() => h.page.ready || h.page.view.status === "error");
        }

        function test_reads_status_and_follows_it() {
            load();
            verify(h.fake.calls.some(c => c[0] === "watch" && c[1] === "status"));
            compare(h.writes().length, 0, "reading changes nothing");
            compare(h.page.view.macros.length, 2);
            h.fake.commands = Object.assign({}, h.fake.commands, { "status": { code: 0, output: Object.assign({}, h.fixture("status.json"), { macros: [] }) } });
            h.fake.change("status");
            tryVerify(() => h.page.view.macros.length === 0);
            verify(h.findNamed("macros-empty") !== null && h.findNamed("macros-empty").visible);
        }

        function test_lists_the_macros() {
            load();
            const triggers = h.all("macro-trigger").map(l => l.text);
            compare(triggers, [".sig", ".addr"]);
            verify(h.findLabel("Best regards, Sam") !== null, "a line break shows as a space");
            verify(h.findLabel("Ctrl+Alt+S") !== null, "the hotkey, in short");
            const switches = h.all("macro-switch");
            compare(switches[0].checked, true);
            compare(switches[1].checked, false, "the off macro");
            compare(h.findNamed("macros-empty").visible, false);
        }

        function test_nothing_shows_markup() {
            load({ commands: { "status": { code: 0, output: Object.assign({}, h.fixture("status.json"), { macros: [{ id: "x1", trigger: "<b>bold</b>", expansion: "<a href='x'>link</a>", hotkey: null, enabled: true }] }) } } });
            const t = h.findNamed("macro-trigger");
            compare(t.text, "<b>bold</b>");
            compare(t.textFormat, Text.PlainText);
        }

        function test_switching_a_macro_runs_enable_or_disable() {
            load();
            const switches = h.all("macro-switch");
            switches[0].checked = false;
            switches[0].toggled();
            compare(h.writes()[0], ["run", "macro-disable", ["m1"]]);
            tryVerify(() => h.runs("status").length >= 2, 2000, "it reads again afterwards");
            tryCompare(h.page, "busy", "");
            h.all("macro-switch")[1].checked = true;
            h.all("macro-switch")[1].toggled();
            compare(h.writes()[1], ["run", "macro-enable", ["m2"]]);
        }

        function test_a_refusal_is_shown_and_the_switch_snaps_back() {
            load({ commands: { "macro-disable": h.refused("error-trigger.json") } });
            const sw = h.all("macro-switch")[0];
            sw.checked = false;
            sw.toggled();
            tryVerify(() => h.findLabel("Another macro already uses this trigger.") !== null);
            compare(h.findNamed("macro-note").visible, true);
        }

        function test_delete_goes_through_uqs_preview_and_confirm() {
            load();
            h.all("macro-delete")[0].clicked();
            compare(h.writes()[0], ["run", "macro-remove", ["m1"]]);
            tryCompare(h.page, "busy", "");
            // Declined: nothing is shown as a failure.
            h.fake.confirms = false;
            h.all("macro-delete")[1].clicked();
            tryCompare(h.page, "busy", "");
            compare(h.page.noteStarting("macro:"), "");
        }

        function test_adding_a_macro_checks_first() {
            load();
            const trigger = h.findNamed("add-trigger");
            const expansion = h.findNamed("add-expansion");
            const add = h.findNamed("add-button");
            compare(add.enabled, false, "empty fields");
            trigger.text = ".sig";
            expansion.text = "again";
            compare(add.enabled, true);
            add.clicked();
            compare(h.writes().length, 0, "a trigger in use is caught before emobie is asked");
            compare(h.findNamed("add-problem").text, "Another macro already uses this trigger.");
            trigger.text = " .new ";
            expansion.text = "A new one";
            add.clicked();
            compare(h.writes()[0], ["run", "macro-add", [".new", "A new one"]]);
            tryCompare(trigger, "text", "", 2000, "the form clears once it is added");
            compare(expansion.text, "");
        }

        function test_what_cannot_be_typed_here_is_explained() {
            load();
            h.findNamed("add-trigger").text = "-x";
            h.findNamed("add-expansion").text = "y";
            h.findNamed("add-button").clicked();
            verify(h.findNamed("add-problem").text.indexOf("dash") >= 0);
            h.findNamed("add-trigger").text = ".ok";
            h.findNamed("add-expansion").text = "a\tb";
            h.findNamed("add-button").clicked();
            verify(h.findNamed("add-problem").text.indexOf("emobie") >= 0);
            compare(h.writes().length, 0);
        }

        function test_expansion_state_follows_the_setting() {
            load();
            verify(h.findNamed("expand-state").text.indexOf("On:") === 0);
            const off = h.fixture("status.json");
            off.config.expandAsYouType.value = false;
            load({ commands: { "status": { code: 0, output: off } } });
            verify(h.findNamed("expand-state").text.indexOf("Off:") === 0);
        }

        function test_deleted_tab_restores_and_purges() {
            load();
            h.tab("Deleted");
            compare(h.all("deleted-trigger").map(l => l.text), [".thanks", ".old"]);
            verify(h.findLabel("Thank you! · deleted " + h.page.view.deleted[0].deleted_at) === null, "dates are wording, not numbers");
            h.all("restore")[0].clicked();
            compare(h.writes()[0], ["run", "macro-restore", ["d1"]]);
            tryCompare(h.page, "busy", "");
            h.all("purge")[1].clicked();
            compare(h.writes()[1], ["run", "macro-purge", ["d2"]]);
            tryCompare(h.page, "busy", "");
            h.findNamed("purge-all").clicked();
            compare(h.writes()[2], ["run", "macro-purge-all", []]);
        }

        function test_restore_refusal_stays_on_the_tab() {
            load({ commands: { "macro-restore": h.refused("error-restore.json") } });
            h.tab("Deleted");
            h.all("restore")[0].clicked();
            tryVerify(() => h.findNamed("deleted-note").visible);
            compare(h.findNamed("deleted-note").text, "There is no such deleted macro.");
        }

        function test_empty_deleted_tab_says_so() {
            load({ commands: { "status": { code: 0, output: Object.assign({}, h.fixture("status.json"), { deleted: [] }) } } });
            h.tab("Deleted");
            compare(h.findNamed("deleted-empty").visible, true);
            compare(h.findNamed("purge-all").visible, false);
        }

        function test_an_odd_id_cannot_be_acted_on_from_here() {
            load({ commands: { "status": { code: 0, output: Object.assign({}, h.fixture("status.json"), { macros: [{ id: "has space/slash", trigger: ".odd", expansion: "x", hotkey: null, enabled: true }] }) } } });
            compare(h.findNamed("macro-delete").enabled, false);
            compare(h.findNamed("macro-switch").enabled, false);
        }
    }
}
