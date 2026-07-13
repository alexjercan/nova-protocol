# Audio SFX: thruster hum audible from far away (attenuation bug)

- STATUS: CLOSED
- PRIORITY: 90
- TAGS: v0.5.2,bug,audio,feedback

## Goal

Investigate and fix SFX distance attenuation for the thruster hum: a distant
ship burning must not be audible at full strength from the player's seat.

Reported 2026-07-11 (user playtest): the thruster hum of other ships is
audible from far away, which is wrong. The turret shoot sound attenuates
correctly at the same distances.

## Mechanism (verified in source, 2026-07-13 plan pass)

There is no attenuation path for the hum at all, and it is not per-ship:
`update_thruster_loop_volume` (crates/nova_gameplay/src/audio.rs:546) drives
ONE global, unpositioned loop entity (spawned by `ensure_thruster_loop`,
audio.rs:520) from the throttle averaged over EVERY active thruster in the
world; the doc comment (audio.rs:541-544) explicitly defers per-ship
attribution "for when there is more than one audible ship" - which v0.5.0's
AI ships made true. A distant AI ship's burn therefore raises the player-side
hum with no distance term. One-shots attenuate because `play_positional`
(audio.rs:224) multiplies the base volume by
`distance_attenuation(listener->source)` (audio.rs:208; near 20, far 320,
geometric rolloff), reading the `SfxListenerMarker` camera.

## Steps

- [x] Trace the reported scenario first (diagnostic-first): traced in the
      SHIPPED app (main menu ambience scene, which is exactly a distant AI
      ship burning: 4 min under Xvfb with a temp trace, pre-fix and
      post-fix). Pre-fix: hum at 0.26/0.3 while the only burning thruster
      was 341 u away (one-shot attenuation there: 0.0). Post-fix: 0.0
      beyond FAR, 0.0033 at 258 u. Full rig + numbers in NOTES.md.
- [x] Fail-first regression test: six App-driven tests over the production
      system + markers (audio.rs tests, `hum_app` rig). Proven against the
      pre-fix behavior by sabotage AFTER committing the fix
      (commit-before-sabotage): reintroducing the global-average/no-
      attenuation computation fails 4 of them -
      a_distant_ships_burn_does_not_raise_the_hum (got 0.3, want 0.0 for a
      ship 500 u away), a_midrange_ships_hum_is_scaled_by_distance_attenuation,
      ships_combine_by_loudest_not_by_global_average,
      a_rootless_thruster_attenuates_at_its_own_pose - while the two that
      pin pre-fix-shared behavior (player exemption, smoothing) survive,
      as expected. Restored via git checkout; 15/15 green after.
- [x] Fix: `update_thruster_loop_volume` split into
      `compute_thruster_hum_volume` (all logic, writes a new
      `ThrusterHumVolume` resource; App-testable headless since an
      `AudioSink` cannot be constructed without an audio device) and
      `apply_thruster_loop_volume` (copies onto the sink). Grouping walks
      `ChildOf` ancestors to the `SpaceshipRootMarker` (verified: ship
      thrusters are one-hop children, input/player.rs:186; torpedo
      thrusters hang off the projectile root instead,
      torpedo_section/projectile.rs:260, and attribute to themselves ->
      attenuate at their own pose). Per-source
      `engine_volume(avg) * distance_attenuation`, loudest source wins,
      reusing `listener_position()`/`SfxListenerMarker`.
- [x] Player's own ship: answered statically from the camera rig constants
      instead of a runtime measurement (camera_controller.rs is the single
      source of truth for the rig): mode offsets are 20.6-31.6 u, already
      past SFX_NEAR_DISTANCE = 20, and the orbit survey dolly stretches the
      rig to SURVEY_MAX_DISTANCE = 250 u - so the player-controlled ship IS
      exempt from attenuation (PlayerSpaceshipMarker sits on the root,
      verified at input/player.rs:303 + require attribute). Pinned by test
      `the_players_own_burn_is_never_attenuated` at the 250 u worst case.
- [x] Keep the exponential smoothing (moved from the Local into the new
      ThrusterHumVolume resource, pinned by
      `the_hum_smooths_toward_its_target_instead_of_jumping`), the "no
      thrusters -> silent" behavior, and the existing audio tests green
      (15/15 pass).
- [x] CHANGELOG.md entry under Unreleased (Fixed).

## Notes

- Comparison anchor: turret shoot = correct (positional one-shot via
  play_positional), thruster loop = wrong (global loop, no distance term).
- Introduced by task 20260708-162011; the deferral was deliberate and is
  documented in the code comment this task removes.
- Existing tests cover the pure math only (engine_volume_...,
  distance_attenuation_... in audio.rs tests); the new regression test is
  the first App-level one for the loop volume system.

## Record (2026-07-13)

What changed: `update_thruster_loop_volume` replaced by
`compute_thruster_hum_volume` (per-ship attribution: ChildOf walk to the
SpaceshipRootMarker ancestor or the thruster itself when rootless; per-source
`engine_volume(avg) * distance_attenuation`; loudest source wins; player ship
exempt; writes new `ThrusterHumVolume` resource) +
`apply_thruster_loop_volume` (sink write). Six new App tests; changelog
entry. Full mechanism, trace rig, A/B numbers, alternatives and behavior
changes (menu ambience now fades with orbit distance; torpedo thrusters
attenuate) in NOTES.md.

Difficulties: no xvfb-run on this host (raw Xvfb only); the harnessed
examples cannot exhibit the bug inside their 6 s autopilot windows (menu
example clicks away before the ambience ship burns; torpedo range never
fires in time) - the reliable trace rig is the plain app sitting on the main
menu. A cold-worktree build also raced the first trace run's source edits;
resolved with a file-copy A/B (details in NOTES.md).

Self-reflection: should have checked the trace vehicle's timeline (when does
anything BURN in this example?) before paying for full example runs - two
runs were spent learning that; reading the autopilot scripts first would
have picked the plain-menu rig immediately. The compute/apply split for
sink-coupled systems is worth reusing anywhere audio volume logic needs
tests.
