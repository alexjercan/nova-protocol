# World-space holo instruments: trajectory ribbon, SOI shell, flip gate

- TASK: 20260710-174629
- SPIKE: tasks/20260710-174523/SPIKE.md
- MODULE: crates/nova_gameplay/src/hud/holo_instruments.rs (+ STOP
  telemetry in flight.rs)

## What was built

The expansion of the world-space holo language the ORBIT ring piloted -
all thin unlit NAV_CYAN geometry, all lifecycles headless-testable:

- **STOP telemetry** (flight.rs): STOP legs now publish ManeuverTelemetry
  toward their predicted rest point (`v*lead + v^2/(2 a margin)` along the
  velocity - the same terms as the GOTO flip, via `stop_rest_distance`).
  A settled STOP clears it before disengage. The destination readout chip
  covers braking legs for free.
- **Trajectory ribbon**: the engaged leg's path as thin cylinder segments
  (ship -> flip -> goal, collapsing to ship -> goal once braking), GOTO
  and STOP alike. Deliberately the straight-line plan the computer
  actually flies; a curved prediction waits for the gravity-aware arrival
  solve (task 20260710-193500) per the AGENTS.md
  derive-from-the-owning-system rule.
- **Flip gate**: a 4u fly-through ring at the flip point, faced down the
  path; retires when braking starts.
- **SOI shell**: three axis-aligned great-circle rings at soi_radius (a
  wire globe without wireframe tech) around the dominant well while
  inside, or the nearest well within 1.5x its SOI on approach; nothing in
  flat space. This delivers the gravity spike's deferred "SOI rings in
  normal play".

## Decisions

- One shared unit cylinder mesh, one cached gate mesh, and one shared
  material for the whole holo family including the orbit ring (HoloAssets
  Resource, lazily filled); segments are transformed
  (midpoint/rotation/length-scale), not re-meshed.
- Gate/shell tori are built per spawn at their radius, like the orbit
  ring, so tube thickness stays constant.
- The shell approach factor is a module const (display concern), promoted
  to settings only if playtests want the knob.

## Verification

- Review round 1 (all MINOR) hardened the STOP publish gate with
  hysteresis (no strobing at the epsilon threshold) and made shell-ring
  identity data instead of Quat equality.
- 2 new flight tests (pure rest-distance math incl. no-authority None;
  physics-level STOP publishes its rest point and settling clears it) +
  the verb-switch arm of the existing telemetry test updated (GOTO -> STOP
  now republishes instead of clearing).
- 3 holo lifecycle tests: ribbon segment counts across flip/braking/leg
  end; gate position/orientation and braking retirement; shell
  approach/inside/flat-space states with no duplicate rings.
- Affected modules green: flight (50), hud (53), input (111). fmt + check
  --workspace --examples clean. Full suite and clippy on CI.

## Difficulties

None material; the sync-system pattern from the orbit ring absorbed all
three elements.

## Self-reflection

- Extending telemetry to STOP was the one real design call: the
  instruments task had recorded "STOP has no spatial goal" and this task
  overturned it (the rest point IS a spatial goal). Overturning a
  documented v1 exclusion explicitly - with the reasoning in the task file
  - kept it a decision rather than drift.
