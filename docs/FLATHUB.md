# Flathub readiness (deferred)

emobie is **not** submitted to Flathub yet. This document tracks the maturity runway
so a future, **human-owned** submission can proceed only when every gate is green.

Official process: [Submission](https://docs.flathub.org/docs/for-app-authors/submission) ·
[Requirements](https://docs.flathub.org/docs/for-app-authors/requirements) ·
[Quality guidelines](https://docs.flathub.org/docs/for-app-authors/metainfo-guidelines/quality-guidelines)

## Packaging layout

| File | Role |
|------|------|
| [`flatpak/io.github.asafelobotomy.emobie.yml`](../flatpak/io.github.asafelobotomy.emobie.yml) | **Flathub-bound** offline source build (cargo + npm) |
| [`flatpak/io.github.asafelobotomy.emobie.deb.yml`](../flatpak/io.github.asafelobotomy.emobie.deb.yml) | GitHub Releases Flatpak (unwraps `.deb`) |
| [`flatpak/cargo-sources.json`](../flatpak/cargo-sources.json) / [`node-sources.json`](../flatpak/node-sources.json) | Offline dependency manifests |
| [`flatpak/shared-modules`](../flatpak/shared-modules) | AppIndicator shared module (git submodule) |

Regenerate dependency manifests after lockfile changes:

```bash
# needs flatpak-builder-tools + flatpak-node-generator on PATH
./scripts/generate-flatpak-sources.sh
```

Then bump `tag` / `commit` in the source manifest to the release being packaged.

## Remaining before Flathub

1. **Autostart portal:** Source Flatpak finish-args omit `xdg-config/autostart` (linter rejects it). [`src-tauri/src/autostart.rs`](../src-tauri/src/autostart.rs) now prefers the XDG **Background** portal under Flatpak (with desktop-file fallback when the sandbox can write autostart). Re-verify portal UX on GNOME/KDE before submission.
2. **Tray own-name:** Source Flatpak does not request `--own-name=org.kde.StatusNotifierItem.*` (Flathub rejects wildcards). emobie disables dbus name ownership inside Flatpak (`ksni::disable_dbus_name`) so Cinnamon/Mint `xapp-sn-watcher` can still host the icon. Confirm tray on Mint with System Tray applet enabled; on GNOME confirm an AppIndicator extension.
3. **Screenshot refresh:** Re-capture without transient “tray unavailable” banners once tray is solid in the packaging under test.
4. **Input helper:** Macros UI ships in Flatpak; as-you-type / auto-paste need host `emobie-inputd` via `--filesystem=xdg-run/emobie` (no `--device=input`). See [`docs/MACROS.md`](MACROS.md).
5. **Sustained releases / human PR:** See checklist below.

## Local validation

```bash
git submodule update --init flatpak/shared-modules

flatpak install -y --user flathub org.flatpak.Builder \
  org.gnome.Sdk//50 \
  org.freedesktop.Sdk.Extension.rust-stable//25.08 \
  org.freedesktop.Sdk.Extension.node22//25.08

flatpak run --command=flatpak-builder-lint org.flatpak.Builder \
  manifest flatpak/io.github.asafelobotomy.emobie.yml

# Full offline source build (slow; uses flathub-build helper):
flatpak run --command=flathub-build --filesystem="$(pwd)" --filesystem=/tmp \
  --share=network org.flatpak.Builder \
  --install flatpak/io.github.asafelobotomy.emobie.yml

flatpak run io.github.asafelobotomy.emobie
```

Smoke after install: tray show/hide, hotkey, copy. Launch-on-startup needs Autostart portal work (see above) before Flathub.

## Deferred submission checklist

Open a Flathub PR **only when all are true**:

- [x] Source-build Flatpak builds offline and runs (`flathub-build` + smoke) — proven locally on 2026-08-07
- [ ] `flatpak-builder-lint` clean on **repo** after screenshots are on `main` (manifest lint already clean)
- [ ] Real screenshots without transient error banners; AppStream validates against live URLs
- [ ] Autostart portal verified on GNOME/KDE Flatpak; tray confirmed without StatusNotifierItem `--own-name`
- [ ] Multiple stable tagged releases with a non-trivial maintenance window since first public release
- [ ] Maintainer ready to open and drive the PR **without** AI-authored submission text
- [ ] Regenerated `node-sources.json` / `cargo-sources.json` match the release tag in the manifest

## Policy notes

- Flathub requires OSS apps to **build from source** (this repo’s source manifest).
- Flathub disallows AI-authored submission PRs and generally AI-generated apps; exceptions may apply to mature, well-maintained projects. Do not scrub history—sustain human maintenance and answer reviewers honestly if asked.
- When ready: fork `flathub/flathub`, branch from `new-pr`, PR title `Add io.github.asafelobotomy.emobie`. **Do not** have an agent open or write that PR.

## Screenshot refresh

Screenshots live in [`branding/screenshots/`](../branding/screenshots/). Re-capture from a normal desktop session (tray working, default theme for the primary shot) before submission if UI or status banners change.
