#!/usr/bin/env python3
"""Generate flatpak/cargo-sources.json from every Cargo.lock the Flatpak builds.

The Flathub-bound manifest builds *two* crates offline (src-tauri and
crates/emobie-inputd), so the vendored set must cover both lockfiles; the stock
flatpak-cargo-generator only takes one. Output format matches that tool for
registry crates (archive + .cargo-checksum.json + a source-replacement config).

Works on any Python 3.\n\nUsage: flatpak-cargo-sources.py [-o OUT] LOCK [LOCK...]
       flatpak-cargo-sources.py --check OUT LOCK [LOCK...]   # exit 1 if stale
"""
import json
import re
import sys

CRATES_IO = "registry+https://github.com/rust-lang/crates.io-index"
CONFIG = (
    '[source.vendored-sources]\ndirectory = "cargo/vendor"\n\n'
    '[source.crates-io]\nreplace-with = "vendored-sources"\n'
)


_KV = re.compile(r'^(name|version|source|checksum) = "([^"]*)"$')


def read_packages(path):
    """Parse `[[package]]` tables from a Cargo.lock.

    Cargo.lock is machine-written with one `key = "value"` per line, so a tiny
    parser is enough and keeps this script dependency-free (the CI runner's
    Python 3.10 has no `tomllib`).
    """
    packages, current = [], None
    with open(path, encoding="utf-8") as fh:
        for raw in fh:
            line = raw.rstrip("\n")
            if line == "[[package]]":
                current = {}
                packages.append(current)
            elif line.startswith("["):
                current = None  # [metadata] etc.
            elif current is not None:
                m = _KV.match(line)
                if m:
                    current[m.group(1)] = m.group(2)
    return packages


def crates_from(lock_paths):
    found = {}
    for path in lock_paths:
        for pkg in read_packages(path):
            source = pkg.get("source")
            if source is None:
                continue  # workspace member / the crate itself
            if source != CRATES_IO:
                sys.exit(f"{path}: {pkg['name']} uses unsupported source {source!r}")
            checksum = pkg.get("checksum")
            if not checksum:
                sys.exit(f"{path}: {pkg['name']} {pkg['version']} has no checksum")
            found[(pkg["name"], pkg["version"])] = checksum
    return found


def build(lock_paths):
    entries = []
    for (name, version), checksum in sorted(crates_from(lock_paths).items()):
        dest = f"cargo/vendor/{name}-{version}"
        entries.append(
            {
                "type": "archive",
                "archive-type": "tar-gzip",
                "url": f"https://static.crates.io/crates/{name}/{name}-{version}.crate",
                "sha256": checksum,
                "dest": dest,
            }
        )
        entries.append(
            {
                "type": "inline",
                "contents": json.dumps({"package": checksum, "files": {}}),
                "dest": dest,
                "dest-filename": ".cargo-checksum.json",
            }
        )
    entries.append(
        {"type": "inline", "contents": CONFIG, "dest": "cargo", "dest-filename": "config"}
    )
    return json.dumps(entries, indent=4) + "\n"


def main(argv):
    if argv and argv[0] == "--check":
        out, locks = argv[1], argv[2:]
        current = open(out).read()
        if current != build(locks):
            sys.exit(f"{out} is stale — run scripts/generate-flatpak-sources.sh")
        print(f"{out} is up to date")
        return
    out = None
    if argv[:1] == ["-o"]:
        out, argv = argv[1], argv[2:]
    text = build(argv)
    if out:
        with open(out, "w") as fh:
            fh.write(text)
    else:
        sys.stdout.write(text)


if __name__ == "__main__":
    main(sys.argv[1:])
