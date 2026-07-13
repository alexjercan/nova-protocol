# Review: Audio SFX - thruster hum distance attenuation

- TASK: 20260711-183417
- BRANCH: fix/audio-hum-attenuation

## Round 1

- VERDICT: APPROVE

- [x] R1.1 (MINOR) crates/nova_gameplay/src/audio.rs:527 - stale doc reference: the
  `ensure_thruster_loop` doc still says "[`update_thruster_loop_volume`] raises its
  volume with thrust", but that system was deleted in this branch. Point it at
  `compute_thruster_hum_volume` / `apply_thruster_loop_volume` instead. (This was the
  only live-code hit of the stale-reference grep; the two hits in
  docs/plans/20260713-v0.5.2-plan.md are a historical mechanism record and are fine.)
  - Response: fixed - ensure_thruster_loop doc now points at
    compute_thruster_hum_volume / apply_thruster_loop_volume.

- [x] R1.2 (MINOR) crates/nova_gameplay/src/audio.rs:896 -
  `a_distant_ships_burn_does_not_raise_the_hum` is a "stays zero" assertion with no
  in-test delivery guard (docs/LESSONS.md `delivery-guards-on-null-assertions`,
  promoted to the review skill): if `spawn_burning_ship` ever drifted out of the
  system's query shape, the test would pass vacuously. Today the guard is only
  cross-test (the midrange/combine tests assert nonzero through the same helper).
  Concrete change: after the 0.0 assert, teleport the SAME root inside the band
  (e.g. set its GlobalTransform to 100 u) and assert the target goes above zero -
  proving the exact entity the null assertion relies on is visible to the system.
  - Response: fixed - the test now teleports the same root to 100 u after the
    zero assert and asserts the target goes above zero.

- [x] R1.3 (NIT) crates/nova_gameplay/src/audio.rs:643-657 - pre-sink smoothing
  semantics changed silently: the old `update_thruster_loop_volume` early-returned
  when no `AudioSink` existed, so `smoothed` stayed frozen at 0 and the hum faded up
  from silence once the sink appeared; `compute_thruster_hum_volume` now advances
  `smoothed` unconditionally, so the first `apply_thruster_loop_volume` after the
  sink appears can snap straight to a caught-up value (a hot-engine scene at asset
  load would click on rather than fade in). Behaviorally negligible today and
  arguably more correct, but it is not the "keep the smoothing / existing behavior"
  parity the task records. Suggest one line in the `apply_thruster_loop_volume` doc
  (or NOTES.md) acknowledging the delta.
  - Response: acknowledged in both - apply_thruster_loop_volume doc explains
    the delta and why it is kept (nothing to fade from pre-sink; a correct
    level beats a late ramp), and NOTES.md lists it under behavior changes.

- [x] R1.4 (NIT) crates/nova_gameplay/src/audio.rs:28 and :594-596 - the docs claim
  "the camera rig sits past `SFX_NEAR_DISTANCE` by design" / "20-30 u out", but
  `mode_camera_rig` Turret mode is `(0, 5, -10)` = 11.2 u, inside NEAR
  (camera_controller.rs:524). The conclusion is unaffected (inside NEAR attenuation
  is 1.0 anyway; the exemption is really carried by the 250 u survey dolly), but the
  sentence overstates the rig. Suggest "the rig sits 11-32 u out and the survey
  dolly stretches it to 250 u" or similar.
  - Response: fixed - module doc now says "11-32 u out by mode and the orbit
    survey dolly stretches it to 250 u"; compute doc names Normal/FreeLook as
    the past-NEAR modes.

## Re-derived claims (all confirmed)

- Thruster sections are one-hop `ChildOf` children of the `SpaceshipRootMarker`
  root in the production assembly for BOTH player and AI ships:
  `insert_spaceship_sections` (crates/nova_scenario/src/objects/spaceship.rs:98-193)
  runs on `On<Add, SpaceshipRootMarker>` and spawns every section via
  `with_children` on the root. (The task's cited input/player.rs:186 is a consumer
  query, not the spawn site - the spawn site confirms the same shape.)
