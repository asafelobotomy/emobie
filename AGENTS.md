# Agent notes for emobie

A compact emoji palette with macros for Linux. Tauri 2: a React and TypeScript frontend (`src/`) over a Rust app (`src-tauri/`), plus two separate Rust crates: `crates/emobie-inputd` (host helper that pastes and reads the keyboard) and `crates/emobie-provider` (emoji search for uQ's launcher). `uq/` holds the uQ desktop integration (see `uq/README.md`). Platform notes are in `docs/LINUX.md` and `docs/MACROS.md`.

## Verify

There is no Cargo workspace: three crates, three lockfiles. Every cargo command needs `--manifest-path` (or run it inside the crate's folder). Node 22.6+ is required (`.nvmrc`); the tests use `--experimental-strip-types`.

```bash
npm ci
npm run smoke:gate        # the local gate: every check below except clippy, npm audit, the Flatpak source check and the uQ page tests
npm test                  # frontend unit tests (src/lib/*.test.ts)
npm run build             # tsc + vite build
npm run test:tauri        # cargo test, src-tauri
npm run test:inputd       # cargo test, crates/emobie-inputd
npm run test:provider     # cargo test, crates/emobie-provider (starts a private dbus-daemon)
npm run emoji:check       # src/data/emoji.json matches emojibase
npm run check:uq-tokens   # vendored uq-tokens is the pinned version
npm run check:acl         # every Tauri command is allowed by the ACL
bash scripts/check-loc.sh
bash scripts/check-inputd-packaging.sh
```

CI (`.github/workflows/ci.yml`) also runs these, which `smoke:gate` leaves out:

```bash
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo clippy --manifest-path crates/emobie-inputd/Cargo.toml --all-targets -- -D warnings
cargo clippy --manifest-path crates/emobie-provider/Cargo.toml --all-targets -- -D warnings
npm audit --omit=dev --audit-level=high
python3 scripts/flatpak-cargo-sources.py --check flatpak/cargo-sources.json src-tauri/Cargo.lock crates/emobie-inputd/Cargo.lock
node --test tests/uq/*.test.mjs
QML_XHR_ALLOW_FILE_READ=1 QT_QPA_PLATFORM=offscreen /usr/lib/qt6/bin/qmltestrunner -import tests/uq/stub -input tests/uq
uqctl integrations check --no-programs uq/
```

Run the app with `npm run tauri dev`. Package with `npm run tauri:build`, never bare `tauri build`: the script sets `NO_STRIP=true APPIMAGE_EXTRACT_AND_RUN=1` (needed on Arch) and runs `scripts/fix-linux-bundle-icons.sh` afterwards. `docs/SMOKE.md` is the manual checklist for a release or after Expand/inputd changes.

## Conventions the checks enforce

- No source file over 400 lines: `.ts`, `.tsx` and `.css` under `src/`, and `.rs` under `src-tauri/src` and `crates/`. Split the file instead of raising the limit.
- A new Tauri command must be registered in `generate_handler!` in `src-tauri/src/lib.rs` and allowed in `src-tauri/permissions/*.toml` and `capabilities/default.json`. Without the ACL entry it compiles and then fails at runtime with "Command X not allowed by ACL".
- A dependency change in `src-tauri` or `crates/emobie-inputd` makes `flatpak/cargo-sources.json` stale. Regenerate it with `python3 scripts/flatpak-cargo-sources.py -o flatpak/cargo-sources.json src-tauri/Cargo.lock crates/emobie-inputd/Cargo.lock` (`scripts/generate-flatpak-sources.sh` also redoes the npm sources and needs `flatpak-node-generator`).

## Generated and vendored files

- `src/data/emoji.json` is generated and committed. After bumping `emojibase-data` or editing `src/data/extraEmoticons.ts`, run `npm run emoji:build`.
- `src/vendor/uq-tokens/uq-tokens.js` is copied unchanged from uQ's `web/uq-tokens`. To update it, copy the new file over and bump the version in the `check:uq-tokens` script in `package.json`. `uq-tokens.css` is emobie's own.
- `tests/uq/fixtures` are the real output of the emobie CLI. When that output changes, run `UPDATE_FIXTURES=1 npm run test:tauri` and commit the result.
- `src-tauri/inputd-bundle/` is build output staged by `scripts/stage-inputd.sh` (git-ignored). `flatpak/shared-modules` is a git submodule.

## Release

- The version lives in ten files that the 0.9.0 release commit (`c30aef2`) shows: `package.json`, `package-lock.json`, `src-tauri/tauri.conf.json`, `Cargo.toml` and `Cargo.lock` in each of `src-tauri`, `crates/emobie-inputd` and `crates/emobie-provider`, and `pkgver` in `packaging/arch/PKGBUILD`. Also add a `<release>` entry to `flatpak/io.github.asafelobotomy.emobie.metainfo.xml` and move the `[Unreleased]` notes in `CHANGELOG.md` under `## [X.Y.Z] - YYYY-MM-DD` (the release notes are that section).
- `release.yml` runs on a `vX.Y.Z` tag and fails if the tag, `package.json`, `src-tauri/Cargo.toml` and `tauri.conf.json` disagree. It does not check the other files, so a missed one ships silently.
- The workflow signs `SHA256SUMS` with minisign and fails at "Sign checksums" without the `MINISIGN_SECRET_KEY` secret. The in-app updater only trusts releases signed by the key in `src-tauri/update-signing.pub`. Never generate, replace or rotate that key yourself; `docs/RELEASING.md` has the procedure and it is the user's to run.
- emobie is not on the AUR. `packaging/arch/PKGBUILD` is a starting point that builds from GitHub's `/archive/` tarball, so its `sha256sums` can only be updated after the tag exists; a release commit that only bumps `pkgver` leaves the checksum stale. There is no `.SRCINFO` and nothing to push after a release until the user decides to publish it.
- Flathub is deferred until the checklist in `docs/FLATHUB.md` is green. The release ships a Flatpak bundle built from the `.deb` (`flatpak/io.github.asafelobotomy.emobie.deb.yml`).

## Gotchas

- `emobie-inputd` injects keystrokes and, for Expand, reads the keyboard. `packaging/setup-input-access.sh`, `install-inputd-user.sh` and `bootstrap-inputd-host.sh` change groups, udev rules and systemd units on the host: don't run them to test a change. `npm run verify:expand` only diagnoses.
- emobie is single-instance: a dev build only raises the installed window while that is running. Quit the installed app first.
- Under uQ, emobie's own summon hotkey stays off (it checks for `org.uq.Desktop1` at start) and the theme follows uQ. The Flatpak build sees neither, so behaviour differs between native and Flatpak; say which one you tested.
- The `uq` CI job checks the Settings page against a pinned uQ release (`uqctl` built from tag `v0.3.0`, `uq_api 3`). Bump that pin in `ci.yml` when the page starts using a newer uQ API.
- `deny.toml` ignores named advisories with the reason in a comment. Remove an entry when its cause is gone.
