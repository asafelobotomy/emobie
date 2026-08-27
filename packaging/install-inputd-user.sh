#!/usr/bin/env bash
# Install emobie-inputd for the current user and enable the systemd --user unit.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_DIR="${XDG_BIN_HOME:-$HOME/.local/bin}"
UNIT_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
UNIT_NAME="emobie-inputd.service"
BIN_NAME="emobie-inputd"

mkdir -p "$BIN_DIR" "$UNIT_DIR" "${XDG_DATA_HOME:-$HOME/.local/share}/emobie"

echo "Building $BIN_NAME (release)…"
cargo build --release --manifest-path "$ROOT/crates/emobie-inputd/Cargo.toml"
install -m 755 "$ROOT/crates/emobie-inputd/target/release/$BIN_NAME" "$BIN_DIR/$BIN_NAME"
install -m 755 "$ROOT/packaging/setup-input-access.sh" \
  "${XDG_DATA_HOME:-$HOME/.local/share}/emobie/setup-input-access.sh"
install -m 644 "$ROOT/packaging/udev/99-emobie-input.rules" \
  "${XDG_DATA_HOME:-$HOME/.local/share}/emobie/99-emobie-input.rules"
install -m 644 "$ROOT/packaging/polkit/io.github.asafelobotomy.emobie.inputd.policy" \
  "${XDG_DATA_HOME:-$HOME/.local/share}/emobie/io.github.asafelobotomy.emobie.inputd.policy"

# User unit pointing at ~/.local/bin (not /usr/bin).
cat >"$UNIT_DIR/$UNIT_NAME" <<EOF
[Unit]
Description=emobie input helper (text expansion / paste)
Documentation=https://github.com/asafelobotomy/emobie/blob/main/docs/MACROS.md
PartOf=graphical-session.target

[Service]
Type=simple
ExecStart=$BIN_DIR/$BIN_NAME
Restart=on-failure
RestartSec=2
NoNewPrivileges=true

[Install]
WantedBy=default.target
EOF

systemctl --user daemon-reload
systemctl --user enable --now "$UNIT_NAME"

echo "Installed $BIN_DIR/$BIN_NAME"
echo "Enabled systemd --user unit: $UNIT_NAME"
systemctl --user --no-pager --full status "$UNIT_NAME" || true
echo
echo "Socket: \${XDG_RUNTIME_DIR}/emobie/emobie-inputd.sock (mode 0600)."
echo "As-you-type: enable Expand in Settings (one Polkit prompt for keyboard access)."
