#!/usr/bin/env bash
# One-time host setup: create emobie-input group, install udev rules, add user.
# Run: pkexec /usr/share/emobie/setup-input-access.sh
#   or: pkexec /usr/local/share/emobie/setup-input-access.sh
#   or: packaging/setup-input-access.sh (self-elevates via pkexec)
#
# Idempotent: safe to re-run when the group was deleted, udev rules were removed,
# or listen still works via a temporary ACL / orphaned GID.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GROUP="emobie-input"
SYSTEM_SETUP="/usr/share/emobie/setup-input-access.sh"
LOCAL_SETUP="/usr/local/share/emobie/setup-input-access.sh"
LOCAL_DIR="/usr/local/share/emobie"
RULES_NAME="99-emobie-input.rules"
POLICY_NAME="io.github.asafelobotomy.emobie.inputd.policy"
RULES_DST="/etc/udev/rules.d/${RULES_NAME}"
POLICY_DST="/usr/share/polkit-1/actions/${POLICY_NAME}"

acl_package_hint() {
  if command -v apt-get >/dev/null 2>&1; then
    echo "Install setfacl: pkexec apt-get install acl"
  elif command -v dnf >/dev/null 2>&1; then
    echo "Install setfacl: pkexec dnf install acl"
  elif command -v zypper >/dev/null 2>&1; then
    echo "Install setfacl: pkexec zypper install acl"
  elif command -v pacman >/dev/null 2>&1; then
    echo "Install setfacl: pkexec pacman -S acl"
  else
    echo "Install the acl package so setfacl can grant immediate keyboard access."
  fi
}

script_path() {
  readlink -f "${BASH_SOURCE[0]}" 2>/dev/null || echo "${BASH_SOURCE[0]}"
}

# Resolve udev rules from packaged, staged, or user bootstrap trees.
# $1 = optional username whose ~/.local/share/emobie should be searched.
resolve_rules_src() {
  local user_home=""
  if [[ -n "${1:-}" ]]; then
    user_home="$(getent passwd "$1" | cut -d: -f6 || true)"
  fi
  local candidate
  for candidate in \
    "/usr/share/emobie/${RULES_NAME}" \
    "${LOCAL_DIR}/${RULES_NAME}" \
    "${SCRIPT_DIR}/${RULES_NAME}" \
    "${SCRIPT_DIR}/udev/${RULES_NAME}" \
    "${SCRIPT_DIR}/../packaging/udev/${RULES_NAME}" \
    "${user_home:+$user_home/.local/share/emobie/${RULES_NAME}}"; do
    [[ -n "$candidate" && -f "$candidate" ]] || continue
    printf '%s\n' "$candidate"
    return 0
  done
  return 1
}

resolve_policy_src() {
  local user_home=""
  if [[ -n "${1:-}" ]]; then
    user_home="$(getent passwd "$1" | cut -d: -f6 || true)"
  fi
  # Empty POLICY_SRC means policy already installed system-wide.
  if [[ -f "$POLICY_DST" ]]; then
    return 0
  fi
  local candidate
  for candidate in \
    "${SCRIPT_DIR}/${POLICY_NAME}" \
    "${LOCAL_DIR}/${POLICY_NAME}" \
    "${user_home:+$user_home/.local/share/emobie/${POLICY_NAME}}" \
    "${SCRIPT_DIR}/polkit/${POLICY_NAME}" \
    "${SCRIPT_DIR}/../packaging/polkit/${POLICY_NAME}"; do
    [[ -n "$candidate" && -f "$candidate" ]] || continue
    printf '%s\n' "$candidate"
    return 0
  done
  return 1
}

stage_user_assets_to_local() {
  # Copy setup + siblings into the Polkit-annotated /usr/local tree (needs root).
  local src_script="$1"
  local rules_src="${2:-}"
  local policy_src="${3:-}"
  mkdir -p "$LOCAL_DIR" "$LOCAL_DIR/selinux"
  install -m 755 "$src_script" "$LOCAL_SETUP"
  if [[ -n "$rules_src" && -f "$rules_src" ]]; then
    install -m 644 "$rules_src" "$LOCAL_DIR/${RULES_NAME}"
  fi
  if [[ -n "$policy_src" && -f "$policy_src" ]]; then
    install -m 644 "$policy_src" "$LOCAL_DIR/${POLICY_NAME}"
  fi
  local te=""
  for te in \
    "$(dirname "$src_script")/selinux/emobie-inputd.te" \
    "${HOME:+$HOME/.local/share/emobie/selinux/emobie-inputd.te}"; do
    if [[ -n "$te" && -f "$te" ]]; then
      install -m 644 "$te" "$LOCAL_DIR/selinux/emobie-inputd.te"
      break
    fi
  done
}

