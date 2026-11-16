// The page's loading, error and first-run states, the Settings tab and the header.
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
        name: "StatesAndSettings"
        when: windowShown

        function load(over, wait) {
            h.fake.calls = [];
            h.arrange(over);
            h.loader.source = "";
            h.loader.source = h.pageUrl;
            verify(h.loader.status === Loader.Ready, "the page loads");
            h.loader.item.ctx = h.fake.ctx;
            if (wait !== false)
                tryVerify(() => h.page.view.status !== "loading");
        }

        function test_shows_a_bar_while_reading() {
            h.fake.calls = [];
            h.arrange();
            h.loader.source = "";
            h.loader.source = h.pageUrl;
            compare(h.page.loading, true);
            verify(h.findNamed("loading-bar") !== null && h.findNamed("loading-bar").visible, "a moving bar, not a bare word");
            h.page.ctx = h.fake.ctx;
            tryCompare(h.page, "ready", true);
            compare(h.findNamed("loading-bar").visible, false);
        }

        function test_a_failed_read_shows_a_banner_and_tries_again() {
            load({ commands: { "status": { error: "timed out" } } });
            compare(h.page.view.status, "error");
            compare(h.findNamed("banner-error").visible, true);
            verify(h.findLabel("Couldn't read emobie's settings: timed out") !== null);
            h.fake.commands = Object.assign({}, h.fake.commands, { "status": { code: 0, output: h.fixture("status.json") } });
            h.findNamed("banner-error").button.clicked();
            tryCompare(h.page, "ready", true);
            compare(h.findNamed("banner-error").visible, false);
        }

        function test_a_first_run_says_so() {
            load({ commands: { "status": { code: 0, output: h.fixture("status-empty.json") } } });
            compare(h.findNamed("banner-new").visible, true);
            compare(h.findNamed("macros-empty").visible, true);
            compare(h.findNamed("pref-theme").current, "system", "the defaults still show");
        }

        function test_open_emobie_in_the_header_and_the_banner() {
            load();
            h.findNamed("open-emobie").clicked();
            compare(h.fake.calls.filter(c => c[0] === "openApp"), [["openApp", "emobie.desktop"]]);
            h.fake.calls = [];
            h.findNamed("perm-open").clicked();
            compare(h.fake.calls.filter(c => c[0] === "openApp").length, 1);
        }

        function test_settings_show_what_is_saved() {
            load();
            h.tab("Settings");
            compare(h.findNamed("pref-theme").current, "dark");
            compare(h.findNamed("pref-hotkey").text, "Control+Alt+E");
            compare(h.findNamed("pref-recentMax").value, 40);
            compare(h.findNamed("pref-skinTone").value, 0);
            compare(h.findNamed("pref-showTitleBar").toggle.checked, false);
            compare(h.findNamed("pref-checkUpdatesOnStartup").toggle.checked, true, "a default of on");
            compare(h.findNamed("perm-expandAsYouType").text, "On");
            compare(h.findNamed("perm-autoPasteOnCopy").text, "Off");
        }

        function test_changing_settings_runs_config_set() {
            load();
            h.tab("Settings");
            h.findNamed("pref-showTitleBar").toggle.checked = true;
            h.findNamed("pref-showTitleBar").toggle.toggled();
            compare(h.writes()[0], ["run", "config-set", ["showTitleBar", "on"]]);
            tryCompare(h.page, "busy", "");
            h.findNamed("pref-theme").picked("light");
            compare(h.writes()[1], ["run", "config-set", ["theme", "light"]]);
            tryCompare(h.page, "busy", "");
            h.findNamed("pref-recentMax").slider.value = 64;
            h.findNamed("pref-recentMax").slider.moved();
            tryVerify(() => h.writes().length === 3, 2000);
            compare(h.writes()[2], ["run", "config-set", ["recentMax", "64"]]);
        }

        function test_the_hotkey_is_set_or_reset() {
            load();
            h.tab("Settings");
            const keys = h.findNamed("pref-hotkey");
            compare(h.findNamed("set-hotkey").enabled, false, "unchanged");
            keys.text = "Control+Alt+K";
            compare(h.findNamed("set-hotkey").enabled, true);
            h.findNamed("set-hotkey").clicked();
            compare(h.writes()[0], ["run", "config-set", ["hotkey", "Control+Alt+K"]]);
            tryCompare(h.page, "busy", "");
            compare(h.findNamed("reset-hotkey").visible, true, "a changed hotkey can go back");
            h.findNamed("reset-hotkey").clicked();
            compare(h.writes()[1], ["run", "config-reset", ["hotkey"]]);
        }

        function test_a_refused_setting_is_shown_under_it_and_snaps_back() {
            load({ commands: { "config-set": h.refused("error-theme.json") } });
            h.tab("Settings");
            h.findNamed("pref-theme").picked("blue");
            tryVerify(() => h.findNamed("note-theme").visible);
            verify(h.findNamed("note-theme").text.indexOf("system, light, dark") >= 0);
            compare(h.findNamed("pref-theme").current, "dark");
        }

        function test_excluded_apps_are_added_and_removed() {
            load();
            h.tab("Settings");
            compare(h.all("app-remove").length, 2);
            h.all("app-remove")[1].clicked();
            compare(h.writes()[0], ["run", "excluded-remove", ["my-terminal"]]);
            tryCompare(h.page, "busy", "");
            h.findNamed("app-name").text = "KeePassXC";
            h.findNamed("app-add").clicked();
            compare(h.writes().length, 1, "already listed, ignoring case");
            compare(h.findNamed("app-problem").text, "KeePassXC is already in the list.");
            h.findNamed("app-name").text = "kate";
            h.findNamed("app-add").clicked();
            compare(h.writes()[1], ["run", "excluded-add", ["kate"]]);
            tryCompare(h.page, "busy", "");
            compare(h.findNamed("apps-reset").visible, true);
        }

        function test_only_one_change_runs_at_a_time() {
            load();
            h.tab("Settings");
            h.page.run("a", "config-set", ["theme", "light"]);
            h.page.run("b", "config-set", ["theme", "dark"]);
            compare(h.writes().length, 1);
            tryCompare(h.page, "busy", "");
        }
    }
}
