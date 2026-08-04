# Retire the mainline and POC example runs, reduce screenshots to capture-only

- PRIORITY: 78
- TAGS: v0.10.0, examples, testing
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE
- PARENT: 20260802-115955
- DEPENDS ON: 20260804-003244, 20260804-093855, 20260804-093934

## Story

Delete what the roster spike (`20260804-003244`) retired, and reduce
`screenshots/` to what its contract allows. All mechanical, no new content.

Story scenarios lose their example coverage on purpose. Not because they churn
- the spike's review measured that and it is false (`broadside.rs` 11 commits
ever, `lifeline.rs` 6) - but because an autopilot-assisted win over 8000 lines
of story RON proves little: `broadside` and `lifeline` assert story wave
timings and object ids, which is content, not system behavior. Story is tested
by players; examples test systems.

## Steps

Deletions first (they are independent), then the reduction, then the run.

- [x] Delete `examples/gameplay/broadside.rs` + `lifeline.rs`, then the
      now-empty `examples/gameplay/` directory (`scenario.rs`/`playable.rs`
      already left with `20260804-093934`).
- [x] Delete `examples/ui/nova_os_rtt_poc.rs` and its example-owned shader
      `assets/shaders/nova_os_rtt_poc.wgsl` (no crate and no bundle manifest
      names it; confirmed by `rg -rn 'nova_os_rtt_poc' --glob '!examples/**'`).
- [x] `Cargo.toml`: delete the three `[[example]]` blocks (`:115-121`,
      `:128-130`), the `gameplay/ - TRANSITIONAL` comment block above them
      (`:112-114`), and the orphaned `[package.metadata.nova_probe]` table with
      its comment (`:27-33`) - `fps_exempt = ["broadside"]` is its only key and
      nothing reads it (`20260804-093855` deleted `parse_fps_exempt` /
      `load_fps_exempt`). See DECISION.md on taking this from `094006`.
- [x] `crates/nova_probe/src/catalog.rs:190-198`: delete the `"gameplay"` row
      from `CATEGORY_POLICIES` - its own comment says "TRANSITIONAL: remove with
      20260804-093910". Leave the `"gameplay"` strings in that file's and
      `aggregate.rs:333`'s synthetic fixtures: they exercise arbitrary-category
      parsing, and a live category name there would weaken the test.
- [x] `tests/examples_smoke.rs`: delete `GAMEPLAY` (`:47-48`) and
      `gameplay_reach_playing_without_panic` (`:109-112`); drop
      `"nova_os_rtt_poc"` from `NOT_SMOKED` (`:86`) with its doc bullet
      (`:74-80`). Reword the surviving `broadside` mention in the completion
      comment at `:326` to name a live self-ending example.
- [x] Reword the retired names out of the two surviving prose references:
      `crates/nova_debug/src/harness.rs:71-72` (the self-ending example list)
      and `examples/ui/menu_scenarios.rs:70,152` ("the broadside pattern").
      Both are comments; the pattern they name now lives in `menu_scenarios`
      itself. Needed for DoD 1, which greps the whole of `examples/`.
- [x] Strip probe enrollment from the six `screenshots/` runs that carry it -
      `nova_probe::{nova_timeline, nova_invariants, nova_frametime}` in
      `screenshot_orbit.rs:44-46`, `screenshot_juice.rs:42-44`,
      `screenshot_combat.rs:60-62`, `screenshot_ui.rs:72-74`,
      `screenshot_reel.rs:61-63`, `screenshot_sections.rs:41-43`. The category
      is `probed: false` since `20260804-093855`, so the wiring is dead weight.
- [x] Delete `screenshot_reel.rs`'s smoke backstop `reel_smoke_probe`
      (`:158-172`, panics if the embedded scene never loaded) and its
      `.input(...)` hookup at `:64`. The `assert_eq!` at `:97` stays - it is
      `include_str!` parse validation in scene setup, not a run assertion.
