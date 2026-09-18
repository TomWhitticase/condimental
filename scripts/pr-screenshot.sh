#!/usr/bin/env bash
# Publishes PNGs so a pull request body can SHOW the change, not just describe it.
#
# Every PR here carries at least one image (docs/WORKFLOW.md, "GitHub"). The
# PNGs that scripts/screenshot-scene.sh writes are gitignored and `gh` cannot
# attach a file to a PR body, so this script pushes them to an orphan branch
# (`pr-media`) that is never merged into main, and prints the markdown that
# embeds them.
#
# The repository is PRIVATE, which decides the URL form: a
# raw.githubusercontent.com link needs a token and renders as a broken image
# for everyone reading the PR, while a same-origin
# github.com/<owner>/<repo>/blob/<branch>/<path>?raw=true link is fetched with
# the reader's own session and renders for anyone who can see the repo.
#
# Usage:
#   scripts/pr-screenshot.sh shot.png [more.png...]
#
# The file name becomes the image caption, so name it after what it shows:
# `contracts-board-empty-state.png`, not `out.png`.
#
# Env: PR_MEDIA_BRANCH overrides the branch (default `pr-media`).

set -euo pipefail

media_branch="${PR_MEDIA_BRANCH:-pr-media}"

die() { echo "pr-screenshot: $*" >&2; exit 1; }

[ $# -gt 0 ] || die "usage: scripts/pr-screenshot.sh <shot.png> [more.png...]"

command -v gh >/dev/null || die "gh is not installed"
command -v jq >/dev/null || die "jq is not installed"
gh auth status >/dev/null 2>&1 || die "gh is not authenticated — do not fall back to a link you cannot verify"

repo=$(gh repo view --json nameWithOwner -q .nameWithOwner)

for f in "$@"; do
  [ -f "$f" ] || die "no such file: $f"
  case $(file -b --mime-type "$f") in
    image/png|image/jpeg|image/gif|image/webp) ;;
    *) die "$f is not an image — publish the PNG that screenshot-scene.sh wrote" ;;
  esac
done

# Create the media branch the first time. An orphan commit (no parents) keeps
# the images out of main's history entirely.
if ! gh api "repos/$repo/git/ref/heads/$media_branch" >/dev/null 2>&1; then
  echo "pr-screenshot: creating orphan branch $media_branch" >&2
  readme=$(cat <<'EOF'
# pr-media

Screenshots embedded in pull request bodies, published by
`scripts/pr-screenshot.sh`. This branch is an orphan — it shares no history
with `main` and is never merged into it. Nothing here is source; deleting an
old directory only breaks the images in an already-merged PR.
EOF
)
  blob=$(jq -n --arg c "$readme" '{content:$c,encoding:"utf-8"}' \
    | gh api "repos/$repo/git/blobs" --input - -q .sha)
  tree=$(jq -n --arg s "$blob" \
    '{tree:[{path:"README.md",mode:"100644",type:"blob",sha:$s}]}' \
    | gh api "repos/$repo/git/trees" --input - -q .sha)
  commit=$(jq -n --arg t "$tree" \
    '{message:"chore: start the pr-media branch",tree:$t,parents:[]}' \
    | gh api "repos/$repo/git/commits" --input - -q .sha)
  jq -n --arg s "$commit" --arg r "refs/heads/$media_branch" '{ref:$r,sha:$s}' \
    | gh api "repos/$repo/git/refs" --input - >/dev/null
fi

branch=$(git branch --show-current 2>/dev/null || echo detached)
slug=$(printf '%s' "$branch" | tr '/' '-' | tr -c 'A-Za-z0-9._-' '-' | sed 's/-*$//')
stamp=$(date -u +%Y%m%d-%H%M%S)

for f in "$@"; do
  base=$(basename "$f")
  name=${base%.*}
  ext=${base##*.}
  path="$slug/$stamp-$name.$ext"
  caption=$(printf '%s' "$name" | tr '_-' '  ')

  sha=$(gh api "repos/$repo/contents/$path?ref=$media_branch" -q .sha 2>/dev/null || true)
  # The base64 blob goes through --rawfile, not --arg: Linux caps a single
  # argv string at MAX_ARG_STRLEN (128 KiB), which a 1600x900 PNG clears
  # easily and `jq -n --arg c "$(openssl base64 ...)"` blew past with
  # "Argument list too long" — a size that never showed up in development
  # on macOS, which has no such per-argument cap.
  b64_file="$(mktemp)"
  openssl base64 -A -in "$f" >"$b64_file"
  payload=$(jq -n \
    --rawfile c "$b64_file" \
    --arg m "chore: pr screenshot for $branch" \
    --arg b "$media_branch" \
    --arg s "$sha" \
    '{message:$m, content:$c, branch:$b} + (if $s == "" then {} else {sha:$s} end)')
  rm -f "$b64_file"
  printf '%s' "$payload" | gh api -X PUT "repos/$repo/contents/$path" --input - >/dev/null

  echo "![$caption](https://github.com/$repo/blob/$media_branch/$path?raw=true)"
done
