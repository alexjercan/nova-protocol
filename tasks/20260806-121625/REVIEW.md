# Review: Refactor nova_* crate for better structure and clarity

- TASK: 20260806-121625
- BRANCH: refactor/l11-perf-correctness

## Round 1

- REVIEWER: out-of-context (three lanes: behavior/proofs,
  correctness/tests, design/standards/docs)
- VERDICT: REQUEST_CHANGES

Lane 11 only. The gates are green and the record is honest about its
numbers - `cargo check --workspace --all-targets`, `cargo clippy
--workspace --all-targets -- -D warnings` and `cargo fmt --all -- --check`
all exit 0 here, and the recorded lib-test counts reproduce exactly
(nova_ship 414, nova_gameplay 136, nova_scenario 155, nova_hud 207). The
`ScenarioConfig` transform is sound: the `Default` derive is genuinely
removed, so the COMPILER, not the script, proves the 39 sites were
converted, and no authored field line was dropped anywhere in the diff.

The open findings are one wrong-clock regression and a set of shipped
behavior changes with no test at their own boundary.

- [ ] R1.1 (MAJOR) crates/nova_ship/src/input/ai/mod.rs:108 - the AI chain
  moved to `FixedUpdate` still reads the RENDER clock, and the NOTE
  justifying the move asserts the opposite ("every pose it reads and every
  intent it writes is fixed-clock"). The moved systems read `&Transform`
  at `acquisition.rs:141,151,260,269`, `behavior.rs:264,277`,
  `guns.rs:37,193`, `maneuver.rs:145,155,230,240`, `torpedo.rs:104,111`
  and `&GlobalTransform` at `guns.rs:182` (muzzle) and `maneuver.rs:224`
  (thruster). `web/src/wiki/dev/architecture.md:229-246` states this rule
  and names it as the exact bug `thruster_impulse_system` was fixed for:
  during `FixedUpdate` of frame N, `GlobalTransform` holds frame N-1's
  eased pose. In `Update` these reads were correct, so the move introduces
  the staleness. Point the pose reads at avian `Position`/`Rotation` (or
  compose the root's raw pose with the local mount offset), or take the
  plan's other option - leave the chain in `Update` and tick only the
  firing gates off `Time<Fixed>`. If the staleness is deliberately
  accepted, rewrite the NOTE to say so with its reasoning instead of
  claiming compliance.
  - Response: Moved back. `update_fire_cadence` alone stays in
    `FixedUpdate` - it reads no pose and its expiry writes the trigger
    `shoot_spawn_projectile` consumes there - and the other twelve
    systems return to `Update` with the eased poses they read. Both
    NOTEs rewritten: one says why the cadence may move, the other names
    the pose reads that hold the rest back and cites `architecture.md`.
    F24's close-out records the wrong first ruling rather than hiding
    it.
- [ ] R1.2 (MAJOR) crates/nova_ship/src/lib.rs:137 - the new `FixedUpdate`
  gate for `SpaceshipInputSystems` (and its twin at
  `crates/nova_scenario/src/loader/lifecycle.rs:29`) is now the only thing
  stopping the relocated AI chain from flying and firing while paused or
  after scenario teardown, and neither rig tests it:
  `spaceship_sets_freeze_while_paused` (`lib.rs:157`) registers its
  `Ticks` probe in `Update` only, and lifecycle's `gated_app` probes
  input-in-`Update` and sections-in-`FixedUpdate` but never
  input-in-fixed. Add a second probe system
  `.in_set(input::SpaceshipInputSystems)` in `FixedUpdate` to both rigs
  and assert it freezes on both edges (Paused, and not-live).
  - Response: Both rigs gained a `FixedUpdate` probe in
    `SpaceshipInputSystems`. `nova_ship/src/lib.rs`: new
    `spaceship_sets_freeze_in_fixed_update_while_paused`, asserting on
    both edges (runs Unpaused, frozen Paused, resumes after).
    `lifecycle.rs`: `Ticks` gained `input_fixed`, `ticks()` returns a
    4-tuple, and both gate tests assert it on the not-live and live
    edges.
- [ ] R1.3 (MAJOR) crates/nova_ship/src/input/ai/mod.rs:115 - the schedule
  move itself - the largest behavior change on the branch, since target
  selection, behavior state and every firing gate now advance at a fixed
  64 Hz instead of per render frame - has no test at its own boundary. The
  AI rigs at `input/ai/passive.rs:713` and `input/ai/maneuver.rs:455`
  already registered these systems in `FixedUpdate` themselves, so they
  passed before and after and prove nothing about production wiring. Add
  one test that builds the app through `SpaceshipAIInputPlugin`, drives N
  `FixedUpdate` steps under two different `Update` frame deltas, and
  asserts the fire-cadence timer advanced by the same amount both times -
  that is the claim the move was made for.
  - Response: `the_burst_window_closes_on_a_fixed_step_not_a_frame_boundary`
    builds the app through `SpaceshipAIInputPlugin` and drives one 2s
    frame - 128 fixed steps spanning the whole 1.5s window - sampling
    `AIFireCadence::firing` after the set on every step. It asserts the
    window closes on the expected fixed step, not at the frame boundary.
    Note the claim changed with R1.1: rate is invariant either way (a
    frame's delta is the sum of its fixed deltas), so the test pins
    GRANULARITY, which is the property `shoot_spawn_projectile` actually
    needs.
- [ ] R1.4 (MAJOR) crates/nova_ui/src/widget/slider.rs:30 - the
  `slider_meter_color` correction (2% must light 1 block, 98% must leave
  one dark) ships untested. The only existing test,
  `sync_slider_tracks_lights_blocks_from_value` at `slider.rs:372`, probes
  0.0 and 1.0 - the exact two fractions the change leaves identical - so
  deleting the fix leaves the suite green. Add direct asserts:
  `slider_meter_color(0, 0.02) == theme::PHOSPHOR`,
  `slider_meter_color(SLIDER_SEGMENTS - 1, 0.98)` dim, plus the unchanged
  0.0/1.0 endpoints.
  - Response: `the_meter_reserves_a_block_at_each_end` asserts
    `slider_meter_color(0, 0.02) == PHOSPHOR` and
    `slider_meter_color(SLIDER_SEGMENTS - 1, 0.98)` dim, plus the
    0.0/1.0 endpoints as the guard that the clamp did not cost
    exactness.
- [ ] R1.5 (MAJOR) crates/nova_gameplay/src/transform/directional_sphere_orbit.rs:114
  - the angle-seam fix changes live camera behavior but is pinned only by
  `normalize_angle`'s own unit test in `math.rs`;
  `directional_sphere_orbit.rs` has no `mod tests` at all, so deleting the
  `unwrapped_theta` line stays green. The close-out names this itself. Add
  an `App` rig that spawns the orbit with `state.theta = PI - 0.05`, sets
  an input direction whose `direction_to_spherical` theta is `-PI + 0.05`,
  runs one update at a small dt, and asserts `state.theta` moved across
  the seam by about the per-step lerp rather than sweeping back toward 0.
  - Response: `easing_across_the_seam_takes_the_short_way` added as the
    file's first `mod tests`: `TimePlugin` + the real plugin, orbit
    seeded at `PI - 0.05`, input direction at `-PI + 0.05`, one update.
    It asserts the signed short-way delta is positive and within the
    per-step lerp, so a sweep back through 0 fails it.
- [ ] R1.6 (MINOR) crates/nova_ship/src/flight/thrusters.rs:305 -
  `spool_allocated_thrusters` heap-allocates a fresh
  `HashMap<Entity, usize>` per ship per fixed tick, where the old
  duplicated loops allocated nothing. That is a NEW per-frame allocation
  in the very system the change was meant to speed up, and the probe
  measured no mean gain. Either take a `Local<HashMap<Entity, usize>>`
  from each caller and `clear()` it per ship, or keep the linear
  `position()` scan over `allocation` (N is small) and hoist nothing.
  - Response: `HashMap` dropped; the body keeps the linear `position()`
    scan over `allocation`, and the docstring now records that N is
    per-ship engines (single digits) and that the map traded the scan
    for a per-tick allocation the probe measured no gain from.
    `bevy::platform::collections::HashMap` import removed. F38's
    close-out updated.
- [ ] R1.7 (MINOR) crates/nova_gameplay/src/math.rs:65 - `snap_tolerance`
  scales by the target's distance from the ORIGIN, not by the size of the
  remaining step, so for the `Vec3` impl the threshold grows without bound
  with world position: a chase camera target at 1e6 units snaps once
  within ~0.12 world units, a visible cut of the last easing. Clamp the
  scale - `f32::EPSILON * magnitude.abs().clamp(1.0, 1.0e4)` - and extend
  `snap_tolerance_scales_with_the_target` with the clamped case.
  - Response: `snap_tolerance` clamps to `f32::EPSILON *
    magnitude.abs().clamp(1.0, 1.0e4)`.
    `snap_tolerance_scales_with_the_target` extended:
    `snap_tolerance(1e6) == snap_tolerance(1e4)` and stays under 2e-3.
    The doc records why the upper bound exists.
- [ ] R1.8 (MINOR) crates/nova_ship/src/sections/turret_section/mod.rs:211
  - `DefaultProjectileRender` is inserted from a `Startup` system while
  `insert_projectile_render` takes it as a plain `Res`, so any
  `TurretBulletProjectileMarker` added before that Startup command flushes
  fails the resource lookup - a hard error under the
  `FallbackErrorHandler(panic)` the autopilot and probe runs install.
  Replace the `Startup` system with `impl FromWorld for
  DefaultProjectileRender` plus `app.init_resource::<DefaultProjectileRender>()`
  in the same `self.render` branch; that deletes the ordering coupling
  rather than documenting it.
  - Response: `init_default_projectile_render` deleted; `impl FromWorld
    for DefaultProjectileRender` plus
    `app.init_resource::<DefaultProjectileRender>()` in the same
    `self.render` branch. The existing test now inits the resource and
    spawns a bullet before any flush on its behalf, so the ordering the
    finding named is covered rather than documented.
- [ ] R1.9 (MINOR) crates/nova_scenario/src/objects/area.rs:61 -
  `forget_area_occupancy` is now dead. A key is only ever inserted when
  the area side has `EntityId` (both collision handlers require it via
  `q_area`), and no code path removes `ScenarioAreaMarker` without
  despawning, so `forget_body_occupancy`'s `*area != remove.entity` half
  already covers every case it did. Delete `forget_area_occupancy` and its
  `add_observer` line, folding its "avian fires no `CollisionEnd`"
  sentence into the survivor's doc.
  - Response: `forget_area_occupancy` and its `add_observer` line
    deleted; the plugin doc drops to one occupancy observer.
    `forget_body_occupancy`'s doc now covers both sides of the pair and
    records that areas carry `EntityId` too, which is why one observer
    suffices.
- [ ] R1.10 (MINOR) crates/nova_scenario/src/objects/area.rs:68 - the doc
  on `forget_body_occupancy` claims it restores "the `OnExit` the scenario
  gates on", but the only `OnExitEvent` fire is in
  `on_collision_end_event:178`; the new observer prunes the row silently,
  so a body destroyed inside a live area still produces no `OnExit` for
  itself. The leak fix is real - the NEXT body can reach zero again - but
  the stated symptom is only half addressed. Either fire `OnExitEvent`
  when the pruned row's count was non-zero and `CurrentScenario` is live,
  or narrow the doc to "prune only; a destroyed body fires no exit".
  - Response: Doc narrowed. It now says PRUNE ONLY in as many words: the
    row is dropped silently, the only `OnExitEvent` is the 1 -> 0
    transition in `on_collision_end_event`, and what the observer
    restores is the NEXT body's ability to reach zero. Firing the event
    was not taken - a scenario that gates on `OnExit` for a destroyed
    body wants the destruction event, and synthesising an exit from a
    despawn observer is a behavior change wider than this lane.
- [ ] R1.11 (MINOR) crates/nova_hud/src/readout.rs:173 - the reused
  `Local<String>` scratch is exercised only by a single-readout test, so
  the classic reused-buffer regression - a missing `out.clear()` in
  `write_readout` leaving a stale suffix on a shorter second row - would
  not be caught. Extend `rows_reconcile_and_clear_on_empty` to two
  readouts where the second renders shorter than the first, and assert
  both row texts.
  - Response: `a_shorter_second_row_does_not_inherit_the_longer_first`
    renders `TIME 01:23.4` then a bare `7` through the real reconcile
    and asserts both row texts. The `write_readout` unit test also now
    writes both cases into ONE buffer, so a missing clear fails there
    too.
- [ ] R1.12 (MINOR) crates/nova_hud/src/readout.rs:62 -
  `HudReadoutFormat::render_into` is a new `pub` item with no caller
  outside the crate: its only two callers are `render` (same impl) and
  `write_readout` (same file). Drop it to private.
  - Response: `render_into` is private.
- [ ] R1.13 (MINOR) tasks/20260806-121625/TASK.md:1674 - the F82
  drop-with-reason states that "`radar.rs` and `component_lock.rs:403` do
  not exist". `component_lock.rs` is indeed 311 lines, but `radar.rs` DOES
  exist at `crates/nova_ship/src/input/targeting/radar.rs`, and line 387
  is inside its `#[cfg(test)]` rig - so the row is falsified for the same
  reason as the other three, not because the file is gone. Correct the
  sentence to name `radar.rs:387` as a fourth `&mut World` test helper at
  a moved path.
  - Response: Corrected. The row now names `component_lock.rs:403` as
    the one path that does not exist, and lists `radar.rs:387` as a
    third falsified row - a `&mut World` rig builder inside the file's
    `#[cfg(test)]` module, not a system param.
- [ ] R1.14 (MINOR) tasks/20260806-121625/TASK.md:3089 - the F72 evidence
  claims "`cargo check` proves no field was dropped or duplicated". It
  does not: under `..ScenarioConfig::new(..)` a dropped explicit field
  silently falls back to `new`'s empty value and still compiles. The
  transform IS clean - no removed line other than
  `id`/`name`/`cubemap`/`..Default::default()` left any literal, verified
  across the diff - so restate the proof as that diff review rather than
  as a compiler guarantee.
  - Response: Restated. The close-out now says `cargo check` does NOT
    prove field preservation and explains why (a dropped field falls
    back to `new`'s empty value under the struct-update syntax), then
    names the diff read across all 39 sites as the actual proof.
- [ ] R1.15 (MINOR) crates/nova_assets/tests/mod_binary_resources.rs:498 -
  the scripted transform dropped the trailing
  `// NOTE: no dependencies: ["base"] - base is implicit` comment, which
  was what told a reader what that fixture pins. This contradicts the
  close-out's "the three moved expressions kept verbatim", and the global
  AGENTS.md rule to keep comments that guard a value. Restore it on the
  `ScenarioConfig::new(..)` line.
  - Response: Comment restored on the `ScenarioConfig::new(..)` line.
- [ ] R1.16 (MINOR) tasks/20260806-121625/DECISION.md:94 - three rulings of
  the same class as the F66/F61 rows already in "Settled since" got no
  entry: F24 (move the whole AI chain's schedule, which forced two new
  gate configurations), F67 (magnitude stays a per-tick impulse) and F82
  (dropped as falsified). Add three rows naming each ruling and its
  consequence.
  - Response: Three rows added to `Settled since`: F24 (cadence-only
    move, with the two gate configurations as its consequence), F67
    (per-tick impulse stands, with the migration it defers), F82
    (dropped as falsified, with no edit as its consequence).
- [ ] R1.17 (NIT) crates/nova_gameplay/src/transform/smooth_look_rotation.rs:14
  - `use crate::math::normalize_angle` is a deep intra-crate path where
  all three sibling files in `transform/` (`directional_sphere_orbit.rs`,
  `random_sphere_orbit.rs`, `sphere_orbit.rs`) use
  `use crate::math::prelude::*`, which this diff extended with
  `normalize_angle`. Change it to match.
  - Response: `use crate::math::prelude::*`, matching the three
    siblings.
- [ ] R1.18 (NIT) crates/nova_ship/src/input/ai/mod.rs:79 -
  `SpaceshipAIInputPlugin` now straddles two schedules
  (`mirror_ai_combat_state` alone in `Update`, the twelve-system chain in
  `FixedUpdate`) and neither the plugin doc nor the module doc says so.
  Add one line naming the split and why `mirror_ai_combat_state` stayed.
  - Response: The plugin doc now names the split and why: the cadence is
    in `FixedUpdate` for the clock, everything else in `Update` for the
    eased poses, both in `SpaceshipInputSystems` so the gates cover
    either schedule.
- [ ] R1.19 (NIT) crates/nova_hud/src/readout.rs:222 - `format_readout`
  survives only as a `#[cfg(test)]` wrapper around `write_readout`. Have
  its two asserting tests write into a local `String` and delete the
  wrapper.
  - Response: `format_readout` deleted; both asserting tests write into
    a local `String`.
- [ ] R1.20 (NIT) crates/nova_ui/src/widget/button.rs:244 - the new
  `Danger` pressed arm duplicates the hover `Paint` for 12 lines, differing
  only in `gradient` and `shadow`. Compute those two fields with a small
  `if pressed` and keep one `Paint` literal.
  - Response: One `Paint` literal for `pressed || hovered`; `gradient`
    picks the inverted `grad2` when pressed and `shadow` is
    `(!pressed).then(drop_shadow)`, with a comment naming the
    sunk-vs-raised intent.
- [ ] R1.21 (NIT) crates/nova_assets/src/merge.rs:530 - the two fixtures
  that gained a cubemap (here and
  `crates/nova_menu/src/tests/scenarios.rs:89`) pass a bare
  `"cubemap.png"`, which `web/src/wiki/dev/guide-author-scenario.md:38`
  says a cubemap must never be ("never bare/scheme-less"). Use
  `"dep://base/textures/cubemap.png"` so the fixtures do not model the
  shape the authoring guide forbids.
  - Response: Both fixtures pass `dep://base/textures/cubemap.png`.
- [ ] R1.22 (NIT) crates/nova_gameplay/src/shake.rs:286 - decorrelating the
  shake offset from the kick changes camera behavior and doubles the RNG
  draws per shaking camera per frame, with no test;
  `camera_recenters_and_does_not_drift` and
  `shake_offset_stays_within_the_configured_bound` pass either way. Assert
  over ~50 frames that the kick axis is not parallel to the offset.
  - Response: `the_kick_axis_is_not_locked_to_the_offset` runs 50
    shaking frames with uniform peaks on both - the case where a shared
    sample gives an exactly parallel axis - and requires the kick axis
    to be non-parallel to the offset on more than 40 of them.
- [ ] R1.23 (NIT) tasks/20260806-121625/TASK.md:3092 - the "lib tests green
  across every touched crate" list omits `nova_ui`, which carries the
  button and slider changes. Add `nova_ui 29`.
  - Response: `nova_ui 29` added to the list.

Process signal: the lane's RE-MEASURE rule paid a fourth and fifth time
(F72 was 39 sites not 15, F44 was 8 not 14, F82 falsified outright, F85
had three moved paths). Every correction is recorded under its step rather
than quietly absorbed.

Process signal: the probe step is unusually honest - it records that the
FPS win F37/F38 predicted did NOT appear, keeps the wrong prediction
visible, and names host noise as the likely cause of the one anomaly. The
consequence, though, is that F37's success now rests on a unit test alone,
and F38 shipped a new per-tick allocation (R1.6) that was never
re-measured.

Process signal: `tatr proofs` lists only epic-level DoD proofs, none
lane-level, so the proof list gave this review nothing to run. L11's steps
carry their evidence inline instead.

Process signal: four of the branch's behavior changes (AI schedule, pause
gate, slider meter, orbit seam) are acknowledged as untested in TASK.md
itself. The acknowledgement is accurate and welcome, but it is not a
substitute for the tests, which is why R1.2-R1.5 stand.

Out of scope: `cargo check`/`clippy` emit a `proc-macro-error2 v2.0.1`
future-incompatibility warning on every run - already filed as F84 in the
epic's close-out list.

Out of scope: both occupancy observers do a full `HashMap::retain` per
despawned entity; during teardown that is O(entities x rows). Pre-existing
shape, amplified but not introduced here.

Out of scope: `sync_readout_rows` does an O(rows) `iter_mut().find()` per
readout per frame. Pre-existing.

Out of scope: `spool_allocated_thrusters` still iterates every thruster in
the world once per ship. Pre-existing.

Out of scope: `transform/sphere_orbit.rs:111` uses the same
`lerp_and_snap` on `theta` with no seam handling. Its input is an
unbounded accumulator so it does not have the bug, but the two orbits now
treat theta differently.

Verified by the recording pass, not taken from a lane: `cargo fmt --all --
--check` clean; `cargo check --workspace --all-targets` exit 0; `cargo
clippy --workspace --all-targets -- -D warnings` exit 0; working tree
clean at `11811ba4`. Lib tests re-run here: nova_ship 414, nova_gameplay
136, nova_scenario 155, nova_hud 207, nova_ui 29 - the four recorded
counts match exactly. The `ScenarioConfig` claim re-derived independently:
`#[derive(Default)]` is gone from the struct, no `ScenarioConfig::default`
reference survives anywhere in `crates/` or `examples/`, and the two
fixtures that previously built with NO cubemap (`nova_assets/src/merge.rs`
and `nova_menu/src/tests/scenarios.rs`) are the only two whose meaning
changed. R1.1 re-derived against `architecture.md` and the moved systems'
own query signatures; R1.2, R1.4 and R1.9 re-derived against the test
bodies and the insertion path.

Pending user checks: none. No `manual:` proof is open against Lane 11.

Inspection commands:

```bash
cd "$(sprout show refactor/l11-perf-correctness)"
git diff master...HEAD
nix develop --command cargo clippy --workspace --all-targets -- -D warnings
nix develop --command cargo test -p nova_ship --lib
sed -n '229,246p' web/src/wiki/dev/architecture.md
```

## Round 2

- REVIEWER: in-session, post-land (L11 squash `a6671015` on master)
- VERDICT: REQUEST_CHANGES

L11 landed before this round ran, so the findings were addressed directly
on master rather than on the lane branch. The substance of the lane holds
up: the `try_despawn` double-despawn fixes, the shared
`DefaultProjectileRender`, the `Activate`-over-`Pressed` change, the angle
seam fix and the relative snap tolerance are each correct and each carries
a test that fails without it.

- [ ] R2.1 (MEDIUM) crates/nova_ship/src/camera/skybox.rs:118 - the
  `unwrap()` became `error!` + `return`, but four comments still justify
  the deferred skybox install by saying the observer PANICS:
  `nova_scenario/src/loader/lifecycle.rs:183`, `:463`, the assertion
  message at `:501`, and `nova_scenario/src/loader/mod.rs:397`. The
  deferral is still required, but for a different reason - the observer is
  an `On<Insert, SkyboxConfig>`, so it runs once and never retries, and a
  camera that misses its image is skyless for the scenario's whole life.
  A reader who checks the stated reason and finds no panic deletes the
  deferral.
  - Response: All four rewritten to the no-retry reason. `skybox.rs` also
    gained the missing half of the contract at its own site: a fn docstring
    saying the observer runs once per insert and is never retried (naming
    `PendingSkyboxSwap` as why that is survivable), and a NOTE on the
    early return itself.
- [ ] R2.2 (LOW) crates/nova_scenario/src/objects/area.rs:72 -
  `forget_body_occupancy` widened from `On<Remove, ScenarioAreaMarker>` to
  `On<Remove, EntityId>`. Correct today (nothing removes `EntityId` from a
  live entity), but the observer is now reachable from every entity in the
  game on an unstated assumption: a bare `remove::<EntityId>()` on a live
  body inside a sensor drops its occupancy row, after which
  `on_collision_end_event`'s missing-row guard swallows the real exit and
  the area never fires `OnExit` for it.
  - Response: Not documented - removed. Bevy 0.19 has a despawn-only
    lifecycle event, so the observer is now `On<Despawn, EntityId>` and the
    assumption is gone rather than written down. The docstring records why
    `Despawn` and not `Remove`.
    `a_body_despawned_inside_an_area_drops_its_occupancy` still passes.
- [ ] R2.3 (LOW) crates/nova_ui/src/widget/button.rs:839 -
  `press_reads_differently_from_hover_in_both_skins` asserts cross-skin
  PARITY only, so it is green whenever a variant is equally broken on both
  skins. Two variants were: `Ghost` (phosphor collapses `hovered ||
  pressed` into one paint, the hardware arm never reads `pressed`) and
  `Primary` (both skins take an inverted early-out that ignores
  `pressed`). `Ghost` is what `segmented_option` builds, so the
  Graphics-preset and UI-skin rows had no press feedback at all - the same
  defect the Danger fix had just closed. The `face()` closure also compared
  gradients by stop COUNT, so a press expressed only in gradient colours
  would read as no reaction.
  - Response: Test widened first and confirmed failing ("Primary has no
    press feedback on Phosphor"): it now asserts `reacts_to_press` for
    every variant on every skin AND keeps the parity assertion, and
    `face()` compares whole gradients. Then fixed: phosphor's inverted
    face dims to `PHOSPHOR_LO` and drops its glow on press; phosphor
    `Ghost` gains its own pressed arm; hardware `Primary` and `Selected`
    invert their bevel and drop the shadow like every other hardware face;
    hardware `Ghost` takes a dark wash under a brighter border (no
    gradient - it is fill-less by contract). `nova_ui` lib 30/30.
- [ ] R2.4 (LOW) CHANGELOG.md - `## [Unreleased]` carries nothing for six
  user-visible changes on this lane, two of them modder-facing and so the
  ones most likely to be lost when the epic close-out reconstructs the list
  from 24 commits: `ScenarioConfig` losing `Default` (a source break) and
  the scenario lint newly warning on `auto_advance_secs: Some(0.0)` /
  `NextScenario` `delay: Some(0.0)` (third-party content starts emitting
  warnings).
  - Response: Six lines added under Gameplay & Flight (orbit seam), a new
    Combat & Weapons (AI cadence on the fixed clock), Modding & Mod Portal
    (the zero-delay lint), a new Interface & HUD (commit-on-release, the
    per-variant press faces, the slider inset, the block meter) and
    Internals & Tooling (the `ScenarioConfig` break). Each claim re-read
    against the shipped code before writing - the slider line quotes the
    ~3px the source comment measures, not a guess.
- [ ] R2.5 (NIT) crates/nova_scenario/src/loader/mod.rs:209 -
  `ScenarioConfig::new(id, name, cubemap)` takes two adjacent `String`s
  (`ScenarioId` is a type alias), so `new(name, id, cubemap)` compiles and
  silently yields a scenario whose id is its display name - a mistake the
  old struct-literal form made impossible. All 39 converted call sites are
  correct.
  - Response: No newtype and no call-site churn. Scenario ids are
    snake_case slugs and display names are not, so `new` now
    `debug_assert!`s the slug shape with a message naming the swap.
    Authored RON is unaffected (serde builds the struct directly), so this
    constrains in-repo builders only.
    `a_display_name_in_the_id_slot_is_rejected` pins it.

Verified for this round: `cargo fmt --all -- --check` clean; `cargo check
--workspace --all-targets` exit 0; `cargo clippy -p nova_ui -p
nova_scenario -p nova_ship --all-targets -- -D warnings` clean (the
`proc-macro-error2` future-incompat note is F84, out of scope). Lib tests:
nova_ui 30, nova_scenario 156 (155 + R2.5's), nova_ship 416, nova_menu 77,
nova_authoring 44, nova_assets 60 - all green. `content -- gen` re-run and
`assets/` came back byte-identical, which is also what proves every shipped
builder satisfies the new slug assert. `probe run widget_zoo` OK (6/7, the
7th is `fps_within_baseline`, not armed) - the interactive button run still
lights its pressed face (0.12 -> 0.2).

Not taken: the news post for these lines. `## [Unreleased]` is the right
home until a version is cut; the release skill owns the post.

Inspection commands:

```bash
git diff HEAD
nix develop --command cargo test --lib -p nova_ui
nix develop --command cargo test --lib -p nova_scenario
```
