#!/usr/bin/env bash
# One-time host setup: create emobie-input group, install udev rules, add user.
# Run: pkexec /usr/share/emobie/setup-input-access.sh
#   or: pkexec /usr/local/share/emobie/setup-input-access.sh
#   or: packaging/setup-input-access.sh (self-elevates via pkexec)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GROUP="emobie-input"
SYSTEM_SETUP="/usr/share/emobie/setup-input-access.sh"
LOCAL_SETUP="/usr/local/share/emobie/setup-input-access.sh"
LOCAL_DIR="/usr/local/share/emobie"

acl_package_hint() {
  if command -v apt-get >/dev/null 2>&1; then
    echo "Install setfacl: sudo apt-get install acl"
  elif command -v dnf >/dev/null 2>&1; then
    echo "Install setfacl: sudo dnf install acl"
  elif command -v zypper >/dev/null 2>&1; then
    echo "Install setfacl: sudo zypper install acl"
  elif command -v pacman >/dev/null 2>&1; then
    echo "Install setfacl: sudo pacman -S acl"
  else
    echo "Install the acl package so setfacl can grant immediate keyboard access."
  fi
}

script_path() {
  readlink -f "${BASH_SOURCE[0]}" 2>/dev/null || echo "${BASH_SOURCE[0]}"
}

# Prefer staged/system assets; fall back to user install and repo layout.
if [[ -f /usr/share/emobie/99-emobie-input.rules ]]; then
  RULES_SRC="/usr/share/emobie/99-emobie-input.rules"
elif [[ -f "$SCRIPT_DIR/99-emobie-input.rules" ]]; then
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
  SELF="$(script_path)"
  if [[ "$SELF" == "$(readlink -f "$SYSTEM_SETUP" 2>/dev/null || echo "$SYSTEM_SETUP")" ]]; then
    exec pkexec "$SYSTEM_SETUP" "$@"
  fi
  if [[ "$SELF" == "$(readlink -f "$LOCAL_SETUP" 2>/dev/null || echo "$LOCAL_SETUP")" ]]; then
    exec pkexec "$LOCAL_SETUP" "$@"
  fi
  exec pkexec env SUDO_USER="${SUDO_USER:-$USER}" PKEXEC_UID="${PKEXEC_UID:-$UID}" \
    /usr/bin/bash "$0" "$@"
fi

TARGET_USER="${SUDO_USER:-}"
if [[ -z "$TARGET_USER" && -n "${PKEXEC_UID:-}" ]]; then
  TARGET_USER="$(getent passwd "$PKEXEC_UID" | cut -d: -f1 || true)"
fi
if [[ -z "$TARGET_USER" || "$TARGET_USER" == "root" ]]; then
  echo "Could not determine invoking user. Run: pkexec env SUDO_USER=\$USER $0" >&2
  exit 1
fi
TARGET_UID="$(id -u "$TARGET_USER")"

for cmd in groupadd usermod udevadm install; do
  if ! command -v "$cmd" >/dev/null; then
    echo "Missing required command: $cmd" >&2
    exit 1
  fi
done

# User/AppImage installs: stage a Polkit-annotated copy for future pkexec runs.
SELF="$(script_path)"
if [[ "$SELF" != "$(readlink -f "$SYSTEM_SETUP" 2>/dev/null || echo __none__)" ]]; then
  mkdir -p "$LOCAL_DIR"
  install -m 755 "$0" "$LOCAL_SETUP"
  install -m 644 "$RULES_SRC" "$LOCAL_DIR/99-emobie-input.rules"
  echo "Installed $LOCAL_SETUP for future Grant prompts."
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

if [[ ! -e /dev/uinput ]]; then
  if command -v modprobe >/dev/null; then
    modprobe uinput 2>/dev/null || true
  fi
fi

udevadm control --reload-rules
udevadm trigger --subsystem-match=input || true
udevadm trigger --subsystem-match=misc || true

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
  for uinput in /dev/uinput /dev/input/uinput; do
    if [[ -e "$uinput" ]]; then
      if setfacl -m "u:${TARGET_USER}:rw" "$uinput"; then
        ACL_OK=1
      else
        echo "Warning: setfacl failed for $uinput." >&2
        ACL_OK=0
      fi
    fi
  done
  shopt -u nullglob
else
  acl_package_hint >&2
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

EVENT=""
shopt -s nullglob
for node in /dev/input/event*; do
  EVENT="$node"
  break
done
shopt -u nullglob
if [[ -n "$EVENT" ]]; then
  if sudo -u "$TARGET_USER" test -r "$EVENT"; then
    echo "Verified: $TARGET_USER can read keyboard devices."
  else
    echo "Warning: $TARGET_USER still cannot read $EVENT — log out/in and retry Expand."
  fi
else
  echo "Warning: no /dev/input/event* nodes found yet — replug a keyboard or reboot."
fi

RUNTIME="/run/user/${TARGET_UID}"
try_load_selinux_module() {
  command -v getenforce >/dev/null 2>&1 || return 0
  [[ "$(getenforce 2>/dev/null)" == "Enforcing" ]] || return 0
  local te=""
  for candidate in \
    "$SCRIPT_DIR/selinux/emobie-inputd.te" \
    "$SCRIPT_DIR/../packaging/selinux/emobie-inputd.te" \
    "/usr/share/emobie/selinux/emobie-inputd.te" \
    "$LOCAL_DIR/selinux/emobie-inputd.te"; do
    if [[ -f "$candidate" ]]; then
      te="$candidate"
      break
    fi
  done
  if [[ -z "$te" ]]; then
    echo "SELinux enforcing — optional module at packaging/selinux/emobie-inputd.te"
    return 0
  fi
  if command -v checkmodule >/dev/null && command -v semodule_package >/dev/null; then
    local mod="/tmp/emobie-inputd.mod" pp="/tmp/emobie-inputd.pp"
    if checkmodule -M -m -o "$mod" "$te" && semodule_package -o "$pp" -m "$mod"; then
      if semodule -i "$pp"; then
        echo "Loaded SELinux module for emobie-inputd."
        return 0
      fi
    fi
    echo "Warning: could not compile/load SELinux module — see packaging/selinux/README.md" >&2
  else
    echo "SELinux enforcing — install checkpolicy/policycoreutils-python-utils for auto module load." >&2
  fi
}

try_load_selinux_module

if [[ -d "$RUNTIME" ]] && command -v systemctl >/dev/null; then
  if sudo -u "$TARGET_USER" env XDG_RUNTIME_DIR="$RUNTIME" \
    systemctl --user restart emobie-inputd.service 2>/dev/null; then
    echo "Restarted emobie-inputd for $TARGET_USER."
  fi
fi
