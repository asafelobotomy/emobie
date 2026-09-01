#!/usr/bin/env bash
# Install a prebuilt emobie-inputd on the host (no Rust/cargo required).
# Used by AppImage and Flatpak on first Expand / app launch.
set -euo pipefail

SOURCE="${1:?usage: bootstrap-inputd-host.sh /path/to/emobie-inputd}"
[[ -f "$SOURCE" ]] || { echo "Missing binary: $SOURCE" >&2; exit 1; }

BIN_DIR="${XDG_BIN_HOME:-$HOME/.local/bin}"
UNIT_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
DATA="${XDG_DATA_HOME:-$HOME/.local/share}/emobie"
ASSET_DIR="$(dirname "$(readlink -f "$SOURCE")")"
PKG_BIN="/usr/bin/emobie-inputd"
USER_UNIT="$UNIT_DIR/emobie-inputd.service"

mkdir -p "$BIN_DIR" "$UNIT_DIR" "$DATA"
install -m755 "$SOURCE" "$BIN_DIR/emobie-inputd"

for asset in setup-input-access.sh 99-emobie-input.rules io.github.asafelobotomy.emobie.inputd.policy; do
  if [[ -f "$ASSET_DIR/$asset" ]]; then
    if [[ "$asset" == "setup-input-access.sh" ]]; then
      install -Dm755 "$ASSET_DIR/$asset" "$DATA/$asset"
    else
      install -Dm644 "$ASSET_DIR/$asset" "$DATA/$asset"
    fi
  fi
done
if [[ -f "$ASSET_DIR/selinux/emobie-inputd.te" ]]; then
  install -Dm644 "$ASSET_DIR/selinux/emobie-inputd.te" "$DATA/selinux/emobie-inputd.te"
fi

# Prefer the distro package binary when it is present and not older than the
# host helper we just installed, so apt/dnf updates are not shadowed by a
# stale ~/.config/systemd/user unit.
EXEC_START="$BIN_DIR/emobie-inputd"
if [[ -x "$PKG_BIN" ]] && [[ ! "$BIN_DIR/emobie-inputd" -nt "$PKG_BIN" ]]; then
  EXEC_START="$PKG_BIN"
  # Drop the user override so /usr/lib/systemd/user/emobie-inputd.service wins.
  rm -f "$USER_UNIT"
else
  cat >"$USER_UNIT" <<EOF
[Unit]
Description=emobie input helper (text expansion / paste)
Documentation=https://github.com/asafelobotomy/emobie/blob/main/docs/MACROS.md
After=graphical-session.target
PartOf=graphical-session.target

[Service]
Type=simple
ExecStart=$EXEC_START
Restart=on-failure
RestartSec=2
NoNewPrivileges=true
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
fi

systemctl --user daemon-reload
# Drop early default.target linkage from older installs (starts before Wayland).
systemctl --user disable emobie-inputd.service >/dev/null 2>&1 || true
systemctl --user enable emobie-inputd.service
# enable --now does not restart an already-active unit after binary replace.
systemctl --user restart emobie-inputd.service
echo "Installed host emobie-inputd (ExecStart=$EXEC_START)"
