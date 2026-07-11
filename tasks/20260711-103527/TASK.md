# Thruster impulses push from the stale eased pose (GlobalTransform in FixedUpdate)

- STATUS: CLOSED
- PRIORITY: 95
- TAGS: v0.5.0,bug,physics,flight

## Goal

`thruster_impulse_system` (FixedUpdate) applies impulses at the thruster's
`GlobalTransform.translation()`, which during FixedUpdate is the PREVIOUS
frame's eased render pose - up to ~2 ticks of ship motion behind raw
physics since the 2026-07-09 interpolation change. The balancer computes
lever arms from the raw pose (flight.rs) and its comment claims both use
the same point; they no longer do. The mismatch grows linearly with
velocity, so hard decel from high speed produces uncompensated torque:
the ship twitches and sometimes flips (umbrella 20260711-094915 symptom).
Root-cause analysis: docs/spikes/20260711-103527-twitching-family-two-clocks.md.

## Steps

- [x] Diagnostic first (residual-roll retro lesson): an `#[ignore]`d tick
      trace test that flew a ship at 150 u/s with a lateral engine mounted
      exactly on the COM and logged, per tick, the impulse application
      point (`GlobalTransform.translation()`) vs the raw point
      (`position.0 + rotation.mul_vec3(local_offset)`). Evidence recorded
      below; the diag was deleted in this branch once the regression
      landed (retro convention).
- [x] Verify avian 0.7 maintains raw-synced `Position`/`Rotation` on child
      collider entities. ANSWER: yes, but one tick stale -
      `update_child_collider_position` runs at `PhysicsStepSystems::First`
      (before integration), composing the BODY's raw pose with
      `ColliderTransform`. So neither the child's avian pose nor its
      GlobalTransform is current-tick; the fix composes from the root's
      own raw pose instead.
- [x] Fix `thruster_impulse_system`
      (crates/nova_gameplay/src/sections/thruster_section.rs): application
      point and thrust direction now composed from the root's raw
      `Position`/`Rotation` (via avian's `Forces` item accessors) and the
      thruster's local mount Transform - the exact math of the balancer's
      lever arms. Balancer comment in flight.rs updated to state the
      shared invariant.
- [x] Audit ALL FixedUpdate-schedule readers of `Transform`/`GlobalTransform`
      across nova crates. Verdict table below; one further fix landed
      (autopilot GOTO target pose), everything else was already clean.
- [x] Regression test `high_speed_lateral_burn_through_the_com_adds_no_spin`
      (flight.rs tests): production-faithful rig (TransformInterpolation on
      the hull), proven to FAIL against the unfixed code (7.1 rad/s spin in
      15 frames from a zero-true-torque engine), passes after the fix
      (spin 0). `high_speed_stop_settles_without_tumbling` stays green
      with its tightened 0.5 rad/s bound.
- [x] cargo check + fmt clean; test modules flight:: (56), sections:: (51),
      input:: (117) all pass locally; full suite runs in CI. Fix recorded
      in the spike doc's new "Fix record" section.

## FixedUpdate pose-source audit (2026-07-11)

| System | Pose reads | Verdict |
| --- | --- | --- |
| thruster_impulse_system | GlobalTransform point + child avian Rotation | FIXED: root raw pose + local mount |
| autopilot_system | q_target GlobalTransform (GOTO goal) | FIXED: prefer avian Position, GlobalTransform fallback for static markers |
| autopilot_system | q_wells avian Position | clean (already raw) |
| manual_burn_system | thruster local Transform + body COM | clean (local statics) |
| sync_controller_section_forces | none (PDControllerOutput -> Forces) | clean |
| gravity_well_system | avian Position everywhere | clean |
| bcs track_command / PD sync | avian Rotation + AngularVelocity | clean (raw) |

AI `on_thruster_input` / `on_projectile_input` read GlobalTransform but run
in Update (render clock) - deliberate "aim at what you see" reads, out of
scope here; the HUD tasks 20260710-231928/231929 own the render-side story.

## Diagnostic evidence (exact rig)

Rig: flight_app() harness (60 fps manual time, 64 Hz fixed physics), hull
of three unit-cube sections at z = -1, 0, +1 (COM at origin),
TransformInterpolation on the root (production-faithful), lateral engine
at the origin thrusting +X (thrust line through the COM, true torque
exactly zero), no FlightIntent, no controller. LinearVelocity = 150 u/s
along +Z, throttle 1.0, probe in FixedUpdate after SpaceshipSectionSystems.

- Unfixed, WITHOUT TransformInterpolation: application-point error is 0 on
  single-tick frames and 2.345 u on double-tick frames (64 vs 60 Hz beat);
  ONE stale tick kicked 0.94 rad/s of spin into the hull.
- Unfixed, WITH TransformInterpolation (production): error nonzero almost
  every tick, sawtoothing 1.72 -> 0.16 u with the easing phase, 2.35 u
  spikes on double-tick frames; spin after 15 frames: 7.11 rad/s.
- Fixed: GlobalTransform still trails (nothing consumes it anymore); spin
  after 15 frames: 0 rad/s exactly, and the regression asserts < 0.05
  after 60 frames.

## Resolution

What changed:

- thruster_section.rs: impulse application point and thrust direction are
  composed from the root's raw `Position`/`Rotation` (avian `Forces` item
  accessors, no extra query terms) and the engine's local mount Transform.
  The old code mixed clocks even internally: direction from the child's
  raw-but-stale avian Rotation, point from the eased GlobalTransform.
- flight.rs: GOTO goal pose prefers the target's raw avian `Position`
  (falls back to GlobalTransform for static, body-less markers); balancer
  comment now states the by-construction invariant with the impulse
  system.

Alternatives considered:

- Using the thruster's own avian `Position`/`Rotation`: still one tick
  stale (avian updates child collider poses before integration, from the
  PREVIOUS tick's body pose) - rejected for the same reason as
  GlobalTransform, just with a smaller, deterministic error.
- Forcing GlobalTransform propagation inside the fixed loop: pays hierarchy
  propagation per tick for every body to fix one consumer, and still
  leaves the render clock eased - rejected.

Difficulties / findings along the way:

- The diagnostic overturned part of the spike's mental model: pure
  prograde/retrograde burns are torque-benign (the stale offset is
  parallel to the thrust line); the damage comes from LATERAL thrust at
  speed (balancer recruits, drift correction) where the stale offset is
  perpendicular to the thrust - and from rotation staleness mid-flip.
  This matches the playtest wording precisely ("cannot hold its
  deceleration path... twitches and flips"): the deceleration path is
  exactly when recruits and drift corrections fire.
- Without TransformInterpolation on the test hull the bug only fires on
  double-tick frames; the first diag run looked "mostly clean" until the
  rig was made production-faithful. Test rigs for clock bugs must carry
  the same interpolation opt-ins as production entities.

Self-reflection: the spike predicted the mechanism but overweighted the
main-drive case; walking the cross-product geometry BEFORE writing the
spike doc would have priced the lateral-thruster pathway in from the
start. The diagnostic-first discipline caught it cheaply.
