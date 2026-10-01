#!/usr/bin/env bash
# The two gauges at the top of docs/TECH_DEBT.md, worked out rather than kept
# by hand.
#
# A hand-bumped counter undercounts silently: two open PRs both turn "1" into
# "2", git sees the same edit on both sides and merges it without a conflict.
# So the change count is derived from git (squash merges are one first-parent
# commit per PR on main since the audit baseline), and the debt total is
# checked against the sum of the items.
#
# Usage:
#   scripts/tech-debt-status.sh          print both gauges against their thresholds
#   scripts/tech-debt-status.sh --write  also rewrite the header, counting this
#                                        branch as one more change (open-pr step 4)
#   scripts/tech-debt-status.sh --check  header total == sum of items, TD numbers
#                                        unique (run by check-invariants.sh; no git)
set -uo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
doc="$repo_root/docs/TECH_DEBT.md"
mode="${1:-}"

sum=0
while read -r pts; do sum=$((sum + pts)); done < <(
  sed -nE 's/^### TD-[0-9]+ · ([0-9]+) pt · .*/\1/p' "$doc")
header_total="$(sed -nE 's/.*\*\*Debt total: ([0-9]+) points?\*\*.*/\1/p' "$doc" | head -1)"

if [ "$mode" = "--check" ]; then
  status=0
  if [ "$header_total" != "$sum" ]; then
    echo "FAIL: docs/TECH_DEBT.md header says ${header_total:-nothing} debt points, its items sum to $sum" >&2
    status=1
  fi
  # The sum above reads only the exact heading form, so a near-miss
  # ("2 points", "####", a hyphen for the dot) would drop out of it silently.
  malformed="$(grep -nE 'TD-[0-9]' "$doc" | grep -E '^[0-9]+:#' \
    | grep -vE '^[0-9]+:### TD-[0-9]+ · [0-9]+ pt · .+$')"
  if [ -n "$malformed" ]; then
    echo "FAIL: docs/TECH_DEBT.md item headings not in the form '### TD-N · P pt · title':" >&2
    echo "$malformed" >&2
    status=1
  fi
  dupes="$(sed -nE 's/^### (TD-[0-9]+) .*/\1/p' "$doc" | sort | uniq -d)"
  if [ -n "$dupes" ]; then
    echo "FAIL: docs/TECH_DEBT.md reuses item numbers: $dupes" >&2
    status=1
  fi
  exit "$status"
fi

# shellcheck disable=SC2016  # the backticks are markdown, not a substitution
baseline="$(sed -nE 's/^Audit baseline: `([0-9a-f]+)`.*/\1/p' "$doc" | head -1)"
if [ -z "$baseline" ]; then
  echo "docs/TECH_DEBT.md has no 'Audit baseline: \`<sha>\`' line" >&2
  exit 1
fi
git -C "$repo_root" fetch -q origin main 2>/dev/null || true
merged="$(git -C "$repo_root" rev-list --first-parent --count "$baseline..origin/main")" || exit 1

change_limit="$(sed -nE 's/^\| Changes since last audit \| \*\*([0-9]+)\*\*.*/\1/p' "$doc")"
debt_limit="$(sed -nE 's/^\| Debt total \| \*\*([0-9]+) points\*\*.*/\1/p' "$doc")"
if [ -z "$change_limit" ] || [ -z "$debt_limit" ]; then
  echo "docs/TECH_DEBT.md's threshold table no longer reads as '| Changes since last audit | **N** |' and '| Debt total | **N points** |'" >&2
  exit 1
fi

changes="$merged"
if [ "$mode" = "--write" ]; then
  # This branch is one more change, unless it is main itself (nothing unmerged).
  ahead="$(git -C "$repo_root" rev-list --count origin/main..HEAD)"
  [ "$ahead" -gt 0 ] && changes=$((merged + 1))
  sed -i.bak -E \
    -e "s/\*\*Changes since last overview audit: [0-9]+\*\*/**Changes since last overview audit: $changes**/" \
    -e "s/\*\*Debt total: [0-9]+ points?\*\*/**Debt total: $sum points**/" "$doc"
  rm -f "$doc.bak"
fi

echo "changes since last overview audit: $changes (audit due at $change_limit)"
echo "debt total: $sum points (feature freeze at $debt_limit)"
[ "$changes" -ge "$change_limit" ] && echo "AUDIT DUE: run an overview audit before the next feature"
[ "$sum" -ge "$debt_limit" ] && echo "FEATURE FREEZE: pay debt down before the next feature"
exit 0