- Torpedo thrusters are `children![]` of the `TorpedoProjectileMarker` projectile
  root (crates/nova_gameplay/src/sections/torpedo_section/mod.rs:579-688), which is
  NOT a `SpaceshipRootMarker`, and the thruster carries its own Transform (so
  GlobalTransform) - the rootless self-attribution path is real.
- `PlayerSpaceshipMarker` sits on the ship root: inserted on the root entity at
  crates/nova_scenario/src/objects/spaceship.rs:198, and
  `#[require(SpaceshipRootMarker, ...)]` (input/player.rs:302) guarantees the root
  marker co-lives with it.
- Camera rig constants check out (camera_controller.rs): Normal |(0,5,-20)| = 20.6,
  FreeLook |(0,10,-30)| = 31.6, `BURN_PUSH_DISTANCE` = 3, `SURVEY_MAX_DISTANCE` =
  250 - at 250 u the rolloff would cut the player's own hum to ~5%, so the
  exemption is justified. (Turret mode is 11.2 u, inside NEAR - see R1.4.)
- System-split parity: pause/resume path untouched (sink-level pause on
  OnEnter/OnExit(Paused)); both new systems joined the same chained
  `SpaceshipSectionSystems` set the old one lived in, gated by
  `run_if(scenario_is_live)` (crates/nova_scenario/src/loader.rs:364,368); "no
  thrusters -> silent" preserved (empty per_source map -> target 0.0, smoothed
  decays); smoothing formula and alpha unchanged. Only delta found: pre-sink
  smoothing advancement (R1.3).
- Two-clocks convention: `compute_thruster_hum_volume` is a render-rate consumer
  running in Update and reading `GlobalTransform` for both listener and ship - the
  eased pose, which is exactly what docs/LESSONS.md `two-clocks` prescribes for
  render-rate consumers. Correct clock; both poses come from the same frame's
  propagation, so no clock mixing.

## Tests (would-it-fail audit)

- a_distant_ships_burn_does_not_raise_the_hum: fails under the recorded
  global-average sabotage (0.3 vs 0.0) - real. Null assertion, no in-test delivery
  guard (R1.2).
- a_midrange_ships_hum_is_scaled_by_distance_attenuation: composes the expectation
  from production helpers and self-guards (`expected > 0 && expected <
  engine_volume(0.8)`), so the rolloff must bite; fails under sabotage. Good.
- the_players_own_burn_is_never_attenuated: pins shared behavior, survives sabotage
  by design (old code also gave full volume) - correctly described in TASK.md.
- ships_combine_by_loudest_not_by_global_average: 0.3 vs 0.225 under the old
  average - fails under sabotage; exact f32 equality is safe here (same ops).
- a_rootless_thruster_attenuates_at_its_own_pose: the trailing near-thruster assert
  is an in-test delivery guard for the null half (though the near thruster omits
  `ChildOf`, so the guarded shape differs slightly from the far one); fails under
  sabotage. Good.
- the_hum_smooths_toward_its_target_instead_of_jumping: pins monotone approach and
  bounds, with a final `last > 0.0` delivery guard against a stuck alpha; sleeps
  only need dt > 0, so flake risk is low. Survives sabotage as recorded (pins
  shared behavior).
- TASK.md's sabotage A/B claims (4 fail / 2 survive) are consistent with this
  analysis.
- No existing test was weakened or deleted: the audio.rs tests diff is
  additions-only (hunk `@@ -763,6 +846,182 @@`); all 9 pre-existing audio tests are
  intact and green.

## Check suite

- `cargo fmt --check`: clean.
- `cargo check -p nova_gameplay --all-targets`: clean (pre-existing unrelated
  future-incompat warning from proc-macro-error2).
- `cargo test -p nova_gameplay audio::`: 15 passed, 0 failed.
- Full suite intentionally not run locally (repo convention: CI runs it); clippy
  intentionally not run.

## CHANGELOG / docs

- CHANGELOG.md Unreleased/Fixed entry matches the shipped behavior, including the
  player exemption and the menu-ambience side effect flagged in NOTES.md.
- Module doc (audio.rs:26-29) and `engine_volume` doc updated to per-ship
  semantics; only stale remnant is R1.1.
