#!/usr/bin/env bash
# Checks that the problem codes this app logs and its list for uQ agree (uQ's docs/diagnostics.md):
# every `[PREFIX-NNN]` in the sources is in the list, and every listed code that isn't
# `retired = true` is still logged somewhere.
#
#   scripts/check-diag-codes.sh uq/diag/<app>.toml <source dir>...
set -euo pipefail
[ $# -ge 2 ] || { echo "usage: $0 LIST.toml DIR..." >&2; exit 2; }
list=$1; shift
prefix=$(grep -m1 -oE '^id *= *"[A-Z0-9]+-' "$list" | sed -E 's/.*"//; s/-$//')
[ -n "$prefix" ] || { echo "check-diag-codes: no codes in $list" >&2; exit 1; }

used=$(grep -rhoE "\[$prefix-[0-9]{3}\]" --exclude='uq_diag.rs' --exclude='uqdiag.*' "$@" | tr -d '[]' | sort -u)
# id and retired of each [[code]] block.
read_list() {
    awk -v want="$1" '
        /^\[\[code\]\]/ { if (id != "") emit(); id = ""; retired = 0; next }
        /^id *=/ { id = $0; sub(/^id *= *"/, "", id); sub(/".*/, "", id) }
        /^retired *= *true/ { retired = 1 }
        function emit() { if (want == "all" || !retired) print id }
        END { if (id != "") emit() }
    ' "$list" | sort
}
listed=$(read_list all)
live=$(read_list live)

bad=0
dups=$(printf '%s\n' "$listed" | uniq -d)
[ -z "$dups" ] || { echo "check-diag-codes: listed twice: $dups" >&2; bad=1; }
unknown=$(comm -23 <(printf '%s\n' "$used") <(printf '%s\n' "$listed" | sort -u) | grep . || true)
[ -z "$unknown" ] || { echo "check-diag-codes: logged but not in $list:" $unknown >&2; bad=1; }
unused=$(comm -13 <(printf '%s\n' "$used") <(printf '%s\n' "$live" | sort -u) | grep . || true)
[ -z "$unused" ] || { echo "check-diag-codes: listed but nothing logs them (set retired = true):" $unused >&2; bad=1; }
for k in severity title means fix; do
    n=$(grep -cE "^$k *= *\"[^\"]+\"" "$list" || true)
    [ "$n" -eq "$(printf '%s\n' "$listed" | grep -c .)" ] || { echo "check-diag-codes: every code needs a $k" >&2; bad=1; }
done
[ $bad = 0 ] && echo "check-diag-codes: $(printf '%s\n' "$live" | grep -c .) codes in $list, all logged"
exit $bad
