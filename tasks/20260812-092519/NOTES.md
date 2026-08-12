# NOTES

## Result

Replaced recurring `OnOrbit` with one-shot orbit lifecycle edges:
`OnOrbitStart`, `OnOrbitStable`, `OnOrbitUnstable`, and `OnOrbitEnd`.

## Semantics

- Start means the ORBIT maneuver engaged, not that insertion is complete.
- Stable and unstable follow transitions into and out of `AutopilotPhase::Hold`.
  Tracking runs after flight in `FixedUpdate`, so multiple physics ticks in one
  render frame cannot hide a brief loss of stability.
- Stability can be lost and recovered more than once in one maneuver.
- Ending a stable orbit emits only end, not unstable then end.
- Switching wells emits end for the old well, then start for the new well.
- Ship destruction emits only `OnDestroyed`. Surviving cancellation,
  capability loss, verb change, and well loss emit `OnOrbitEnd`.
- All lifecycle events carry the well as `id` and ship as `other_id`.

## Migration

Removed `orbit_hold_secs`. Shakedown now starts a five-second scenario timer on
stable, cancels it on unstable or end, and completes the lesson on timer end.

## Coverage

- Scenario unit suite covers every lifecycle edge, switch ordering, well loss,
  and destruction exclusion.
- Shakedown walk covers stable -> timer start, unstable -> cancel, recovery ->
  fresh timer, and timer-end progression.
- `screenshot_flight` reaches physical `AutopilotPhase::Hold` and waits for the
  real `OnOrbitStable` handler result. Probe run completed at frame 691 with
  6/7 measured checks passing and FPS N/A because the example claims none.

## Documentation follow-up

An event relationship graph would make mutually exclusive and ordered edges
clear. Add it after the event vocabulary work settles, so it describes the
final orbit, lock, area, destruction, and timer model once.