- [x] Convert `screenshot_orbit.rs`, `screenshot_juice.rs`,
      `screenshot_combat.rs` from `.hold(GameStates::Loading, N)` +
      `.input(<script>)` to explicit `AutopilotPlugin` step timelines: one
      `.step(name).on_enter(...).until(...)` per beat, with `predicate::elapsed`
      (which is IN-STEP seconds, `predicate.rs:11`) replacing the
      `elapsed - playing_since` offsets, and `harness::player_ship_present()` /
      `state_is(GameStates::Playing)` as the load gate instead of a wall-clock
      hold. This deletes the `OrbitScript`/`JuiceScript`/`CombatScript` beat
      booleans outright. Each capture gets its own step, which preserves the
      one-capture-per-frame constraint noted at `screenshot_combat.rs:225-228`.
- [x] Convert `screenshot_ui.rs` the same way (`:75-78` + `ui_capture_script`),
      and `screenshot_nova_os.rs`'s twelve-stage `NovaOsScript` machine
      (`:53-67`, `:214-333`) into one step per beat. Its per-beat `settle` frame
      counts are load-bearing for the capture path - carry them over as
      `predicate::frames(n)`, reading `NOVA_REEL` once when the plugin is built.
      The conversion removes the last hand-rolled `HarnessCompletion::done`
      call in `screenshots/`; the driver reports done when the last step ends.
- [x] `render_scale_shot.rs` is untouched: it is already a bare
      `NOVA_SHOT`-driven single capture with no probe wiring, and its
      `NOT_PROBED` entry stays (see DECISION.md).
- [x] Verify: `nix develop --command cargo check --examples --features debug`,
      then RUN all seven smoked producers under Xvfb, then run the capture path
      (`NOVA_REEL=1 NOVA_SHOT_DIR=target/reel`) for the five converted producers
      and confirm every PNG they used to write still lands.

## Definition of Done

- The retired runs are gone from the tree and the catalog.
  (cmd: `! rg -n 'broadside|lifeline|nova_os_rtt_poc' Cargo.toml examples tests`)
- No `screenshots/` producer carries a hand-rolled beat script, wall-clock
  origin, or script-owned completion - the driver owns every beat.
  (cmd: `! rg -n 'playing_since|get_or_insert|HarnessCompletion' examples/screenshots`)
- No `screenshots/` producer enrolls in probe.
  (cmd: `! rg -n 'nova_probe::' examples/screenshots`)
- The retired `gameplay/` probe policy is gone with its examples.
  (cmd: `! rg -n 'TRANSITIONAL: remove with 20260804-093910' crates`)
- The reduced screenshot producers still run their full harnessed cycle and
  exit clean, so the capture path is intact even though probe no longer runs
  them. (test: `screenshots_reach_playing_without_panic`)
- The catalog, disk and smoke lists agree after the deletions.
  (test: `catalog_matches_disk`, `every_category_has_a_probe_policy`)
- The capture path still writes every shot the web build consumes: a
  `NOVA_REEL=1 NOVA_SHOT_DIR=target/reel` run of `screenshot_orbit`,
  `screenshot_juice`, `screenshot_combat`, `screenshot_ui` and
  `screenshot_nova_os` under Xvfb produces the same PNG filenames as before,
  each non-empty. (cmd: record the `ls target/reel` listing in the task record)

## Notes

- Base-branch redness confirmed for all four `cmd:` proofs (2026-08-04):
  proof 1 hits 6 files, proof 2 hits 4, proof 3 hits 6, proof 4 hits 1.
- RETIRE `examples/gameplay/broadside.rs` and `examples/gameplay/lifeline.rs`.
  Their SYSTEM coverage (scenario chaining, Defeat + Retry reload-clean,
  Victory/CHECKPOINT) is NOT dropped - it moved to `systems/outcomes` with
  `20260804-093934`, which is CLOSED, so the ordering risk is discharged.
