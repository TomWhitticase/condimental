#!/usr/bin/env bash
# Fails when a pull request body does not actually show the change.
#
# Run by .github/workflows/pr-screenshot.yml with PR_BODY set; runnable locally
# against a PR body file:  PR_BODY="$(cat body.md)" scripts/check-pr-screenshot.sh
#
# Three things are checked, in the order they go wrong in practice:
#   1. the body embeds at least one image;
#   2. no image is a raw.githubusercontent.com link — this repository is
#      private, so such a link needs a token and renders broken for readers;
#   3. every link into this repository resolves to a file that exists, so a
#      guessed or stale URL fails here rather than in the reviewer's browser.

set -uo pipefail

body="${PR_BODY:-}"
repo="${REPO:-}"

fail() {
  echo "::error::$*"
  echo
  echo "Render one with scripts/screenshot-scene.sh (pick the fixture that shows"
  echo "the change), publish it with scripts/pr-screenshot.sh, and paste the"
  echo "markdown it prints into the PR body under '## Screenshots'."
  exit 1
}

urls=$(printf '%s' "$body" \
  | grep -oE '!\[[^]]*\]\([^) ]+\)|<img[^>]+src="[^"]+"' \
  | grep -oE 'https?://[^) "]+' || true)

[ -n "$urls" ] || fail "This pull request body embeds no image. Every PR here shows the change."

if printf '%s\n' "$urls" | grep -q 'raw\.githubusercontent\.com'; then
  fail "raw.githubusercontent.com links render as broken images in a private repository. Use the github.com/<owner>/<repo>/blob/<branch>/<path>?raw=true form that scripts/pr-screenshot.sh prints."
fi

status=0
while IFS= read -r url; do
  case "$url" in
    "https://github.com/$repo/blob/"*) ;;
    *) continue ;;                      # an off-repo image is the author's call
  esac
  rest=${url#"https://github.com/$repo/blob/"}
  rest=${rest%%\?*}
  ref=${rest%%/*}
  path=${rest#*/}
  if ! gh api "repos/$repo/contents/$path?ref=$ref" --jq .sha >/dev/null 2>&1; then
    echo "::error::image not found in this repository: $ref/$path"
    status=1
  fi
done <<< "$urls"

[ "$status" -eq 0 ] || fail "A linked image does not exist, so it will render broken."

echo "PR body embeds $(printf '%s\n' "$urls" | wc -l | tr -d ' ') image(s); links resolve."
