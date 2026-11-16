# emobie in uQ

What this folder adds to the [uQ](https://github.com/asafelobotomy/uq) desktop. emobie's package
installs it unconditionally; without uQ nothing reads it.

| File | Installed to | What it does |
| --- | --- | --- |
| `launcher/emobie.provider` | `/usr/share/uq/launcher/` | Typing `:` in look=see>do_ searches emoji. Enter copies the emoji; Shift+Enter types it into the focused window |
| `dbus/org.emobie.Provider.service` | `/usr/share/dbus-1/services/` | Lets the session bus start `/usr/bin/emobie-provider` when uQ's launcher opens |

| `pages/emobie/` | `/usr/share/uq/settings/pages/emobie/` | The **Emoji and macros** page of uQ Settings (under Apps): macros, deleted macros and settings |

The package also installs `/usr/bin/emobie-provider` (from `crates/emobie-provider`) and
`/usr/share/emobie/emoji.json`, the emoji data the provider searches.

## How it works

`emobie-provider` is a small session-bus service on `org.emobie.Provider` that serves
`org.uq.Launcher1.Provider` at `/org/uq/Launcher1`. It is activatable: uQ asks the bus to start
it when look=see>do_ or the app menu opens (at most once every 30 s), and it exits after 60 s
without a call. The emobie app does not need to be running.

- **Search.** uQ strips the `:` and sends the rest. It is matched against each emoji's name,
  keywords, shortcodes and emoticons, as the app's own search does (`:smile:` and `smile` match
  the same). Favourites rank first, then recents, then the rest by how well they match; the
  skin tone from Settings applies. Up to 20 rows, with the emoji as the title and its name as
  the subtitle.
- **Data.** `/usr/share/emobie/emoji.json`, generated from emojibase by
  `scripts/build-emoji-json.ts` (`npm run emoji:check` in CI keeps it current). The app is built
  from the same file (`src/data/emoji.json`), so both find the same emoji.
- **Preferences.** Favourites, recents and the skin tone come from the app's preferences file
  (`~/.local/share/io.github.asafelobotomy.emobie/emobie-preferences.json`, or the durable copy in
  `~/.local/share/emobie/preferences.json` that a Flatpak install also writes), re-read when it
  changes. Without either file the provider still searches, with no favourites or recents.
- **Copy** (`copy`, Enter) runs `/usr/bin/wl-copy -- <emoji>` with no shell; install
  `wl-clipboard`. The provider gets its environment (`WAYLAND_DISPLAY`) from the session bus's
  activation environment, which uQ's session sets.
- **Type** (`type`, Shift+Enter) copies the emoji, then asks `emobie-inputd` to paste it into the
  window that had focus before look=see>do_. When the helper isn't running it stays a copy.
- **Input checks.** Queries over 256 bytes or with control characters get no rows. `Activate`
  takes only an emoji that is in the data and only the `copy` and `type` actions; anything else
  is `InvalidArgs`.

uQ binds Super+. to open look=see>do_ with `:`. emobie's own summon hotkey stays off while uQ is the
desktop (the app checks whether `org.uq.Desktop1` has an owner on the session bus when it starts),
so the two never fight over a key. Macro hotkeys are unaffected.

## How it was tested

- `uqctl integrations check uq/` is clean.
- `cargo test --manifest-path crates/emobie-provider/Cargo.toml` covers search ranking
  (favourites, recents, match quality, skin tone, the 20-row cap), preferences parsing, input
  checks, copying through a stub `wl-copy` (argv only, shell syntax stays literal), and the
  `inject_paste` request to a fake helper socket.
- `tests/dbus.rs` in that crate runs on a private `dbus-daemon`: `Search` and `Activate` as uQ calls
  them, the error names for bad ids and actions, `type` with and without the helper, and bus
  activation of the real binary from the packaged service file, through to its idle exit.

By hand, with emobie installed:

```sh
busctl --user call org.emobie.Provider /org/uq/Launcher1 org.uq.Launcher1.Provider Search s smile
busctl --user call org.emobie.Provider /org/uq/Launcher1 org.uq.Launcher1.Provider Activate ss 🙂 copy
```

## The Settings page

Settings → Apps → **Emoji and macros** (`pages/emobie`, `uq_api = 3`) has three tabs and shows no emoji:

- **Macros.** Your macros, each with a switch and Delete; a form to add one (a trigger and a
  single-line expansion; hotkeys, fonts and longer text are set in emobie); and where Expand as
  you type stands.
- **Deleted.** Macros you deleted, kept so they can be restored. **Restore** puts one back as it
  was (refused if another macro took its trigger meanwhile); **Delete for good** and **Delete all
  for good** show what goes first. The newest 50 are kept.
- **Settings.** Window, starting and opening (the summon shortcut, autostart), macros and pasting
  (when macros expand, the paste key), the apps where macros never expand, and the palette's size,
  order, skin tone and recents. Auto-paste and Expand as you type are shown, and turned on in
  emobie: they need a one-time permission to type into other apps.

Everything the page reads or changes goes through the `emobie` command line, so it follows the
same preferences file as the window and the two stay in step (the window notices a change by
itself). The page shows only while `/usr/bin/emobie` exists.

## The command line

`emobie` with no command starts the window. These commands answer on the terminal and exit
without opening one; each takes `--json` (a failure is then `{"ok": false, "error": …}` with exit
code 1), and the ones that change something take `--dry-run`, which prints the plan uQ's
confirmation sheet shows (`{summary, changes[{group, label, detail, reversible}], would_refuse,
reason}`).

```
emobie status [--json]                       settings, macros and deleted macros in one document
emobie config show [--json]                  every setting: value, default, type, allowed values
emobie config set <key> <value> [--dry-run]  change a setting (checked like the app checks it)
emobie config add|remove expandExcludedApps <app>
emobie config reset <key> [--dry-run]        back to the default
emobie macro list | deleted [--json]
emobie macro add <trigger> <expansion> [--hotkey <keys>] [--dry-run]
emobie macro set <id|trigger> [--enabled true|false] [--trigger t] [--expansion e] [--hotkey keys]
emobie macro remove <id|trigger> [--dry-run]       moves it to Deleted
emobie macro restore <id|trigger>
emobie macro purge <id|trigger> | --all [--dry-run]  removes deleted macros for good
```

`autoPasteOnCopy` and `expandAsYouType` can't be changed here: turning them on needs a one-time
permission that only the window asks for.
