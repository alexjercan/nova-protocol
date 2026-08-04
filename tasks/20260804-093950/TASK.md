# Deepen sections/ to multi-scene multi-round runs, merging com_range and torpedo_guidance

- PRIORITY: 80
- TAGS: v0.10.0, content, examples, testing
- KIND: STORY
- ACTIVITY: WORKING
- GATES: PLAN
- RESOLUTION: -
- PARENT: 20260802-115955
- DEPENDS ON: 20260804-003244, 20260804-093855

## Story

Deepen the `sections/` runs and collapse seven examples into five, one per
section family.

Today's section runs are mostly one scene, one section, one beat, with
wall-clock runways where an assertion belongs. With predicate steps a run can
walk several rounds across at least two scenes or section layouts - spawn,
drive, damage, destroy, reload, re-enter, assert the invariant again - and gate
each beat on the value it depends on rather than sleeping past it.

## Steps

Owner call 2026-08-04: "deepen" is bounded by a NAMED INVARIANT LIST per run,
not by a scene or round count. Done means every listed invariant is asserted.
Scenes and rounds are means to that end - if an invariant needs two scenes, the
run gets two scenes; if it does not, padding one in proves nothing.

ROSTER CONFIRMED at planning with one amendment: torpedo #5 is NEW, not
merged. `torpedo_guidance` only LOGS closest approach
(`track_closest_approach` -> `BestApproach`, torpedo_guidance.rs:234-245) and
asserts nothing; its autopilot is `nova_autopilot().input(autopilot_fire)`
(:60). Total stays 27: 14 exist, 4 merged, 9 new.

Mechanics all five runs adopt (see DECISION.md):

- Explicit `AutopilotPlugin` step list per run, replacing the
  `nova_autopilot()` wall-clock preset and the hand-rolled `elapsed <` stage
  offsets. `com_range.rs:70-101` and `systems/outcomes.rs:76-155` are the
  shapes to copy; every step carries `.deadline(...)` so a stall names the
  beat. Keep the deadline sum well under `DEFAULT_DEADLINE_SECS` (120s).
- RELOAD is two steps, no new API and no state:
  `.step("tear the rig down").on_enter(<re-trigger LoadScenario>)
  .until(nova_autopilot::predicate::not(any_entity::<With<SpaceshipRootMarker>>()))`
  then `.step("wait for the fresh rig").until(any_entity::<...>())`. The gap
  is real: `on_load_scenario` despawns scoped entities immediately and the
  ship respawns later off `OnStartEvent`
  (`nova_scenario/src/loader/lifecycle.rs:161-253`). `not` must be qualified -
  it clashes with `bevy::prelude::not`.
- Every invariant emits exactly ONE `nova_probe::probe_marker` named
  `outcome: <slug>` beside its `assert!`. The names are the roster; Step 6
  pins them. Existing markers that cover two invariants get split.
- `AutopilotPlugin` must be named `nova_protocol::nova_debug::harness::` -
  qualified (`examples_name_drivers_through_the_nova_harness` enforces it).

Each step below is an atomic green checkpoint: run the example before
committing (`Xvfb :99 & DISPLAY=:99 NOVA_AUTOPILOT=1 nix develop --command
cargo run --example <name> --features debug`). `cargo check` does not catch
duplicate-component panics.

- [ ] 1. `controller_section` (examples/sections/controller_section.rs) - 4
      invariants (2 exist, 2 new). Script: load rig A (controller + hull) ->
      assert 1 (command swept > 0.6 rad, :200) and 2 (tracking error <
      `TRACK_TOLERANCE_RAD`, :213) -> reload rig A -> assert 4 (tracking
      re-converges) -> load rig B (layout with a second hull offset off-axis,
      so the inertia tensor differs) -> assert 3 (tracking holds). Two rig
      builders, both local. Wait for tracking with a predicate on the live
      `Rotation`-vs-command error, not `elapsed`.
- [ ] 2. `thruster_section` (examples/sections/thruster_section.rs) - 5
      invariants (3 exist, 2 new). Script: full burn -> assert 1 (nose speed
      grew, :189), 2 (plume material exists, :204), 3 (plume input == 1.0,
      :216) -> partial throttle round -> assert 4 (nose-speed gain at the
      partial setting is positive and strictly below the full-burn gain
      measured in round 1) -> release -> assert 5 (plume input returns to 0).
      Invariant 4 compares two MEASURED gains rather than a magic constant.
- [ ] 3. `hull_section` (examples/sections/hull_section.rs) - 8 invariants (3
      exist, 4 merged from `com_range`, 1 new). The rig becomes `com_range`'s
      5-section line (controller, hull1..3, thruster) under
      `SpaceshipController::Player` with an empty `input_mapping` - required,
      not cosmetic: the chase camera (invariant 7) only follows a player ship.
      Round order matters:
      R1 damage a REAR hull: assert 1 (partial hit subtracts exactly, :193),
      2 (overkill destroys and despawns, :220), 3 (root + controller survive,
      :230,:234) - all three need the controller alive, so R1 comes first.
      R2 spin, kill the controller, kill hull1, settle: assert 4 (drift < 0.3,
      com_range.rs:381), 5 (local COM moved aft), 6 (`TransformInterpolation`,
      :394), 7 (camera anchor drift < 0.5, :408). RECOMPUTE the aft threshold
      from the surviving set - R1 already removed a rear hull, so
      com_range's `local_com.z > 2.4` / "near 2.75" no longer describes this
      run. Port `com_range`'s gizmos, hotkeys and status log with the beats.
      R3 reload, repeat R1+R2: assert 8 (1-7 hold again).
      Then delete examples/sections/com_range.rs, its `[[example]]` block
      (Cargo.toml:82-84) and its `SECTIONS` entry (tests/examples_smoke.rs:39).
