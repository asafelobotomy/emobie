#!/usr/bin/env bash
# Fail if any tracked frontend/Rust source file exceeds the LOC budget.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MAX_LINES=400
failed=0

check_file() {
  local file="$1"
  local lines
  lines="$(wc -l < "$file" | tr -d ' ')"
  if (( lines > MAX_LINES )); then
    printf 'LOC check failed: %s has %s lines (max %s)\n' "$file" "$lines" "$MAX_LINES"
    failed=1
  fi
}

while IFS= read -r -d '' file; do
  check_file "$file"
done < <(
  find "$ROOT/src" -type f \( -name '*.ts' -o -name '*.tsx' -o -name '*.css' \) -print0
  find "$ROOT/src-tauri/src" -type f -name '*.rs' -print0
)

if (( failed )); then
  exit 1
fi

echo "LOC check passed (max ${MAX_LINES} lines per source file)."
