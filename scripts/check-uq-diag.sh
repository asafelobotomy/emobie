#!/usr/bin/env bash
# check-uq-diag.sh <pinned-version> <vendored file>...
# Drift check for vendored uq-diag files (uq_diag.rs, uqdiag.h, uqdiag.cpp): each file's
# `uq-diag X.Y.Z` header must be the pinned version and its contents exactly that release's
# (the checksums below), so a local edit shows up in CI. Copy it into a consuming repo as
# scripts/check-uq-diag.sh. Exit 0 when all match, 1 on any mismatch or missing file, 2 on bad usage.
set -u
declare -A SUMS=(
    [1.0.0/uq_diag.rs]=861dadd1d35a77f97aae66c2525388c7c1bce20615a60048429ffcc2d8998852
    [1.0.0/uqdiag.cpp]=eb1d55eadf33992ac35990106a074e724026c325c785fc00a9c9b2e2550d6d86
    [1.0.0/uqdiag.h]=889d86e7364b5bf75111665b4c50bf144f8de822fb6ec9c64af3fc4c8beb4fff
    [1.0.1/uq_diag.rs]=a3480bcbd583420923e1cffced45c9be48190fc99b22d60828d2f50ba3123dbc
    [1.0.1/uqdiag.cpp]=b542647423d06e451e592188f255900a7b39f6ca65ee1f55e8e1279a1d83dd48
    [1.0.1/uqdiag.h]=94e92614fdb20e6a67203e7fca09685afb35475bc3272611cb0c26c4f279620b
)
if [ $# -lt 2 ] || ! [[ $1 =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "usage: check-uq-diag.sh <pinned-version X.Y.Z> <vendored file>..." >&2
    exit 2
fi
pinned=$1; shift
status=0
for file in "$@"; do
    name=$(basename "$file")
    if [ ! -r "$file" ]; then
        echo "check-uq-diag: cannot read $file" >&2; status=1; continue
    fi
    found=$(head -n 3 "$file" | grep -m1 -oE 'uq-diag [0-9]+\.[0-9]+\.[0-9]+' | cut -d' ' -f2)
    if [ "$found" != "$pinned" ]; then
        echo "check-uq-diag: $file is uq-diag ${found:-?}, pinned $pinned" >&2; status=1; continue
    fi
    want=${SUMS[$pinned/$name]:-}
    have=$(sha256sum "$file" | cut -d' ' -f1)
    if [ -z "$want" ]; then
        echo "check-uq-diag: this script does not know uq-diag $pinned $name; copy the newer script" >&2; status=1
    elif [ "$want" != "$have" ]; then
        echo "check-uq-diag: $file was edited (it is not uq-diag $pinned's $name); copy it again from uQ" >&2; status=1
    else
        echo "check-uq-diag: $file is uq-diag $found"
    fi
done
exit $status
