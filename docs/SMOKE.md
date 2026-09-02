# Emobie smoke checklist

Run before a release or after Expand/inputd changes. Mark each item Pass / Fail / Skip.

Prefs: `~/.local/share/emobie/preferences.json`  
Helper socket: `$XDG_RUNTIME_DIR/emobie/emobie-inputd.sock`  
Default summon: `Ctrl+Shift+Space`

## A — Automated gate (CI-equivalent)

| ID | Check | Command | Pass if |
|----|-------|---------|---------|
| A1 | Frontend unit tests | `npm test` | All green |
| A2 | inputd unit tests | `npm run test:inputd` | All green |
| A3 | Frontend build | `npm run build` | `tsc` + Vite OK |
| A4 | Tauri compile | `cargo check --manifest-path src-tauri/Cargo.toml` | OK |
| A5 | Packaging maps | `bash scripts/check-inputd-packaging.sh` | OK |
| A6 | LOC gate | `bash scripts/check-loc.sh` | OK |
| A7 | Expand host diagnose | `npm run verify:expand` | Exit 0 (Wayland: uinput writable) |

## B — Helper / Expand runtime

| ID | Check | How | Pass if |
|----|-------|-----|---------|
| B1 | Helper running | `systemctl --user is-active emobie-inputd.service` | `active` |
| B2 | Status RPC | `{"cmd":"status"}` on socket | `can_listen` + `can_inject` |
| B3 | Matches synced | Status / `inputd-state.json` | Expected triggers present |
| B4 | Expand E2E | `python3 scripts/smoke-expand-e2e.py` | Expansion text appears in focused field |
| B5 | Cold expand | Wait >45s idle, run B4 again | First fire pastes (no empty erase) |
| B6 | Disable expand | SetEnabled false; type trigger | No expansion |
| B7 | Re-enable | SetEnabled true; B4 | Works again |

## C — App shell (manual / AppImage)

| ID | Check | How | Pass if |
|----|-------|-----|---------|
| C1 | Launch | AppImage / package / `npm run tauri dev` | Window + brand **emobie** |
| C2 | Hide to tray | Toolbar close | Hides; process alive |
| C3 | Tray restore | Tray left-click / **Show emobie** | Window shows |
| C4 | Summon hotkey | `Ctrl+Shift+Space` | Toggle show/hide |
| C5 | Quit | Tray **Quit** / Preferences **Quit** | Process exits |

## D — Emoji / clipboard

| ID | Check | How | Pass if |
|----|-------|-----|---------|
| D1 | Browse categories | Click tabs | Grid updates |
| D2 | Search | Type `smile` | Filtered results |
| D3 | Copy emoji | Click glyph | Status **Copied**; paste works |
| D4 | Favorite | Right-click emoji | Appears under Favorites |
| D5 | Recents | Copy a few | **Rec** strip updates |
| D6 | Auto-paste | Pref on; unpinned; copy into another app | Pastes into previous focus |

## E — Macros

| ID | Check | How | Pass if |
|----|-------|-----|---------|
| E1 | Add macro | Macros → **+** → Save | Card shows trigger + expansion |
| E2 | Macro copy | Click card | Expansion on clipboard |
| E3 | Export / import YAML | Preferences → Export then Import | Round-trip OK |
| E4 | Macro hotkey | Assign unique hotkey; press outside app | Fires copy/paste |

## F — Settings

| ID | Check | How | Pass if |
|----|-------|-----|---------|
| F1 | Theme | System / Light / Dark | Appearance changes |
| F2 | Expand toggle | On/off + Grant if prompted | Status matches; listen syncs |
| F3 | Expand modes | After Space vs immediate | Behavior matches |
| F4 | Autostart | Toggle **Launch on startup** | Desktop file / portal updates |
| F5 | Update check | **Check for updates** | Banner or up-to-date (no crash) |

## Suggested order

1. `npm run smoke:gate` (section A)  
2. `npm run smoke:expand` (B4; optional cold: wait 50s and rerun)  
3. Launch app → C, D, E, F as needed  

### npm scripts

| Script | Runs |
|--------|------|
| `npm run smoke:gate` | A1–A6 + verify:expand |
| `npm run smoke:expand` | B4 Expand E2E (GTK TextView + virtual keyboard) |
| `npm run verify:expand` | Host diagnose only |

## Results — 2026-09-02

| ID | Status | Notes |
|----|--------|-------|
| A1–A5, A7 | Pass | |
| A6 | Pass after fix | Was failing 6 files >400 LOC; modules split |
| B1–B5, B6–B7 | Pass | Cold expand OK after 50s idle |
| C/D/F UI | Manual | AppImage was running; tray/summon not automated |
| E YAML | Pass | macroYaml round-trip |

See canvas `emobie-smoke-2026-09-02` for the live board.
