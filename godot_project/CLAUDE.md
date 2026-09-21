# godot_project — the presentation layer

Authority: `docs/UI_UX.md` (visual/UI), `docs/ARCHITECTURE.md` §4–§5 (the
wire, the rebuild rule). Reasoning for anything below: `docs/DECISIONS.md`.

## The one rule

**Godot renders, formats and sorts. The sim decides.** No gameplay logic in
GDScript, and never recompute a number the wire already carries: the staff
panel reads `next_bill`/`shortfall` and the sim's own `demand`/`assigned`/
`short`, never a Godot-side `Σ headcount × wage` (`_staff_next_bill`,
`_staff_shortfall` — both fall back to a local sum only for a pre-field
snapshot). Same for load windows, production counts and stall reasons.
Godot-side derivation is legitimate for *drawing*: belt/pipe shape from
`links`, floor variant from tile coords, payout detection by polling
`deliveries_made`.

## Wire and rebuild invariants

- The 3D World subtree is rebuilt from `world_layout_json` on every
  place/remove/rotate — **never per tick** (`_rebuild_world`).
- A rebuild re-reads **every occupied chunk's FULL** `chunk_snapshot_json`
  (`_reread_every_chunk`). Re-reading only the dirty chunk made items on
  belts elsewhere vanish, permanently on a backed-up belt.
- The tick path reads `chunk_delta_json` (`_apply_dirty_chunks`), not the
  full snapshot.
- Per-frame polls are gated on `_wire_changed()`: `sim.revision()` (ticks +
  commands, refused or not) plus viewport size, hotbar selection and the
  open-modal bitmask. Wall-clock fades (`_update_money_flyout`,
  `_update_hotbar_flyout`) run in the **`else` branch** so nothing freezes
  under the pause menu.
- Fluid and tank auto-fill dirty nothing — polled (`fluid_snapshot_json`).
  Machine load, staff and production are polled on their own tick cadence.
- `_advance_simulation` caps catch-up at `MAX_TICKS_PER_FRAME` and
  **discards** the backlog; paused means no ticks and no accumulation.

## Presentation invariants

