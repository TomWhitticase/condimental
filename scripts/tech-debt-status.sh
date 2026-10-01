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
# The freeze has hysteresis — it starts at one total and lifts only below a
# lower one — so the header records whether it is in force ("Feature freeze:
# on|off"). --write sets it; --check holds it to the totals.
#
# Usage:
#   scripts/tech-debt-status.sh          print both gauges against their thresholds
#   scripts/tech-debt-status.sh --write  also rewrite the header, counting this
#                                        branch as one more change (open-pr step 4).
#                                        Run it before the PR merges, never after:
#                                        a merged branch would count itself twice.
#   scripts/tech-debt-status.sh --check  header total == sum of items, TD numbers
#                                        unique, freeze state consistent (run by
#                                        check-invariants.sh; no git)
set -uo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
doc="$repo_root/docs/TECH_DEBT.md"
mode="${1:-}"

sum=0
while read -r pts; do sum=$((sum + pts)); done < <(
  sed -nE 's/^### TD-[0-9]+ · ([0-9]+) pt · .*/\1/p' "$doc")
header_total="$(sed -nE 's/.*\*\*Debt total: ([0-9]+) points?\*\*.*/\1/p' "$doc" | head -1)"
freeze_state="$(sed -nE 's/^Feature freeze: \*\*(on|off)\*\*.*/\1/p' "$doc" | head -1)"

change_limit="$(sed -nE 's/^\| Changes since last audit \| \*\*([0-9]+)\*\*.*/\1/p' "$doc")"
debt_limit="$(sed -nE 's/^\| Debt total \| \*\*([0-9]+) points\*\*.*/\1/p' "$doc")"
debt_floor="$(sed -nE 's/^\| Freeze lifts below \| \*\*([0-9]+) points\*\*.*/\1/p' "$doc")"
if [ -z "$change_limit" ] || [ -z "$debt_limit" ] || [ -z "$debt_floor" ]; then
  echo "docs/TECH_DEBT.md's threshold table no longer reads as '| Changes since last audit | **N** |', '| Debt total | **N points** |' and '| Freeze lifts below | **N points** |'" >&2
  exit 1
fi
if [ -z "$freeze_state" ]; then
  echo "docs/TECH_DEBT.md has no 'Feature freeze: **on**' or '**off**' line" >&2
  exit 1
fi

# The freeze starts at debt_limit and holds until the total drops below
# debt_floor; between the two, the recorded state decides.
freeze=off
if [ "$sum" -ge "$debt_limit" ]; then freeze=on
elif [ "$sum" -ge "$debt_floor" ] && [ "$freeze_state" = "on" ]; then freeze=on
fi

if [ "$mode" = "--check" ]; then
  status=0
  if [ "$header_total" != "$sum" ]; then
    echo "FAIL: docs/TECH_DEBT.md header says ${header_total:-nothing} debt points, its items sum to $sum" >&2
    status=1
  fi
  if [ "$freeze_state" != "$freeze" ]; then
    echo "FAIL: docs/TECH_DEBT.md says 'Feature freeze: $freeze_state' at $sum points — run scripts/tech-debt-status.sh --write" >&2
    status=1
  fi
  # The sum above reads only the exact heading form, so a near-miss
  # ("2 points", "####", "td 1", a hyphen for the dot) would drop out of it
  # silently: any heading naming an item must be in that form.
  malformed="$(grep -niE '^[[:space:]]*#.*td[- ]?[0-9]' "$doc" \
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

changes="$merged"
if [ "$mode" = "--write" ]; then
  # This branch is one more change, unless it is main itself (nothing unmerged).
  ahead="$(git -C "$repo_root" rev-list --count origin/main..HEAD)"
  [ "$ahead" -gt 0 ] && changes=$((merged + 1))
  sed -i.bak -E \
    -e "s/\*\*Changes since last overview audit: [0-9]+\*\*/**Changes since last overview audit: $changes**/" \
    -e "s/\*\*Debt total: [0-9]+ points?\*\*/**Debt total: $sum points**/" \
    -e "s/^Feature freeze: \*\*(on|off)\*\*/Feature freeze: **$freeze**/" "$doc"
  rm -f "$doc.bak"
fi

echo "changes since last overview audit: $changes (audit due at $change_limit)"
echo "debt total: $sum points (feature freeze at $debt_limit, lifts below $debt_floor)"
[ "$changes" -ge "$change_limit" ] && echo "AUDIT DUE: run an overview audit before the next feature"
[ "$freeze" = "on" ] && echo "FEATURE FREEZE: pay debt down below $debt_floor points before the next feature"
exit 0