- [ ] 4. `turret_section` (examples/sections/turret_section.rs) - 4 invariants
      (2 exist, 2 new). Script: fire round -> assert 1 (a round fired, :478)
      and 2 (a gate took hits, :479) -> assert 3 (aim error vs the SWEEPING
      gate converges: the error `report_status` already computes, :397, falls
      below a stated threshold while the gate is moving) -> reload -> assert 4
      (1-3 again). Keep `turret_section/slider.rs` untouched.
- [ ] 5. `torpedo_section` (examples/sections/torpedo_section.rs) - 6
      invariants (4 exist, 1 merged scene, 1 new). Script: gate round ->
      assert 1-4 (fired/armed/detonated/gate damaged, :517-:523) -> load the
      crosser scene (port `torpedo_guidance`'s crossing target, its
      `range_autotarget` lock-at-launch and its `BestApproach` tracker) ->
      assert 5 (closest approach falls under a stated bound - NEW, the source
      run only logged it) -> assert 6 (the fired/armed/detonated chain
      repeated in this second scene). Keep the run's longer runway budget
      (`RANGE_AUTOPILOT_SECS`, :52) as per-step deadlines.
      Then delete examples/sections/torpedo_guidance.rs, its `[[example]]`
      block (Cargo.toml:78-80) and its `SECTIONS` entry
      (tests/examples_smoke.rs:38).
- [ ] 6. Pin the roster: `sections_assert_their_invariant_roster` in
      tests/examples_smoke.rs - a display-free source grep (same class as
      `examples_name_drivers_through_the_nova_harness`, :217) asserting each
      sections example contains each of its named `outcome: <slug>` markers,
      and that the totals are 27 across the five files. This is the stopping
      rule made executable; the run passing is what proves they HOLD.
- [ ] 7. Docs in the same task: `web/src/wiki/dev/development.md:162-163`
      (drop both merged names from the sections list), `:220` (`torpedo_guidance`
      cited as a scenario-load asserter), `:251` (`com_range` cited as the
      wait-on-the-world style - repoint to `hull_section`);
      `web/src/wiki/dev/automation-harness.md:115-129` (the before/after
      worked example is `com_range`'s script - repoint to `hull_section`);
      one `Internals & Tooling` CHANGELOG line under `[Unreleased]`. Historical
      CHANGELOG and `web/src/news/0.4.0.md` mentions stay - they are release
      history, not live references.
- [ ] 8. Build the ship fixture LOCALLY here. Do NOT extract a shared builder:
      owner call 2026-08-04, `20260804-094006` is the third caller and does the
      extraction, having seen all three shapes. One caller is not an
      abstraction. This applies to the ship builders only - the reload gate is
      two lines of existing predicate vocabulary per run, not a helper.

## Definition of Done

- Every one of the 27 invariants is NAMED by its run: the roster test lists
  them and each is present as an `outcome: <slug>` marker beside its assert.
  Red on base (no sections example carries a roster today).
  (test: `sections_assert_their_invariant_roster`)
- Every invariant HOLDS: all five runs pass headless, where a failed assert
  panics the process and `process_exit` fails the report
  (nova_probe/src/run_report/checks.rs:130-133).
  (cmd: `nix develop --command cargo run -p nova_probe -- run sections`)
- No run advances a beat on `elapsed` where a world value is available: every
  script is an `AutopilotPlugin` step list, and `elapsed`/`frames` appear only
  as settle beats with a stated reason.
  (manual: reviewer reads the five scripts)
- The two merged runs are gone and their assertions live in the absorbing run.
  Red on base (both names are still referenced).
  (cmd: `! rg -n 'com_range|torpedo_guidance' Cargo.toml examples tests`)
- The catalog, disk and smoke lists agree after the merges. A CONSISTENCY
  GATE, not a red proof: green on base, and it goes red the moment a file is
  deleted without its catalog and smoke entries.
  (test: `catalog_matches_disk`)

## Notes

Roster per the spike (`20260804-003244`) - each run gets harder, not thinner:

| Run | Change |
| --- | --- |
| `controller_section` | Deepen. PD attitude control across multiple layouts and repeated rounds. |
| `thruster_section` | Deepen. Throttle -> impulse + plume, same shape. |
| `hull_section` | Deepen, ABSORBS `com_range`. |
| `turret_section` | Deepen. PDC tracking + firing. |
| `torpedo_section` | Deepen, ABSORBS `torpedo_guidance`. |

The two merges:

- `com_range` -> `hull_section`. `hull_section` owns the damage -> destroy
  pipeline, and COM-follows-destruction is that pipeline's consequence, not a
  separate subject. `com_range.rs:374` (`assert_com_follows_sections`) becomes
  a round after the destroy round. `com_range` is already predicate-driven, so
  the beats port directly.
- `torpedo_guidance` -> `torpedo_section`. Both are the torpedo bay family and
  one example per family is the contract. The PN closest-approach assertion
  becomes the lead-a-crosser round of the merged run.

- Assert through predicates on the values the section family owns (mass/COM,
  thrust, integrity, guidance, lock, range), not through elapsed time.
- `turret_section` carries a 203-line interactive slider submodule for human
  tuning. It stays; if the probe path never touches it, a later task may
  extract it to a shared dev-widget module. Not blocking here.
- Ship builders stay LOCAL here. `20260804-094006` extracts the shared `fn`
  with its count knob as the third caller (owner call 2026-08-04), designing
  the signature from three visible shapes instead of from this one.
- `sections/` carries no fps window.
- Examples must be RUN under Xvfb :99, not only checked.
