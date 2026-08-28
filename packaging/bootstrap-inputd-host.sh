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

mkdir -p "$BIN_DIR" "$UNIT_DIR" "$DATA"
install -m755 "$SOURCE" "$BIN_DIR/emobie-inputd"

for asset in setup-input-access.sh 99-emobie-input.rules io.github.asafelobotomy.emobie.inputd.policy; do
  if [[ -f "$ASSET_DIR/$asset" ]]; then
    install -Dm644 "$ASSET_DIR/$asset" "$DATA/$asset"
  fi
done
if [[ -f "$ASSET_DIR/selinux/emobie-inputd.te" ]]; then
  install -Dm644 "$ASSET_DIR/selinux/emobie-inputd.te" "$DATA/selinux/emobie-inputd.te"
fi

cat >"$UNIT_DIR/emobie-inputd.service" <<EOF
[Unit]
Description=emobie input helper (text expansion / paste)
Documentation=https://github.com/asafelobotomy/emobie/blob/main/docs/MACROS.md
After=graphical-session.target
PartOf=graphical-session.target

[Service]
Type=simple
ExecStart=$BIN_DIR/emobie-inputd
Restart=on-failure
RestartSec=2
NoNewPrivileges=true
PassEnvironment=WAYLAND_DISPLAY DISPLAY XAUTHORITY

[Install]
WantedBy=graphical-session.target
EOF

systemctl --user daemon-reload
systemctl --user enable --now emobie-inputd.service
echo "Installed host emobie-inputd at $BIN_DIR/emobie-inputd"
