#!/usr/bin/env bash
# Diagnose text expansion prerequisites on Linux (native or Flatpak host).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
GROUP="emobie-input"
FAIL=0
WARN=0

pass() { echo "OK   $*"; }
warn() { echo "WARN $*"; WARN=$((WARN + 1)); }
fail() { echo "FAIL $*"; FAIL=$((FAIL + 1)); }

echo "emobie text expansion setup check"
echo "================================"

if [[ "$(uname -s)" != "Linux" ]]; then
  fail "Linux only (this script checks /dev/input and systemd user units)."
  exit 1
fi

# --- helper binary ---
if [[ -x /usr/bin/emobie-inputd ]]; then
  pass "Packaged emobie-inputd at /usr/bin/emobie-inputd"
elif [[ -x "${XDG_BIN_HOME:-$HOME/.local/bin}/emobie-inputd" ]]; then
  pass "User emobie-inputd at ${XDG_BIN_HOME:-$HOME/.local/bin}/emobie-inputd"
else
  fail "emobie-inputd not found — install .deb/.rpm or run packaging/install-inputd-user.sh"
fi

# --- systemd user unit ---
if systemctl --user is-active emobie-inputd.service >/dev/null 2>&1; then
  pass "systemd --user emobie-inputd.service is active"
elif systemctl --user cat emobie-inputd.service >/dev/null 2>&1; then
  warn "emobie-inputd unit exists but is not active — run: systemctl --user start emobie-inputd"
else
  fail "No user emobie-inputd.service — run packaging/install-inputd-user.sh or install .deb/.rpm"
fi

# --- socket ---
RUNTIME="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
SOCK="$RUNTIME/emobie/emobie-inputd.sock"
TMP_SOCK="/tmp/emobie-$(id -u)/emobie-inputd.sock"
if [[ -S "$SOCK" ]]; then
  if systemctl --user is-active emobie-inputd.service >/dev/null 2>&1; then
    pass "Socket $SOCK"
  else
    warn "Socket exists but emobie-inputd.service is not active — stale socket? systemctl --user restart emobie-inputd"
  fi
elif [[ -S "$TMP_SOCK" ]]; then
  warn "Socket at $TMP_SOCK (XDG fallback) — prefer $RUNTIME/emobie for Flatpak"
  SOCK="$TMP_SOCK"
else
  warn "Socket missing at $SOCK — start emobie-inputd or open emobie once"
fi

# --- daemon status via socket ---
if [[ -S "$SOCK" ]] && command -v python3 >/dev/null; then
  RESP="$(python3 - <<PY
import json, socket, os
p = ${SOCK@Q}
try:
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.connect(p)
    s.sendall(b'{"cmd":"status"}\n')
    print(s.recv(4096).decode().strip())
except Exception as e:
    print(json.dumps({"ok": False, "detail": str(e)}))
PY
)"
  if echo "$RESP" | grep -q '"can_listen":true'; then
    pass "Daemon reports can_listen (keyboard access)"
  elif echo "$RESP" | grep -q '"can_listen":false'; then
    fail "Daemon cannot read keyboards — run Grant in emobie or: pkexec /usr/share/emobie/setup-input-access.sh"
  fi
  if echo "$RESP" | grep -q '"can_inject":true'; then
    pass "Daemon reports can_inject (compositor/uinput)"
  elif echo "$RESP" | grep -q '"can_inject":false'; then
    warn "Daemon cannot inject text — restart from a graphical session (Wayland/X11 env missing)"
  fi
fi

# --- group membership ---
if getent group "$GROUP" >/dev/null 2>&1; then
  pass "Group $GROUP exists"
else
  fail "Group $GROUP missing — run Grant in emobie or: pkexec /usr/share/emobie/setup-input-access.sh (AppImage: pkexec /usr/local/share/emobie/setup-input-access.sh)"
fi

if id -nG 2>/dev/null | tr ' ' '\n' | grep -qx "$GROUP"; then
  pass "User is in group $GROUP"