if [[ "$(id -u)" -ne 0 ]]; then
  echo "Re-running with pkexec…"
  SELF="$(script_path)"
  INVOKING_USER="${SUDO_USER:-${USER:-}}"
  RULES_FOR_STAGE="$(resolve_rules_src "$INVOKING_USER" || true)"
  POLICY_FOR_STAGE="$(resolve_policy_src "$INVOKING_USER" || true)"

  if [[ "$SELF" == "$(readlink -f "$SYSTEM_SETUP" 2>/dev/null || echo "$SYSTEM_SETUP")" ]]; then
    exec pkexec env SUDO_USER="${INVOKING_USER}" "$SYSTEM_SETUP" "$@"
  fi
  if [[ "$SELF" == "$(readlink -f "$LOCAL_SETUP" 2>/dev/null || echo "$LOCAL_SETUP")" ]]; then
    # Ensure siblings exist beside the annotated script before elevation.
    if [[ -n "$RULES_FOR_STAGE" && ! -f "$LOCAL_DIR/${RULES_NAME}" ]]; then
      pkexec install -D -m 644 "$RULES_FOR_STAGE" "$LOCAL_DIR/${RULES_NAME}"
    fi
    exec pkexec env SUDO_USER="${INVOKING_USER}" "$LOCAL_SETUP" "$@"
  fi

  # Stage full asset set, then run the annotated local script (one Polkit path).
  if [[ -z "$RULES_FOR_STAGE" ]]; then
    echo "Cannot find ${RULES_NAME} beside setup script or under ~/.local/share/emobie" >&2
    exit 1
  fi
  pkexec install -D -m 755 "$SELF" "$LOCAL_SETUP"
  pkexec install -D -m 644 "$RULES_FOR_STAGE" "$LOCAL_DIR/${RULES_NAME}"
  if [[ -n "$POLICY_FOR_STAGE" ]]; then
    pkexec install -D -m 644 "$POLICY_FOR_STAGE" "$LOCAL_DIR/${POLICY_NAME}" || true
  fi
  TE_SRC="$(dirname "$SELF")/selinux/emobie-inputd.te"
  if [[ -f "$TE_SRC" ]]; then
    pkexec install -D -m 644 "$TE_SRC" "$LOCAL_DIR/selinux/emobie-inputd.te" || true
  fi
  exec pkexec env SUDO_USER="${INVOKING_USER}" "$LOCAL_SETUP" "$@"
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
TARGET_GID="$(id -g "$TARGET_USER")"

RULES_SRC="$(resolve_rules_src "$TARGET_USER" || true)"
if [[ -z "$RULES_SRC" ]]; then
  echo "Cannot find ${RULES_NAME} (looked in /usr/share/emobie, /usr/local/share/emobie, and ~${TARGET_USER}/.local/share/emobie)" >&2
  exit 1
fi
POLICY_SRC="$(resolve_policy_src "$TARGET_USER" || true)"

# Run a command as TARGET_USER without depending on sudo (we are already root).
run_as_user() {
  if command -v runuser >/dev/null 2>&1; then
    runuser -u "$TARGET_USER" -- "$@"
  elif command -v setpriv >/dev/null 2>&1; then
    # --init-groups keeps supplementary groups (needed when ACL is absent).
    setpriv --reuid="$TARGET_UID" --regid="$TARGET_GID" --init-groups -- "$@"
  else
    su -s /bin/sh "$TARGET_USER" -c 'exec "$@"' sh "$@"
  fi
}

for cmd in groupadd usermod udevadm install; do
  if ! command -v "$cmd" >/dev/null; then
    echo "Missing required command: $cmd" >&2
    exit 1
  fi
done

# Detect immutable / read-only /etc early with a clear error.
if [[ ! -d /etc/udev/rules.d ]] && ! mkdir -p /etc/udev/rules.d 2>/dev/null; then
  echo "Cannot create /etc/udev/rules.d — this OS may be immutable (Silverblue/Ostree)." >&2
  echo "Layer udev/group packages or unlock /etc before Grant." >&2
  exit 1
fi
if [[ ! -w /etc/udev/rules.d ]]; then
  echo "Cannot write /etc/udev/rules.d — this OS may be immutable (Silverblue/Ostree)." >&2
  echo "Layer udev/group packages or unlock /etc before Grant." >&2
  exit 1
fi

# User/AppImage installs: keep Polkit-annotated tree complete for future Grants.
SELF="$(script_path)"
if [[ "$SELF" != "$(readlink -f "$SYSTEM_SETUP" 2>/dev/null || echo __none__)" ]]; then
  stage_user_assets_to_local "$0" "$RULES_SRC" "${POLICY_SRC:-}"
  echo "Installed $LOCAL_SETUP for future Grant prompts."
fi

if ! getent group "$GROUP" >/dev/null; then
  groupadd --system "$GROUP"
  echo "Created group $GROUP"
else
  echo "Group $GROUP already present."
fi

install -m 644 "$RULES_SRC" "$RULES_DST"
if [[ -n "${POLICY_SRC:-}" ]]; then
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

# Permanent configuration checks (not just ephemeral ACL listen).
if ! getent group "$GROUP" >/dev/null; then
  echo "FAIL: group $GROUP missing after setup." >&2
  exit 1
fi
if [[ ! -f "$RULES_DST" ]]; then
  echo "FAIL: $RULES_DST missing after setup." >&2
  exit 1
fi
if ! id -nG "$TARGET_USER" | tr ' ' '\n' | grep -qx "$GROUP"; then
  echo "Warning: $TARGET_USER not listed in $GROUP yet — log out/in if listen still fails." >&2
fi

EVENT=""
shopt -s nullglob
for node in /dev/input/event*; do
  EVENT="$node"
  break
done
shopt -u nullglob
if [[ -n "$EVENT" ]]; then
  if run_as_user test -r "$EVENT"; then
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
    "$LOCAL_DIR/selinux/emobie-inputd.te" \
    "/usr/share/emobie/selinux/emobie-inputd.te" \
    "$(getent passwd "$TARGET_USER" | cut -d: -f6)/.local/share/emobie/selinux/emobie-inputd.te"; do
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
  if run_as_user env XDG_RUNTIME_DIR="$RUNTIME" \
    systemctl --user restart emobie-inputd.service 2>/dev/null; then
    echo "Restarted emobie-inputd for $TARGET_USER."
  fi
fi
