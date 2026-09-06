# Sauce Factory — Development Log & Status Tracker

This doc answers "what's actually been done, and what's the state of things
right now" for whoever (human or agent) picks this project up cold. It's a
**log and status tracker, not a design doc** — the reasoning and specifics
live in [ARCHITECTURE.md](ARCHITECTURE.md), [UI_UX.md](UI_UX.md), and
[GAMEPLAY.md](GAMEPLAY.md); this file indexes *when* and *why* things changed,
and what's true *now*. It is deliberately **not** `@`-imported into
`CLAUDE.md` (unlike those three) — it will grow without bound over the life
of the project, so it's meant to be checked when you need history/status, not
force-loaded into every session's context.

---

## Current status

*(This section reflects "now" — edit it in place as things change. The Log
below is history and should only ever be appended to.)*

- **Stage:** Pre-alpha, docs-only. No game code exists yet.
- **Repo:** Private GitHub repo (`TomWhitticase/sauce-factory`), `main` branch. Conventional Commits enforced via a local `commit-msg` hook + CI backstop (see CONTRIBUTING.md).
- **Design docs:** ARCHITECTURE.md, UI_UX.md, and GAMEPLAY.md are all written and cross-referenced; docs/README.md indexes them.
- **Milestone progress:** none of ARCHITECTURE.md §9's milestones have started. Next up is **milestone 1 (Scaffold)** — Godot project + `gdext` hello-world binding, plus the build script and `.gdextension` file called for in §7.
- **Open threads:** each doc's own "Open questions" section (ARCHITECTURE.md §12-ish, UI_UX.md §9, GAMEPLAY.md §11) is the real TODO list at this stage — nothing is blocked, these are just undecided-and-known-undecided.

---

## How to use this doc

- **Newest entries at the top of the Log.**
- **Keep it high-level.** A few sentences per entry: what changed in general terms and why it matters for where the project is at, not a blow-by-blow of every file/decision — that detail already lives in the docs' own decisions/changes and in `git log`/commit messages. If you're tempted to write a bullet per commit, zoom out instead.
- One entry per meaningful chunk of work (a session, a design pass, a milestone), not one per commit.
- **Update "Current status" above whenever it goes stale.** That section is what anyone should read first; the log is for "how did we get here," briefly.
- **Not every commit needs an entry.** Small/trivial changes (typo fixes, a tweaked wording, a minor config change) don't warrant one — see CLAUDE.md's commit convention for the actual rule on when to add one.

---

## Log

### 2026-09-06 — Repo set up, full design-doc pass complete

Set up the repo (private, Conventional Commits enforced via hook + CI) and
wrote all three core design docs: **ARCHITECTURE.md** (Rust sim core +
Godot/`gdext` presentation, chosen to be agent-friendly, then hardened via a
critical-review pass that turned several open options into firm decisions),
**UI_UX.md** (isometric voxel art at a 32×32×32 tile scale, Minecraft-adjacent
UI, plus a researched asset pipeline so voxel models can be authored and
reviewed by an agent), and **GAMEPLAY.md** (tiered sauce/equipment
progression, a contracts-based economy designed to force a diversified
factory, staff as an abstract resource pool). Still no game code — the docs
are the whole project so far. Next up: milestone 1 (scaffold) from
ARCHITECTURE.md. For specifics, see the docs themselves, CLAUDE.md's
decisions log, or `git log`.

