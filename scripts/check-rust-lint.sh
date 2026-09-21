#!/usr/bin/env bash
# rustfmt and clippy, as RATCHETS rather than as all-or-nothing gates.
#
# Why a ratchet. Both backlogs are now burned down — fmt in issue #28,
# clippy in issue #29 (2026-09-21) — so BOTH baselines are 0 and this is an
# ordinary all-or-nothing gate wearing a ratchet's clothes: one new warning
# is one over the baseline, and CI goes red.
#
# It stays a ratchet rather than becoming a plain `-D warnings` on purpose.
# A backlog can accrue again — a toolchain bump is the usual way, since a
# new clippy release lints code nobody touched — and when it does, the
# choice is between a red-by-default CI (a check everybody learns to ignore)
# and one deliberate, reviewed line raising a baseline with a reason beside
# it. `-D warnings` offers only the first. It also reports the count on
# every run and tells you when it can be lowered, which a hard denial cannot
# do. The rule is unchanged and is the whole point: LOWER freely, never
# raise without a reason in the commit message.
#
# So: the counts below are a high-water mark. New code cannot push them up —
# a newly added .rs file has to be formatted, and new code cannot introduce a
# new clippy warning — while the existing backlog is burned down separately.
# When you clean some up, LOWER the numbers here in the same commit; the
# script tells you when it can be lowered. Never raise them.
#
# Usage: scripts/check-rust-lint.sh   (same command CI runs; run it locally
# before pushing, it needs nothing but cargo)

set -uo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root" || exit 1

# High-water marks. Lower freely, never raise. See the header.
FMT_BASELINE=0       # unformatted files — burned down in issue #28
CLIPPY_BASELINE=0    # clippy warnings   — burned down in issue #29

status=0

summary() {
  # GitHub renders $GITHUB_STEP_SUMMARY on the job page; locally it is unset
  # and this is a no-op.
  [ -n "${GITHUB_STEP_SUMMARY:-}" ] && printf '%s\n' "$1" >> "$GITHUB_STEP_SUMMARY"
  return 0
}

ratchet() {
  local label="$1" count="$2" baseline="$3" fix="$4"
  if [ "$count" -gt "$baseline" ]; then
    echo "FAIL: $label went up: $count, baseline $baseline" >&2
    echo "      Fix what this change added — do not raise the baseline. $fix" >&2
    summary "❌ **$label**: $count (baseline $baseline) — new ones were added."
    status=1
  elif [ "$count" -lt "$baseline" ]; then
    echo "OK: $label is $count, below the baseline of $baseline."
    echo "    Lower FMT_BASELINE/CLIPPY_BASELINE in scripts/check-rust-lint.sh to $count."
    summary "✅ **$label**: $count (baseline $baseline) — lower the baseline to $count."
  else
    echo "OK: $label is $count, at the baseline."
    summary "✅ **$label**: $count (at baseline)."
  fi
}

echo "==> cargo fmt --all --check (ratchet: no NEW unformatted file)"
# rustfmt prints one "Diff in <path>:<line>:" header per hunk; the count that
# matters is DISTINCT FILES, so that a hundred hunks in one legacy file stay
# one unit of backlog.
fmt_out="$(cargo fmt --all --check 2>/dev/null)"
fmt_count="$(printf '%s\n' "$fmt_out" \
  | sed -n 's/^Diff in \(.*\):[0-9]*:$/\1/p' | sort -u | grep -c . )"
ratchet "unformatted files" "$fmt_count" "$FMT_BASELINE" "Run: cargo fmt -p <crate>"
if [ "$fmt_count" -gt 0 ]; then
  printf '%s\n' "$fmt_out" | sed -n 's|^Diff in .*/\([^/]*\):[0-9]*:$|  unformatted: \1|p' | sort -u
fi
echo

echo "==> cargo clippy --workspace --all-targets (ratchet: no NEW warning)"
# --all-targets covers tests too: sim_core/tests/ is where most of the sim's
# behaviour is actually asserted (ARCHITECTURE.md §7), so it gets linted like
# the library it exercises.
# clippy only emits a lint when it actually COMPILES the crate — a second run
# against a warm target/ prints nothing at all and would score a perfect zero.
# Dropping just the workspace members' artifacts (never the dependencies,
# which are the slow part) makes the count honest every run. CI hits the same
# trap from the other side: Swatinem/rust-cache already evicts workspace
# artifacts from its cache for this exact reason.
cargo clean -p sim_core -p voxgen -p xtask >/dev/null 2>&1
clippy_out="$(cargo clippy --workspace --all-targets --message-format short 2>&1)"
clippy_status=$?
if [ "$clippy_status" -ne 0 ]; then
  # A clippy failure here is a COMPILE error, not a lint: no lint is denied.
  printf '%s\n' "$clippy_out"
  echo "FAIL: clippy could not compile the workspace" >&2
  summary "❌ **clippy**: the workspace does not compile."
  exit 1
fi
clippy_count="$(printf '%s\n' "$clippy_out" | grep -cE '^[^ ]+:[0-9]+:[0-9]+: warning: ')"
ratchet "clippy warnings" "$clippy_count" "$CLIPPY_BASELINE" "Run: cargo clippy --fix"
printf '%s\n' "$clippy_out" | grep -E '^[^ ]+:[0-9]+:[0-9]+: warning: ' | sed 's/^/  /'

exit $status
