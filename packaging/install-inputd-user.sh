#!/usr/bin/env bash
# Install emobie-inputd for the current user and enable the systemd --user unit.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_DIR="${XDG_BIN_HOME:-$HOME/.local/bin}"
UNIT_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
UNIT_NAME="emobie-inputd.service"
BIN_NAME="emobie-inputd"

mkdir -p "$BIN_DIR" "$UNIT_DIR" "${XDG_DATA_HOME:-$HOME/.local/share}/emobie"

if ! command -v cargo >/dev/null; then
  echo "cargo is required to build emobie-inputd. Install Rust from https://rustup.rs" >&2
  exit 1
fi
if ! command -v setfacl >/dev/null; then
  echo "Note: install the acl package so Grant can apply session ACLs without logout." >&2
  if command -v pacman >/dev/null; then
    echo "  pkexec pacman -S acl" >&2
  elif command -v apt-get >/dev/null; then
    echo "  pkexec apt-get install acl" >&2
  elif command -v dnf >/dev/null; then
    echo "  pkexec dnf install acl" >&2
  elif command -v zypper >/dev/null; then
    echo "  pkexec zypper install acl" >&2
  fi
fi

echo "Building $BIN_NAME (release)…"
cargo build --release --manifest-path "$ROOT/crates/emobie-inputd/Cargo.toml"
install -m 755 "$ROOT/crates/emobie-inputd/target/release/$BIN_NAME" "$BIN_DIR/$BIN_NAME"
install -m 755 "$ROOT/packaging/setup-input-access.sh" \
  "${XDG_DATA_HOME:-$HOME/.local/share}/emobie/setup-input-access.sh"
install -m 644 "$ROOT/packaging/udev/99-emobie-input.rules" \
  "${XDG_DATA_HOME:-$HOME/.local/share}/emobie/99-emobie-input.rules"
install -m 644 "$ROOT/packaging/polkit/io.github.asafelobotomy.emobie.inputd.policy" \
  "${XDG_DATA_HOME:-$HOME/.local/share}/emobie/io.github.asafelobotomy.emobie.inputd.policy"

# User unit pointing at ~/.local/bin (dev / from-source install).
# Distro packages use /usr/lib/systemd/user + /usr/bin; bootstrap prefers the
# packaged binary when it is present and not older than the host helper.
cat >"$UNIT_DIR/$UNIT_NAME" <<EOF
[Unit]
Description=emobie input helper (text expansion / paste)
Documentation=https://github.com/asafelobotomy/emobie/blob/main/docs/MACROS.md
After=graphical-session.target
PartOf=graphical-session.target

[Service]
Type=simple
ExecStart=$BIN_DIR/$BIN_NAME
Restart=on-failure
RestartSec=2
NoNewPrivileges=true
# Inherit compositor env when the user manager has it; daemon also auto-detects
# $XDG_RUNTIME_DIR/wayland-0 when these are missing (common on Plasma Wayland).
PassEnvironment=WAYLAND_DISPLAY DISPLAY XAUTHORITY XDG_RUNTIME_DIR XKB_DEFAULT_LAYOUT XKB_DEFAULT_MODEL XKB_DEFAULT_VARIANT XKB_DEFAULT_OPTIONS
UMask=0077
RuntimeDirectory=emobie
RuntimeDirectoryMode=0700
PrivateDevices=no
PrivateNetwork=yes
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths=%h/.local/share/emobie
RestrictAddressFamilies=AF_UNIX
RestrictNamespaces=yes
ProtectKernelTunables=yes
ProtectKernelModules=yes
ProtectControlGroups=yes
LockPersonality=yes
RestrictRealtime=yes
RestrictSUIDSGID=yes

[Install]
WantedBy=graphical-session.target
EOF

systemctl --user daemon-reload
# Older installs also linked default.target (starts before Wayland).
systemctl --user disable "$UNIT_NAME" >/dev/null 2>&1 || true
systemctl --user enable "$UNIT_NAME"
# enable --now does not restart an already-active unit after binary replace.
systemctl --user restart "$UNIT_NAME"

echo "Installed $BIN_DIR/$BIN_NAME"
echo "Enabled systemd --user unit: $UNIT_NAME"
systemctl --user --no-pager --full status "$UNIT_NAME" || true
echo
echo "Socket: \${XDG_RUNTIME_DIR}/emobie/emobie-inputd.sock (mode 0600)."
echo "As-you-type: enable Expand in Settings (one Polkit prompt for keyboard access)."
echo "Verify setup: bash scripts/verify-expand-setup.sh"
