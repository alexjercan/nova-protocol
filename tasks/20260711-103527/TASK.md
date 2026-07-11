# Thruster impulses push from the stale eased pose (GlobalTransform in FixedUpdate)

- STATUS: OPEN
- PRIORITY: 95
- TAGS: v0.5.0,bug,physics,flight

## Goal

`thruster_impulse_system` (FixedUpdate) applies impulses at the thruster's
`GlobalTransform.translation()`, which during FixedUpdate is the PREVIOUS
frame's eased render pose - up to ~2 ticks of ship motion behind raw
physics since the 2026-07-09 interpolation change. The balancer computes
lever arms from the raw pose (flight.rs:1499) and its comment claims both
use the same point; they no longer do. The mismatch grows linearly with
velocity, so hard decel from high speed produces uncompensated torque:
the ship twitches and sometimes flips (umbrella 20260711-094915 symptom).
Root-cause analysis: docs/spikes/20260711-103527-twitching-family-two-clocks.md.

## Steps

- [ ] Diagnostic first (residual-roll retro lesson): an `#[ignore]`d tick
      trace test in the flight tests that flies a ship at high speed under
      full burn and logs, per tick, the impulse application point
      (`GlobalTransform.translation()`) vs the raw point
      (`position.0 + rotation.mul_vec3(local_offset)`); confirm divergence
      scales with V. Record the exact rig in this file.
- [ ] Verify avian 0.7 maintains raw-synced `Position`/`Rotation` on child
      collider entities (the thruster query already reads avian `Rotation`);
      answer explicitly here - it decides the fix shape.
- [ ] Fix `thruster_impulse_system`
      (crates/nova_gameplay/src/sections/thruster_section.rs:131-159) to
      derive the application point from raw physics state: the thruster's
      own avian `Position` if the verification holds, else root raw
      `Position`/`Rotation` composed with the thruster's local offset (the
      exact math of flight.rs:1499), so balancer and impulse agree by
      construction. Fix the flight.rs:1496-1499 comment to state the shared
      invariant.
- [ ] Audit ALL FixedUpdate-schedule readers of `Transform`/`GlobalTransform`
      across nova crates (grep `add_systems(FixedUpdate` and check each
      system's queries; also FixedPre/PostUpdate): list them here with a
      raw-or-eased verdict each, and fix the ones that feed physics
      (candidates: torpedo guidance/fuze, gravity, AI).
- [ ] Replace the diagnostic with a tight regression test: at high V under
      burn, the applied point equals the raw-derived point within epsilon;
      plus keep `high_speed_stop_settles_without_tumbling` green (spin
      bound stays at the tightened 0.5 rad/s).
- [ ] cargo check + fmt + run the newly written tests (suite runs in CI);
      record the fix in docs/ (extend the spike doc with a fix record
      section or a dated doc).

## Notes

- Evidence: thruster_section.rs:134 (query reads `&GlobalTransform`),
  :153 (direction from raw avian `Rotation` - the system already mixes
  clocks internally), :155 (application point from GlobalTransform);
  flight.rs:1473-1500 (raw-pose lever arms + stale comment).
- Magnitude: error ~ V * (0..2 ticks); at 100 u/s up to ~3 u misplacement
  per main-drive impulse.
- Depends on: nothing. Do this task first; 20260710-231931 re-tests the
  ship-twitch symptom against it.
