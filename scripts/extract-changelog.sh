#!/usr/bin/env bash
# Extract a Keep-a-Changelog section for GitHub release notes.
# Usage: scripts/extract-changelog.sh 0.5.0
set -euo pipefail

VERSION="${1:?version required, e.g. 0.5.0}"
CHANGELOG="${2:-CHANGELOG.md}"

if [[ ! -f "$CHANGELOG" ]]; then
  echo "Missing changelog: $CHANGELOG" >&2
  exit 1
fi

awk -v version="$VERSION" '
  BEGIN { printing = 0 }
  index($0, "## [" version "]") == 1 {
    printing = 1
    print
    next
  }
  printing && (/^## \[/ || /^\[[0-9A-Za-z]/) { exit }
  printing { print }
' "$CHANGELOG" | awk 'NF {p=1} p' | awk '
  {
    lines[NR] = $0
  }
  END {
    end = NR
    while (end > 0 && lines[end] ~ /^[[:space:]]*$/) end--
    for (i = 1; i <= end; i++) print lines[i]
  }
'