- **Every animation/celebration clock is `sim.current_tick()`**, never wall
  time — pause freezes them and screenshot runs stay reproducible. The one
  exception is the title screen (`title_scene.gd`'s camera drift,
  `menu.gd`'s splash pulse): there is no simulation on that screen to take a
  tick from.
- **The hall is drawn in first person, never in isometric.** A sky is
  intersected against `EYEDIR`; an orthographic camera's rays are parallel,
  so it comes out as stripes fanning from the corners.
  `factory_environment.gd`'s `set_backdrop_for` is the one place that
  decides, called from `main.gd`'s `_apply_backdrop_mode` at build and on
  every TAB. **Only `background_mode` changes** — the `Sky` stays the ambient
  and reflection source in both modes, or the factory's look shifts on TAB.
- **Never colour alone** (UI_UX §7): pair every status colour with a glyph,
  word, shape or position.
- **One meter language**: `_build_meter`/`_set_meter` only. A bar exists
  only where the sim defines the ceiling; three fills, one meaning each —
  `METER_FILL_LEVEL` / `_PROGRESS` / `_WARN`. `_set_meter` sizes the fill
  explicitly (never anchors) and records the fraction on the track, so a
  headless test can assert it with no layout pass.
- **Seconds, never ticks** in player-facing clocks; **display names**
  (`_kind_display_name`/`_item_display_name`), never wire ids.
- **UI scale is two constants** — `UI_SCALE`, `UI_FONT_BASE` — threaded
  through `_ui_px`/`_ui_i`/`_ui_v`/`_ui_font`. Not `content_scale_factor`:
  the scene maps 3D→screen in several places.
- **Every content-id lookup needs a documented fallback** (`MACHINE_MODELS`
  → `MACHINE_MODEL_FALLBACK`, `ITEM_MODELS`, `FLUID_TINTS`, `_icon_for`). A
  stale key ships silently with every test green; only the rendered frame
  catches it (UI_UX §2b point 12).
- **3D burst distances are measured in `TILE`**, not raw world units.
- **Visual randomness is derived from an index**, never `randf()`, so a
  review screenshot is repeatable (coin lift/spread; `audio.gd` uses an LCG).
- **A cursor-following panel positions against the fixed HUD plates**
  (hotbar reserved height, legend, clipboard), not just viewport edges, and
  every candidate is judged *after* the clamp (`_panel_position_near`,
  `_pushed_clear_of` — UI_UX §2b point 33). The right-hand column is **one
  list** — `_contract_panel_rect`, `_stock_rect`, `_crew_rect` — never a
  branch per plate, and a panel flipped aside lands its edge *on* the bound.
- **No modal is ever wider than the window, and no control is drawn where
  it cannot be clicked** (UI_UX §5c). Width yields in this order: prose to
  the row's hover (`_fit_staff_width`), then fewer grid columns
  (`_fit_grid_columns` — `SHOP_SLOT_COLUMNS`/`MARKET_SLOT_COLUMNS` are the
  *most* a grid takes), then scroll. **No `autowrap_mode` on a Label whose
  width a fit pass measures**: a wrapping Label reports a 1px minimum width
  and a height shaped at whatever width it was last *drawn* at, which is
  meaningless under `--headless`, so a panel measured around one is measured
  around nothing. Break such a sentence at a join it already has. It is used
  legitimately 17 times in `main.gd` and twice in `menu.gd`, every one of them
  with the width pinned first — by a fixed-size ancestor (`SHOP_SLOT_SIZE`,
  `MARKET_SLOT_SIZE`, `CONFIG_PICKER_SLOT_SIZE`) or by the Label's own
  `custom_minimum_size.x` (`CONFIG_PANEL_WIDTH`, `CREW_TEXT_WIDTH`,
  `PAUSE_BUTTON_MIN_SIZE.x`) — so the pin dominates the 1px and the
  measurement stays honest. **An unpinned one is the bug, not the exception**:
  the configuration panel's footer was the only Label on that panel with
  neither a pin nor a wrap, and at 489px it made the whole panel 99px wider
  than `CONFIG_PANEL_WIDTH` says it is — which is what put it on the legend
  it could not then get off (issue #107). Pin the width or do not wrap.
- **A headless test cannot prove a width.** Headless font metrics are
  narrower than a real window's, and `get_combined_minimum_size()` can
  underreport what is on screen — take `maxf(minimum, size)` when
  positioning. Nor can you get a width out of a wrapping Label by turning
  wrapping off to measure it: the cached minimum is only re-shaped when the
  TEXT changes, so a single-word label answers "1px, it fits" to every
  question. Measure the **font** (`Font.get_string_size`) when you need a
  number. `scripts/screenshot-scene.sh` at 1280x720 is the acceptance.
- **Audio is synthesised into `AudioStreamWAV` at runtime**; no `.wav` in the
  repo. A cue is fired by the handler that knows the real outcome (compare
  money/owned before and after), never by the input — so a refused command
  always sounds like `REFUSED`.
- `.tscn`/`.tres` stay **text**, never binary, no visual scripting. `main.tscn`
  and `menu.tscn` are a script on a bare node: the whole scene graph and HUD
  are built in code. Shared scripts are reached by `preload`, not
  `class_name` (unregistered on a cold `.godot/` under `--script`).

## Orientation

`main.gd` (~11.9k lines) — the 3D scene, HUD, build mode, every panel, the
tick loop. `game_session.gd` — autoload; the New Game/Load handoff **and all
save-file rules** (dir, listing, slot sanitizer, reserved `autosave` slot);
`sim_core` does no file I/O. `music.gd` — autoload; the one looping track,
ducked (not stopped) under pause, skipped headless. `audio.gd` — every cue,
synthesised. `menu.gd` — main menu; validates a save through a throwaway
`SimBridge` **before** the scene changes. `title_scene.gd` — the menu's 3D
backdrop, a hand-placed poster of the chain; **art, not a sim** (nothing
ticks, nothing is placed by a command). `voxel_assets.gd` and
`factory_environment.gd` — the `.vox`/material loading and the hall's
sky/lights, shared by `main.gd` and the title screen so the two cannot drift;
neither is a place for new logic. `test_layouts.gd` — shared fixture,
builds factories through real purchase/place commands.

## Running things

- `scripts/verify-core-loop.sh` — cargo tests, `cargo xtask`, the imports,
  then every headless `test_*.gd`.
- One test: `godot --headless --path godot_project --script test_x.gd`.
  **Every test must print its own success marker** (`FOO_OK`) — Godot exits 0
  even on a script parse failure, so exit status proves nothing.
- Godot needs **two** `--import` passes on a cold `.godot/` cache: the `.vox`
  importer is a plugin and registers only on the pass after the first scan.
- `scripts/screenshot-scene.sh` is the one **non-headless** check
  (`test_screenshot.gd`); it is the only thing that catches a stale lookup or
  a panel landing on a plate. Read the PNG.
- `bin/` and `voxels/` are gitignored build output staged by `cargo xtask`;
  `sim_core.gdextension` is checked in and points at them.
