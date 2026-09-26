# Contributing

## Commit messages

This repo uses [Conventional Commits](https://www.conventionalcommits.org/). The
commit history is treated as a changelog source — messages that don't follow the
format will eventually feed broken release notes, so they're linted, not just
"encouraged."

```
<type>(<optional scope>): <description>

[optional body]

[optional footer(s)]
```

**Types:** `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`

**Description:** imperative mood, lowercase start, no trailing period.

**Examples:**

```
feat(conveyor): add belt splitter component
fix(sim): correct tick order for fluid mixing
docs: note tech stack decision in CLAUDE.md
```

**Breaking changes:** append `!` after the type/scope and add a `BREAKING CHANGE:`
paragraph in the body:

```
feat(save)!: change save file format to binary

BREAKING CHANGE: old JSON save files are no longer loadable.
```

### Enforcement

- **Locally**: a `commit-msg` git hook rejects non-conforming messages before the
  commit is even made. One-time setup per clone:

  ```
  git config core.hooksPath .githooks
  ```

- **CI**: `.github/workflows/commitlint.yml` re-checks every commit message on
  push/PR as a backstop, in case the local hook was skipped
  (e.g. `git commit --no-verify`).

## Branches, pull requests and issues

**Nothing lands on `main` by a direct push.** Work on a branch named for the
Conventional Commit type of the change (`feat/`, `fix/`, `docs/`, `chore/`…),
push it, and open a pull request:

```
git switch -c fix/belt-corner-arc
git push -u origin HEAD
gh pr create --fill
gh pr checks --watch
gh pr merge --squash --delete-branch   # only after a VERDICT: APPROVED review
```

**No pull request merges before it is reviewed**: green checks *and* a comment
review whose first line is `VERDICT: APPROVED`, from someone (or a session) that
did not write the branch — the `review` skill in `.claude/skills/` is how.

A `pre-push` hook (same `.githooks` setup as below) refuses a push to `main`
and prints those steps. `.github/pull_request_template.md` sets out what a PR
body needs: what changed and why, **the verification commands actually run and
what they reported**, whether the DEVLOG and the decision record needed
updating, and `Closes #N` for the issues it settles.

**Squash-merge by default.** The squashed subject becomes the single
Conventional Commit for the change and is linted like any other, which is what
keeps the history usable as a changelog source (see *Why* below). A real merge
commit is for the rare branch whose individual commits are each worth keeping.

**File an issue for anything you notice and decide not to fix.** This project
is built largely by AI agents in separate sessions that share no memory, so a
defect mentioned only in a session's closing summary is gone. Use the
*Deferred finding* issue template, label it `deferred` plus one `area:` label,
and name the file and function, how it was found, why it was deferred and what
the fix probably is. `owner-decision` marks anything blocked on a human call —
a gameplay change, a deviation from a design doc, anything invalidating saves.

`docs/WORKFLOW.md` has the full rule and the reasoning behind it.

## Development log

Every commit should prompt the question "does docs/DEVLOG.md need a note?" —
not every commit needs one (see docs/DEVLOG.md's own guidance: keep it
high-level, skip trivial changes), but the question should get asked, not
skipped. A `pre-commit` hook (same `.githooks` setup as above) prints a
reminder if a commit doesn't touch `docs/DEVLOG.md`; it never blocks the
commit, since whether an entry is actually warranted is a judgment call. See
`docs/WORKFLOW.md` for the actual rule an AI agent should follow here.

### Why

Once this settles into a real release cadence, `feat`/`fix`/`BREAKING CHANGE`
commits can be run through a changelog generator (e.g.
[git-cliff](https://git-cliff.org/) or `conventional-changelog`) to produce
release notes automatically. That only works if the history is clean from the
start.

## Licence of contributions

CONDIMENTAL is licensed under the [GNU GPL v3](LICENSE) (`GPL-3.0-only`). By
opening a pull request you agree that your contribution is licensed under the
same terms (inbound = outbound), and that you have the right to contribute it.
Do not add a dependency whose licence is not on `deny.toml`'s allow-list — CI
refuses it, and widening that list is a licensing decision, not a build fix.
The CONDIMENTAL name and brand are reserved; see [TRADEMARKS.md](TRADEMARKS.md).
