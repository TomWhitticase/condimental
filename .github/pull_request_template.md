<!--
docs/WORKFLOW.md, "GitHub: branches, pull requests and issues", is the rule.
Delete any section that genuinely does not apply — do not leave it empty.
-->

## What and why

<!-- What changed, and what problem it solves. Link the design doc or the
decision record it follows (ARCHITECTURE §x, UI_UX §2b point N, GAMEPLAY §x).
If it deviates from one of the three design docs, say so here explicitly —
that is a discussion, not a merge. -->

## How it was verified

<!-- Name the commands actually run and their result. Not "tests pass".
  - [ ] `cargo test -p sim_core`
  - [ ] `cargo test -p voxgen`
  - [ ] `scripts/verify-core-loop.sh` (30 headless Godot markers + PASS)
  - [ ] `scripts/screenshot-scene.sh <flags>` — the PNG goes under
        Screenshots below, and what it showed goes in words here.
Anything sim-core needs a headless test covering it (ARCHITECTURE §10). -->

## Screenshots

<!-- REQUIRED, on every PR — CI fails a body with no image in it.
Render the frame that shows this change (`scripts/screenshot-scene.sh out.png
1600x900 <flag>`), read the PNG, then publish it:

    scripts/pr-screenshot.sh out.png

and paste the markdown it prints below, each image with a line saying what it
shows and what to look at. For a change with no visual surface, show the state
it affects — the panel whose numbers moved, or the demo line at this commit.
Name the PNG after its subject: the file name becomes the caption. -->

## Record-keeping

<!-- Say which of these was done, or why it was not needed:
  - `docs/DEVLOG.md` — a status/history note, or "not DEVLOG-worthy"
  - `docs/DECISIONS_LOG.md` + `docs/DECISIONS.md` + the design doc — only if
    a non-trivial decision was made (see the `record-decision` skill) -->

## Issues

<!-- `Closes #N` for each issue this settles — the merge closes them.
And file an issue for anything noticed and deliberately NOT fixed here;
list those too, so the deferral is visible at review time rather than lost. -->
