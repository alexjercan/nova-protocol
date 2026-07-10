# Review: Diegetic flight instruments - maneuver instruments v1

- TASK: 20260709-103454
- BRANCH: flight-instruments

## Round 1

- VERDICT: REQUEST_CHANGES

Reviewed commit f948566 against master with an independent adversarial
pass (spec: TASK.md + the instruments spike). Structure is sound: clearing
paths traced complete (in-system Has branch covers verb switch, observer
covers disengage incl. all early-continue paths), observer safety, asset
refcounting, Torus::new semantics, from_rotation_arc antiparallel case,
registration and harness wiring all verified. One correctness defect.

- [x] R1.1 (MAJOR) flight.rs (goto_flip_point / arrival_eta) - the flip
  estimate omits the rotation lead. The autopilot's brake onset solves
  `v*lead + v^2/(2 a margin) = remaining` (arrival_speed_limit; lead =
  flip rotation time + spool pad), but the telemetry uses only
  `standoff + v^2/(2a)`, so the FLIP marker sits v*lead short of where
  the ship actually flips - at cruise speeds with a main-drive-only ship,
  ~seconds of coast (~100+u) of visible error, defeating the instrument
  and contradicting the "cannot drift from what the computer flies"
  claim. seconds_to_flip and eta inherit it. Fix: pass the in-scope
  `lead` through and use `standoff + v*lead + v^2/(2a)`; update the pure
  tests.
  - Response: fixed - goto_flip_point/arrival_eta take the lead and mirror arrival_speed_limit exactly (standoff + v*lead + v^2/(2a)); call site passes the in-scope lead; pure tests updated with lead-aware numbers and the physics test samples while the (earlier) flip is still ahead.

- [x] R1.2 (MINOR) flight.rs (ManeuverTelemetry::flip_point) - under
  heavy lateral drift the estimate is optimistic (brake group chosen on
  full speed, flip computed on along-track speed; the controller kills
  lateral first). Right inputs for v1; add a doc-comment caveat.
  - Response: fixed via documentation - flip_point doc notes the optimistic bias under lateral drift and why.

- [x] R1.3 (MINOR) maneuver_instruments.rs (CHIP_SIZE 160x16) - the
  readout line is ~28 chars at ~9px/char (~250px); Bevy UI Text wraps in
  a fixed 160px node, so the line likely renders wrapped in a 16px-tall
  chip. Widen the readout chip (and/or LineBreak::NoWrap).
  - Response: fixed - readout chip is 260x16 with LineBreak::NoWrap; the short chips stay 120px.

- [x] R1.4 (MINOR) maneuver_instruments.rs (drive_destination_readout) -
  a moving GOTO target separates the marker (Entity anchor, interpolated
  GlobalTransform) from the readout (Point anchor at last-FixedUpdate
  Position): the caption slides off its marker. Carry the target entity
  in the telemetry for GOTO and anchor Entity (Point only for GotoPos).
  - Response: fixed - ManeuverTelemetry carries goal_entity (Some for GOTO); the readout anchors the entity when present so the caption rides the same interpolated transform as the marker, Point only for GotoPos.

- [x] R1.5 (MINOR) flight.rs (tests) - verb-switch clearing (GOTO ->
  STOP, the insert-overwrite path where OnRemove does NOT fire and the
  in-system Has branch carries it) is the path a player hits most and is
  untested; extend the telemetry test.
  - Response: fixed - the telemetry test now covers the GOTO -> STOP insert-overwrite path (asserts cleared) before the disengage path.

- [x] R1.6 (MINOR) maneuver_instruments.rs (sync_orbit_ring) - the
  replan-rebuild branch (marker.radius mismatch) is advertised in the doc
  but untested; extend the ring test with a changed plan radius.
  - Response: fixed - the ring test replans to radius 80 and asserts the old ring is despawned and the rebuilt one carries the new radius.

- [x] R1.7 (NIT) maneuver_instruments.rs - `emissive` on an `unlit`
  StandardMaterial is dead (unlit outputs base_color only); drop one.
  - Response: fixed - emissive dropped; unlit base_color only.

- [x] R1.8 (NIT) maneuver_instruments.rs (sync_orbit_ring) - the ring
  Transform is written through Mut every frame even when unchanged,
  dirtying change detection; guard with a != check.
  - Response: fixed - the transform write is guarded by a != check.

- [x] R1.9 (NIT) - NAV cyan duplicated across maneuver_instruments.rs and
  flight_status.rs (marker tint, cue color); hoist a shared const. The
  "m" unit label matches the task spec's example and stays.
  - Response: fixed - NAV_CYAN hoisted to hud/mod.rs and used by the marker tint, the cue, and all chips; the "m" label stays per the task spec example.

Checked and found sound: Torus::new(inner, outer) -> major = plan radius
(verified against bevy_math source); from_rotation_arc antiparallel
fallback; observer removes a component only (no despawn-in-observer
hazard); insert-then-remove flush ordering; try_insert replace-in-place
after first insert; asset handles refcounted (no leak on ring rebuild);
find_map stability with one ring per player; orbit_ring_point usage; ETA
continuity at the flip boundary; format strings vs tests; registration,
harness observer, hud/mod.rs despawn coverage for both the layer and the
ring.

## Round 2

- VERDICT: APPROVE

Verified every response against the new diff: the flip math now solves the
same equation as arrival_speed_limit (lead included) and the updated pure
tests pin the numbers; the readout chip is wide and non-wrapping; GOTO legs
anchor the tracked entity; the verb-switch and replan-rebuild paths are
tested; material, transform guard, and NAV_CYAN hoist are in. All affected
modules green (48 flight, 47 hud, AI patrol), fmt + check --workspace
--examples clean. No new findings; ready to land.
