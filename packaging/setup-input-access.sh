#!/usr/bin/env bash
# One-time host setup: create emobie-input group, install udev rules, add user.
# Run: pkexec /usr/share/emobie/setup-input-access.sh
#   or: packaging/setup-input-access.sh (self-elevates via pkexec)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GROUP="emobie-input"

# Prefer staged/system assets; fall back to repo layout when run from source.
if [[ -f /usr/share/emobie/99-emobie-input.rules ]]; then
  RULES_SRC="/usr/share/emobie/99-emobie-input.rules"
elif [[ -f "$SCRIPT_DIR/udev/99-emobie-input.rules" ]]; then
  RULES_SRC="$SCRIPT_DIR/udev/99-emobie-input.rules"
elif [[ -f "$SCRIPT_DIR/../packaging/udev/99-emobie-input.rules" ]]; then
  RULES_SRC="$SCRIPT_DIR/../packaging/udev/99-emobie-input.rules"
else
  echo "Cannot find 99-emobie-input.rules" >&2
  exit 1
fi

RULES_DST="/etc/udev/rules.d/99-emobie-input.rules"
POLICY_DST="/usr/share/polkit-1/actions/io.github.asafelobotomy.emobie.inputd.policy"
POLICY_SRC=""
if [[ -f /usr/share/polkit-1/actions/io.github.asafelobotomy.emobie.inputd.policy ]]; then
  POLICY_SRC="" # already installed by package
elif [[ -f "$SCRIPT_DIR/polkit/io.github.asafelobotomy.emobie.inputd.policy" ]]; then
  POLICY_SRC="$SCRIPT_DIR/polkit/io.github.asafelobotomy.emobie.inputd.policy"
elif [[ -f "$SCRIPT_DIR/../packaging/polkit/io.github.asafelobotomy.emobie.inputd.policy" ]]; then
  POLICY_SRC="$SCRIPT_DIR/../packaging/polkit/io.github.asafelobotomy.emobie.inputd.policy"
fi

if [[ "$(id -u)" -ne 0 ]]; then
  echo "Re-running with pkexec…"
  exec pkexec env \
    "SUDO_USER=${SUDO_USER:-$USER}" \
    "PKEXEC_UID=${PKEXEC_UID:-$(id -u)}" \
    bash "$0" "$@"
fi

TARGET_USER="${SUDO_USER:-}"
if [[ -z "$TARGET_USER" && -n "${PKEXEC_UID:-}" ]]; then
  TARGET_USER="$(getent passwd "$PKEXEC_UID" | cut -d: -f1 || true)"
fi
if [[ -z "$TARGET_USER" || "$TARGET_USER" == "root" ]]; then
  echo "Could not determine invoking user. Run: pkexec env SUDO_USER=\$USER $0" >&2
  exit 1
fi

if ! getent group "$GROUP" >/dev/null; then
  groupadd --system "$GROUP"
  echo "Created group $GROUP"
fi

install -m 644 "$RULES_SRC" "$RULES_DST"
if [[ -n "$POLICY_SRC" && -d "$(dirname "$POLICY_DST")" ]]; then
  install -m 644 "$POLICY_SRC" "$POLICY_DST"
fi

usermod -aG "$GROUP" "$TARGET_USER"
udevadm control --reload-rules
udevadm trigger --subsystem-match=input || true

echo "Added $TARGET_USER to $GROUP and installed udev rules."
echo "Log out and back in (or reboot) before expand-as-you-type can read keyboards."
echo "Membership in $GROUP is sensitive — it grants keyboard event access."