- RETIRE `examples/ui/nova_os_rtt_poc.rs`: the RTT pipeline shipped, and a POC
  is not coverage. Its coverage becomes an RTT element test beside the other
  widget tests - owned by `20260804-094021`, NOT this task.
- Coverage flag: `--report`, one name, built and owned by `20260724-082856`
  (which now DEPENDS ON this task). Deliberately NOT in this task's DoD - that
  would be circular, since 082856 needs these rebuilt producers and this task
  cannot be gated on a flag 082856 has not written yet. Shot-for-shot coverage
  of the web build is 082856's criterion; this task proves only that the
  reduced producers still run clean and still write their files.
- The three `*_poc.html` design sources are NOT this task's: epic child
  `20260804-003301` owns that move. Named here only so the boundary is clear.
- Examples must be RUN under Xvfb :99, not only checked - `cargo check` misses
  duplicate-component panics and inert-harness wiring.
- The step vocabulary this converts onto already exists and needs no additions:
  `AutopilotPlugin::step/on_enter/each/until/deadline/add`
  (`crates/nova_autopilot/src/autopilot.rs:172-290`), `predicate::{elapsed,
  frames, state_is, resource_where, any_entity, and, not}`, and
  `harness::{player_ship_present, scenario_variable_is, section_gone}`.
- No documentation, CI workflow, or web script names the retired examples
  (checked `docs web scripts .github AGENTS.md README.md`: no hits), so the
  deletions need no doc follow-up.

## Close-out

### What and why

Two halves, as planned. The RETIREMENT deleted `examples/gameplay/`
(`broadside`, `lifeline`), `examples/ui/nova_os_rtt_poc.rs` + its example-owned
shader, their three `[[example]]` blocks, the orphaned
`[package.metadata.nova_probe]` table, the `"gameplay"` row in
`CATEGORY_POLICIES`, and the `GAMEPLAY` smoke list with its test. The
REDUCTION stripped probe enrollment from all six enrolled `screenshots/`
producers, deleted `screenshot_reel`'s `reel_smoke_probe` backstop, and
converted the five beat-script producers onto `AutopilotPlugin` step
timelines.

The conversion is the substance. Each producer had a `*Script` resource of
one-shot booleans plus a `playing_since` wall-clock origin, re-deriving what
the driver already owns; `screenshot_nova_os` additionally hand-rolled
`HarnessCompletion::done`. All of it became named steps: 3 for `orbit`, 4 for
`juice`, 15 for `combat`, 6 for `ui`, 13 for `nova_os`. The load gate is now
`player_ship_present()` (or `state_is`) rather than a guessed hold, so a slow
load delays the beats instead of eating them, and a beat that never resolves
is an error exit NAMING that step instead of a silent pass.

Two structural wins fell out. Every capture owns a step, so the
one-capture-per-frame rule is enforced by shape rather than by a `shot_*`
guard. And `screenshots/` now has one idiom: no producer holds a wall-clock
origin, a beat boolean, or its own completion.

### Alternatives considered

- Leave `screenshot_ui`/`screenshot_nova_os` on their stage machines: the DoD
  greps only force `orbit`/`juice`/`combat`. Rejected per DECISION.md 3 - a
  surviving counter-example invites the next writer to copy it.
- Keep `combat`'s chip entities in a resource across the spawn/teardown steps.
  Rejected: naming the two spawns (`CHIP_NAMES`) and re-finding them by `Name`
  in the teardown step drops the resource entirely, which is what the DoD's
  "no script-owned state" is actually about.

### Difficulties and diagnosis

- `capture_window` is ASYNCHRONOUS: it spawns a bare `Screenshot` with a
  `save_to_disk` observer and registers NO completion collector. The old
  scripts never noticed because they idled out the autopilot runway after the
  last capture. Under step timelines the driver reports done the moment the
  last step ends, so a naive conversion would have exited before the final PNG
  landed. Every converted producer therefore ends on a capture step whose
  `until` is the settle hold (20 frames capturing, 2 otherwise) - this is
  load-bearing, and it is the one place the conversion ADDS rather than
  removes. `nova_os`'s existing per-beat settles (40/20 capturing, 6/2 smoke)
  were carried over verbatim, not re-derived, per DECISION.md 3.
