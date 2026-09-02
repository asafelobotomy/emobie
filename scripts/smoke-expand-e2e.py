#!/usr/bin/env python3
"""End-to-end Expand smoke: virtual keyboard → inputd match → uinput paste.

Creates a throwaway uinput keyboard (not filtered by emobie-inputd), focuses a
GTK entry, types a space-mode trigger, and asserts the expansion appears.

Requires: GTK 3, /dev/uinput write access, running emobie-inputd with Expand on.
"""
from __future__ import annotations

import fcntl
import json
import os
import struct
import sys
import time
from pathlib import Path

TRIGGER = ".links"
# Must match space-mode trigger (trigger + space) in inputd-state / prefs.
TYPE_CHARS = ".links "

# linux/input.h
EV_KEY = 0x01
EV_SYN = 0x00
SYN_REPORT = 0
UI_SET_EVBIT = 0x40045564
UI_SET_KEYBIT = 0x40045565
UI_DEV_SETUP = 0x405C5503
UI_DEV_CREATE = 0x5501
UI_DEV_DESTROY = 0x5502
BUS_USB = 0x03

# US QWERTY keycodes
KEYMAP = {
    ".": 52,  # KEY_DOT
    "l": 38,
    "i": 23,
    "n": 49,
    "k": 37,
    "s": 31,
    " ": 57,  # KEY_SPACE
}


def socket_path() -> Path:
    runtime = Path(os.environ.get("XDG_RUNTIME_DIR", f"/run/user/{os.getuid()}"))
    return runtime / "emobie" / "emobie-inputd.sock"


def rpc(cmd: dict) -> dict:
    import socket

    path = socket_path()
    if not path.is_socket():
        raise SystemExit(f"FAIL: no helper socket at {path}")
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.settimeout(5)
    s.connect(str(path))
    s.sendall((json.dumps(cmd) + "\n").encode())
    data = s.recv(65536).decode()
    s.close()
    return json.loads(data)


def expected_expansion() -> str:
    state = Path.home() / ".local/share/emobie/inputd-state.json"
    if state.is_file():
        doc = json.loads(state.read_text())
        for m in doc.get("matches") or []:
            if m.get("trigger") == TRIGGER:
                return m.get("expansion") or ""
    # Fallback: preferences macros
    prefs = Path.home() / ".local/share/emobie/preferences.json"
    if prefs.is_file():
        doc = json.loads(prefs.read_text())
        for m in doc.get("macros") or []:
            if m.get("trigger") == TRIGGER:
                return m.get("expansion") or m.get("replace") or ""
    raise SystemExit(f"FAIL: no expansion for trigger {TRIGGER!r}")


class UInputKbd:
    def __init__(self, name: bytes = b"smoke-test-kbd"):
        self.fd = os.open("/dev/uinput", os.O_WRONLY | os.O_NONBLOCK)
        fcntl.ioctl(self.fd, UI_SET_EVBIT, EV_KEY)
        # Advertise a full alphabet so inputd's is_keyboard() accepts the device
        # (requires KEY_A + KEY_Z + KEY_ENTER).
        for code in range(1, 128):
            fcntl.ioctl(self.fd, UI_SET_KEYBIT, code)
        # struct uinput_setup: input_id (4xu16) + name[80] + ff_effects_max u32
        setup = struct.pack(
            "@HHHH80sI",
            BUS_USB,
            0xE6F0,
            0x534D,
            1,
            name.ljust(80, b"\0")[:80],
            0,
        )
        fcntl.ioctl(self.fd, UI_DEV_SETUP, setup)
        fcntl.ioctl(self.fd, UI_DEV_CREATE)
        time.sleep(0.15)

    def emit(self, etype: int, code: int, value: int) -> None:
        t = time.time()
        sec = int(t)
        usec = int((t - sec) * 1_000_000)
        os.write(self.fd, struct.pack("@llHHi", sec, usec, etype, code, value))

    def click(self, code: int) -> None:
        self.emit(EV_KEY, code, 1)
        self.emit(EV_SYN, SYN_REPORT, 0)
        time.sleep(0.02)
        self.emit(EV_KEY, code, 0)
        self.emit(EV_SYN, SYN_REPORT, 0)
        time.sleep(0.02)

    def type_text(self, text: str) -> None:
        for ch in text:
            code = KEYMAP.get(ch)
            if code is None:
                raise SystemExit(f"FAIL: no keycode for {ch!r}")
            self.click(code)

    def close(self) -> None:
        try:
            fcntl.ioctl(self.fd, UI_DEV_DESTROY)
        finally:
            os.close(self.fd)


def main() -> int:
    status = rpc({"cmd": "status"})
    if not status.get("can_listen") or not status.get("can_inject"):
        print("FAIL: helper cannot listen+inject:", status)
        return 1
    if not status.get("enabled", True):
        en = rpc({"cmd": "set_enabled", "enabled": True})
        if not en.get("ok", en.get("enabled")):
            # protocol may return status shape
            pass
        rpc({"cmd": "set_enabled", "enabled": True})

    expansion = expected_expansion()
    if not expansion.strip():
        print("FAIL: empty expansion")
        return 1

    # Ensure match is loaded (space mode adds trailing space in daemon).
    matches = [
        {
            "trigger": TRIGGER,
            "expansion": expansion,
            "mode": "space",
        }
    ]
    sync = rpc({"cmd": "sync_matches", "matches": matches})
    if sync.get("ok") is False:
        print("FAIL: sync_matches:", sync)
        return 1

    import gi

    gi.require_version("Gtk", "3.0")
    from gi.repository import Gtk, GLib

    entry = Gtk.TextView()
    entry.set_wrap_mode(Gtk.WrapMode.WORD_CHAR)
    buf = entry.get_buffer()
    win = Gtk.Window(title="emobie Expand smoke")
    win.set_default_size(520, 200)
    scrolled = Gtk.ScrolledWindow()
    scrolled.add(entry)
    win.add(scrolled)
    win.connect("destroy", Gtk.main_quit)
    win.show_all()
    win.present()

    result: dict[str, str | bool] = {"ok": False, "text": ""}

    def buffer_text() -> str:
        start, end = buf.get_bounds()
        return buf.get_text(start, end, True)

    def run_keys() -> bool:
        # Create the keyboard first, then wait for inputd hotplug (~5s).
        kbd = UInputKbd()
        try:
            time.sleep(5.5)
            entry.grab_focus()
            while Gtk.events_pending():
                Gtk.main_iteration_do(False)
            time.sleep(0.2)
            kbd.type_text(TYPE_CHARS)
            # Erase + paste can take ~1s with clipboard settle.
            time.sleep(2.0)
            while Gtk.events_pending():
                Gtk.main_iteration_do(False)
            text = buffer_text()
            result["text"] = text
            result["ok"] = text.startswith(expansion.rstrip()) or expansion.rstrip() in text
            if not result["ok"]:
                result["ok"] = expansion[:20] in text
        finally:
            kbd.close()
        Gtk.main_quit()
        return False

    GLib.timeout_add(200, run_keys)
    Gtk.main()

    if result["ok"]:
        print("PASS: Expand E2E — expansion appeared in focused field")
        print("got:", repr(result["text"][:120]))
        return 0
    print("FAIL: Expand E2E — expected expansion missing")
    print("expected prefix:", repr(expansion[:80]))
    print("got:", repr(result["text"]))
    return 1


if __name__ == "__main__":
    sys.exit(main())
