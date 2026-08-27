#!/usr/bin/env bash
# One-time host setup: create emobie-input group, install udev rules, add user.
# Run: pkexec /usr/share/emobie/setup-input-access.sh
#   or: packaging/setup-input-access.sh (self-elevates via pkexec)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GROUP="emobie-input"
SYSTEM_SETUP="/usr/share/emobie/setup-input-access.sh"

# Prefer staged/system assets; fall back to user install and repo layout.
if [[ -f /usr/share/emobie/99-emobie-input.rules ]]; then
  RULES_SRC="/usr/share/emobie/99-emobie-input.rules"
elif [[ -f "$SCRIPT_DIR/99-emobie-input.rules" ]]; then
  # packaging/install-inputd-user.sh stages rules next to this script.
  RULES_SRC="$SCRIPT_DIR/99-emobie-input.rules"
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
elif [[ -f "$SCRIPT_DIR/io.github.asafelobotomy.emobie.inputd.policy" ]]; then
  POLICY_SRC="$SCRIPT_DIR/io.github.asafelobotomy.emobie.inputd.policy"
elif [[ -f "$SCRIPT_DIR/polkit/io.github.asafelobotomy.emobie.inputd.policy" ]]; then
  POLICY_SRC="$SCRIPT_DIR/polkit/io.github.asafelobotomy.emobie.inputd.policy"
elif [[ -f "$SCRIPT_DIR/../packaging/polkit/io.github.asafelobotomy.emobie.inputd.policy" ]]; then
  POLICY_SRC="$SCRIPT_DIR/../packaging/polkit/io.github.asafelobotomy.emobie.inputd.policy"
fi

if [[ "$(id -u)" -ne 0 ]]; then
  echo "Re-running with pkexec…"
  # Match Polkit annotate exec.path for the packaged script.
  if [[ "$(readlink -f "$0" 2>/dev/null || echo "$0")" == "$(readlink -f "$SYSTEM_SETUP" 2>/dev/null || echo "$SYSTEM_SETUP")" ]]; then
    exec pkexec "$SYSTEM_SETUP" "$@"
  fi
  exec pkexec /usr/bin/bash "$0" "$@"
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
if [[ -n "$POLICY_SRC" ]]; then
  if [[ ! -d "$(dirname "$POLICY_DST")" ]]; then
    echo "Polkit actions directory missing; cannot install policy." >&2
    exit 1
  fi
  install -m 644 "$POLICY_SRC" "$POLICY_DST"
fi

usermod -aG "$GROUP" "$TARGET_USER"
udevadm control --reload-rules
udevadm trigger --subsystem-match=input || true

ACL_OK=0
ACL_TRIED=0
if command -v setfacl >/dev/null; then
  ACL_TRIED=1
  shopt -s nullglob
  events=(/dev/input/event*)
  if ((${#events[@]})); then
    if setfacl -m "u:${TARGET_USER}:r" "${events[@]}"; then
      ACL_OK=1
    else
      echo "Warning: setfacl failed for /dev/input/event* — logout may be required." >&2
    fi
  fi
  if [[ -e /dev/uinput ]]; then
    if ! setfacl -m "u:${TARGET_USER}:rw" /dev/uinput; then
      echo "Warning: setfacl failed for /dev/uinput." >&2
      ACL_OK=0
    fi
  fi
  shopt -u nullglob
fi

echo "Added $TARGET_USER to $GROUP and installed udev rules."
if [[ "$ACL_TRIED" -eq 1 && "$ACL_OK" -eq 1 ]]; then
  echo "Session ACLs applied — restart emobie-inputd (or toggle Expand) without logging out."
elif [[ "$ACL_TRIED" -eq 1 ]]; then
  echo "Session ACLs were not fully applied — log out/in so group $GROUP takes effect."
else
  echo "setfacl unavailable — log out/in so new sessions inherit $GROUP."
fi
echo "Membership in $GROUP is sensitive — it grants keyboard event access."
