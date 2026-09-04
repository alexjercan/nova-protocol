# Review and simplify autopilot arrival standoff

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog

## Goal

Start with a code review investigation of every arrival-standoff and target-radius path. Determine whether the current controls have overlapping responsibilities, then propose one clear model before changing behavior.

GOTO must choose a safe and useful final distance for beacons, asteroids, gravity wells, ships, and scripted positions. Large bodies must use their actual geometric extent. Small navigation targets must permit close, precise arrival.

## Current findings

- `FlightSettings::arrival_standoff` supplies the global default. Its current engine value corresponds to 500 m.
- `FlightArrivalStandoff` overrides the setting per moving ship.
- `SpaceshipConfig::arrival_standoff` authors that per-ship override.
- `MoveShipToActionConfig::arrival_standoff` temporarily overrides it per scripted order.
- Entity GOTO adds the target's resolved `BodyRadius`, including an asteroid radius derived from its actual collider geometry.
- Gravity-well GOTO resolves the well geometry and can transition into a dynamically selected stable orbit ring.
- `GotoPos` has no target geometry.
- There is no target-specific navigation-clearance contract.
- Arrival distance does not provide obstacle avoidance. A direct route can still cross another asteroid.

## Investigation

1. Trace all reads, writes, conversions, defaults, tests, documentation, and authored fields for:
   - `FlightSettings::arrival_standoff`.
   - `FlightArrivalStandoff`.
   - `SpaceshipConfig::arrival_standoff`.
   - `MoveShipToActionConfig::arrival_standoff`.
   - `BodyRadius` and gravity-well radius resolution.
   - GOTO, `GotoPos`, orbit parking, patrol, and shared ship orders.
2. Identify duplicated controls, inconsistent units, unclear precedence, and names that describe different concepts as "standoff."
3. Review actual mover geometry. Determine whether final clearance should include the moving ship's derived radius.
4. Review target categories: unsized point, beacon, ship, asteroid without gravity, gravity well, and authored anchor.
5. Separate final parking clearance from route obstacle avoidance.
6. Record compatibility effects for authored RON, mods, examples, AI orders, and scenario actions.

## Design questions

Evaluate a model similar to:

```text
final centre distance =
    resolved target geometric radius
    + resolved moving ship radius
    + navigation safety margin
```

The safety margin may have an automatic default and an explicit target or order override. Do not adopt this formula without proving how it composes with gravity-well orbit parking and existing ship/order overrides.

Decide and document:

- Whether the primary override belongs to the mover, target, order, or a small precedence chain.
- Whether target-specific clearance should be a general component/config or fields repeated on target object kinds.
- How explicit zero/close clearance is authored for navigation beacons.
- What `GotoPos` means when no target geometry exists.
- Whether unsafe authored values warn, clamp, or remain the creator's responsibility.

## Outcome of the investigation

Recorded in full in `INVESTIGATION.md`. Summary:

- Dials 3 and 4 (`AIControllerConfig::arrival_standoff`,
  `MoveShipToActionConfig::arrival_standoff`) are not independent controls.
  Both are writers of `FlightArrivalStandoff`, with different guards, different
  clearing rules and different lint coverage. That is the duplication.
- Five defects, each re-read in the source: the ORBIT park ignores the per-ship
  margin and can burn a ship outward to a ring it was never told to fly; a
  COMPLETED order never gives its override back, contradicting the shipped
  creator doc; `Some(0.0)` is honored on an order and silently dropped on a
  spawn; the global constant is a bare engine literal while its neighbours are
  typed; and the arrival model and the ORBIT band floor cross at a body radius
  the campaign's own planetoid straddles.
- The two target categories the goal names first, beacons and ships, are
  exactly the two that resolve no geometry at all today.

Recommended model, R1-R7 in `INVESTIGATION.md`:

```text
final centre distance = target geometric radius
                      + mover geometric radius
                      + navigation margin
```

with the margin resolved once and read at all three sites, the order override
retired on completion as well as cancellation, `None`/`Some(x)` meaning the
same thing in both authored paths, and a well-bearing target's arrival floored
at the ORBIT band's own floor so the two safe-distance models become one. No
new dial is added; the target contributes size, never a margin.

## Expected output

Keep the investigation, evidence, alternatives, recommendation, and migration impact in this task before implementation. Prefer removing redundant controls over adding another independent knob.

If implementation is approved, add focused behavior tests for every target category and update creator/autopilot documentation. Do not fold pathfinding or obstacle avoidance into this task.

