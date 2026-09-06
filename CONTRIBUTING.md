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

### Why

Once this settles into a real release cadence, `feat`/`fix`/`BREAKING CHANGE`
commits can be run through a changelog generator (e.g.
[git-cliff](https://git-cliff.org/) or `conventional-changelog`) to produce
release notes automatically. That only works if the history is clean from the
start.
