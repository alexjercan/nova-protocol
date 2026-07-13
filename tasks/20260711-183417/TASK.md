# Audio SFX: thruster hum audible from far away (attenuation bug)

- STATUS: OPEN
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

- [ ] Trace the reported scenario first (diagnostic-first): a rig with the
      player idle and an AI ship thrusting beyond SFX_FAR_DISTANCE; log
      avg_throttle and the sink volume to confirm the distant ship raises
      the hum. Record the numbers in NOTES.md.
- [ ] Fail-first regression test: same rig as an App-driven test - the hum
      target volume must stay ~0 when only the far ship thrusts, and rise
      when the player's ship thrusts. Confirm it fails against the current
      code and record the failing numbers.
- [ ] Fix in `update_thruster_loop_volume`: group active thrusters by ship
      root (verify the actual ChildOf chain from `ThrusterSectionMarker` to
      the ship root before coding against it), compute per-ship
      `engine_volume(avg ship throttle) * distance_attenuation(listener ->
      ship root)`, and take the max contribution as the loop target
      (summing would pin the hum to max, per the existing comment). Reuse
      `listener_position()`/`SfxListenerMarker`, not a new listener path.
- [ ] Player's own ship: measure the SfxListenerMarker camera's distance to
      the player ship root in a normal run; if it can exceed
      SFX_NEAR_DISTANCE, exempt the player-controlled ship from attenuation
      (its hum is cockpit-diegetic), otherwise no special case is needed.
      Verify first what marks the player ship before relying on it, and
      record the measured distance either way.
- [ ] Keep the exponential smoothing (audio.rs:570-574), the "no thrusters
      -> silent" behavior, and the existing audio tests green.
- [ ] CHANGELOG.md entry under Unreleased (Fixed).

## Notes

- Comparison anchor: turret shoot = correct (positional one-shot via
  play_positional), thruster loop = wrong (global loop, no distance term).
- Introduced by task 20260708-162011; the deferral was deliberate and is
  documented in the code comment this task removes.
- Existing tests cover the pure math only (engine_volume_...,
  distance_attenuation_... in audio.rs tests); the new regression test is
  the first App-level one for the loop volume system.