else
  if groups 2>/dev/null | grep -Eq '(^|[[:space:]])[0-9]+([[:space:]]|$)'; then
    warn "Session has numeric supplementary GIDs (possible orphaned group) — re-run Grant to recreate $GROUP"
  fi
  warn "User not in $GROUP yet — run Grant or log out/in after setup-input-access.sh"
fi

# --- udev rules ---
if [[ -f /etc/udev/rules.d/99-emobie-input.rules ]]; then
  pass "udev rules installed"
else
  fail "Missing /etc/udev/rules.d/99-emobie-input.rules — run setup-input-access.sh"
fi

# Ephemeral listen must not hide broken permanent config
if [[ -S "$SOCK" ]] && command -v python3 >/dev/null; then
  if echo "${RESP:-}" | grep -q '"can_listen":true'; then
    if ! getent group "$GROUP" >/dev/null 2>&1 || [[ ! -f /etc/udev/rules.d/99-emobie-input.rules ]]; then
      fail "Daemon can_listen is true but permanent group/udev config is incomplete — Expand would break after reboot; run Grant"
    fi
  fi
fi

# --- keyboard device nodes (ignore mice/joysticks after keyboard-only udev) ---
READABLE_KB=0
ANY_KB=0
shopt -s nullglob
for node in /dev/input/event*; do
  if command -v udevadm >/dev/null 2>&1; then
    if ! udevadm info -q property -n "$node" 2>/dev/null | grep -qx 'ID_INPUT_KEYBOARD=1'; then
      continue
    fi
  fi
  ANY_KB=1
  if [[ -r "$node" ]]; then
    READABLE_KB=1
    pass "Can read keyboard $node"
    break
  fi
done
shopt -u nullglob
if [[ "$READABLE_KB" -eq 0 ]]; then
  if [[ "$ANY_KB" -eq 1 ]]; then
    fail "Cannot read keyboard event nodes — run Grant (Polkit + setfacl) or log out/in"
  else
    warn "No keyboard event nodes identified — plug in a keyboard or check udev"
  fi
fi

if [[ -e /dev/uinput ]]; then
  if [[ -w /dev/uinput ]] || getfacl /dev/uinput 2>/dev/null | grep -q "user:$(whoami):"; then
    pass "/dev/uinput accessible"
  else
    warn "/dev/uinput exists but is not writable — paste fallback may fail on some sessions"
  fi
else
  warn "/dev/uinput missing — modprobe uinput (setup script tries this)"
fi

# --- compositor env (for inject) ---
if [[ -n "${WAYLAND_DISPLAY:-}" || -n "${DISPLAY:-}" ]]; then
  pass "Compositor env set (WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-} DISPLAY=${DISPLAY:-})"
elif [[ -S "$RUNTIME/wayland-0" ]]; then
  warn "Wayland socket exists but WAYLAND_DISPLAY unset in this shell — emobie-inputd auto-detects at startup"
else
  warn "No compositor env in this shell — run checks from a desktop session"
fi

# --- setfacl ---
if ! command -v setfacl >/dev/null; then
  warn "setfacl not installed — install acl package for instant access without logout"
fi

# --- Flatpak note ---
if [[ -n "${FLATPAK_ID:-}" ]]; then
  warn "Running inside Flatpak — expansion requires host emobie-inputd + host Grant"
fi

# --- SELinux ---
if command -v getenforce >/dev/null && [[ "$(getenforce 2>/dev/null)" == "Enforcing" ]]; then
  warn "SELinux enforcing — if Grant succeeds but expand fails, check: ausearch -m avc -ts recent | grep emobie"
  if [[ -f "$ROOT/packaging/selinux/emobie-inputd.te" ]]; then
    echo "     Optional module: see packaging/selinux/README.md"
  fi
fi

echo "================================"
if [[ "$FAIL" -gt 0 ]]; then
  echo "$FAIL failure(s), $WARN warning(s). Fix FAIL items then retry Expand in emobie."
  exit 1
fi
if [[ "$WARN" -gt 0 ]]; then
  echo "No hard failures; $WARN warning(s). Expansion may still work."
  exit 0
fi
echo "All checks passed."
exit 0