- `cargo check --examples` does not compile `tests/`, so a leftover `GAMEPLAY`
  reference in `catalog_matches_disk` (`:149`, outside the two ranges the
  Steps named) survived the example check and only surfaced under `cargo
  test`. Caught and fixed.
- The task Notes claimed no documentation names the retired examples. That is
  WRONG: `web/src/wiki/dev/development.md` carried a `gameplay/` catalog
  bullet, a `gameplay` smoke-category reference, the "being retired" paragraph
  and a line asserting `screenshot_nova_os` self-ends. All four updated. The
  original sweep likely used `rg -r` (ripgrep's REPLACE flag), which silently
  rewrites matches instead of recursing.

### Evidence

- DoD 1 `! rg -n 'broadside|lifeline|nova_os_rtt_poc' Cargo.toml examples tests` - no hits.
- DoD 2 `! rg -n 'playing_since|get_or_insert|HarnessCompletion' examples/screenshots` - no hits.
- DoD 3 `! rg -n 'nova_probe::' examples/screenshots` - no hits.
- DoD 4 `! rg -n 'TRANSITIONAL: remove with 20260804-093910' crates` - no hits.
- `cargo fmt --check` clean; `cargo clippy --examples --features debug` adds no
  warning in any changed file (the `nova_ui/src/hud.rs` doc warnings are
  pre-existing and untouched).
- `cargo test --test examples_smoke` under Xvfb :99 - 7 passed, 0 failed
  (93.44s), including `screenshots_reach_playing_without_panic`,
  `catalog_matches_disk` and `every_category_has_a_probe_policy`.
- `cargo test -p nova_probe` - 102 passed, 0 failed across its five targets.
- All seven smoked producers RUN individually under Xvfb: exit 0, `reached
  Playing`, `autopilot: cycle complete, no panic`. `screenshot_nova_os` now
  reports the driver's `cycle complete` in place of its old `probe: script
  complete, exiting`, as the conversion intends. Every declared step logged
  its `begins` line, so no beat was skipped.
- DoD 7, the capture path. `NOVA_AUTOPILOT=1 NOVA_REEL=1
  NOVA_SHOT_DIR=target/reel` under Xvfb for the five converted producers, all
  exit 0. `ls target/reel` - 15 PNGs, every one non-empty, exactly the set
  these producers wrote before (orbit 1, juice 1, combat 7, ui 2, nova_os 4):

  ```
  devlog5-target-viewfinder.png  1334849   nova-os-map.png          1305424
  feature-autopilot.png          1254663   nova-os-ship.png         1347958
  feature-combat.png             1334848   nova-os-welcome.png      1333125
  feature-editor.png             1213725   tutorial-combat-lock.png 1334864
  feature-hud.png                1336650   tutorial-menu.png        1791677
  feature-juice.png              1141388   tutorial-orbit.png       1322517
  hud-nav-chips.png              1368491   tutorial-radar-lock.png  1374723
  nova-os-active.png             1430272
  ```

### Reflection

The step vocabulary was genuinely sufficient - no additions to
`nova_autopilot`/`nova_debug`, as the Notes predicted. What the Notes did not
predict is that the driver's completion is EAGER where the old runway was
lazy, and that difference is invisible to every `cmd:` proof and to the smoke
test: a producer that exits before its PNG lands still reaches Playing and
still completes its cycle. Only the DoD's `ls target/reel` catches it. That
proof earned its place, and a future conversion of a capture producer should
treat "what holds the run open for the last write" as the first question, not
the last.

The stale-doc miss is the reusable lesson: a Notes claim of the form "checked
X, no hits" is only as good as the command behind it, and `rg -r` is a trap
that reports success while doing something else entirely. Re-run the sweep
during work rather than trusting the planning-time result.
