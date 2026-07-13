# Gravity-aware arrival planning (GOTO/STOP)

Task: tasks/20260710-193500 - Spike: docs/spikes/20260710-204802-gravity-aware-arrival.md

## What changed

The arrival solver in crates/nova_gameplay/src/flight.rs now budgets the
well pull (summed positive along-track component over all wells) on every
GOTO/GotoPos leg and in STOP's rest-point telemetry. Before, the plan assumed the brake group's thrust was the only
acceleration in play, so a GOTO at a well body flipped on the vacuum curve
and the well fed speed through the whole descent - the integration test
reproduces the playtest crash: with the budget disabled the ship dips to
12u from the center of a 40u body; with it, it parks at the 50u standoff.

## The formula

All four pure helpers gained a `gravity_along` parameter (`g`, the pull
component toward the goal in u/s^2, clamped >= 0):

- `arrival_speed_limit` solves
  `v*lead + g*lead^2/2 + (v + g*lead)^2 / (2*(a*margin - g)) = d`
  (substituting `u = v + g*lead` keeps it closed-form). Gravity adds
  `g*lead` of speed and `g*lead^2/2` of drift during the un-braked flip
  window and reduces the brake to `a*margin - g` afterwards.
- `goto_flip_point`, `arrival_eta`, `stop_rest_distance` mirror the same
  terms, so the FLIP marker, ETA and STOP rest point self-correct through
  the existing ManeuverTelemetry seam - no HUD changes.
  `ManeuverTelemetry.brake_accel` also carries the effective value, but no
  HUD instrument reads that field yet; it is forward-correct for whichever
  decel readout adopts it.
- `g = 0` reduces every formula to the previous behavior exactly; all
  pre-existing tests pass untouched apart from the added argument.

`g` is evaluated at the leg's REST POINT (the standoff point), not the
ship: on a monotonic inward leg that is the strongest pull the leg will
see, so the plan is conservative from the first tick and per-tick
replanning only ever tightens it (spike option d). The system scans every
`GravityWell` rather than the ship's `DominantWell` because the flip is
usually planned from OUTSIDE the SOI, where the ship has no DominantWell
yet but the goal is already deep in one. STOP evaluates at the ship
instead (its rest point is what is being solved for); documented asymmetry.

## Degradation

If the pull meets or exceeds the brake authority no stopping plan exists:
the desired velocity is zero WITHOUT the min_approach floor (the floor
would nurse the ship inward on a leg it can never stop), telemetry
publishes `brake_accel = 0` with `flip_point`/`eta = None` (instruments go
blank - the honest signal; `arrival_eta` refuses its braking-regime
fallback in this state), and the autopilot stays engaged braking flat out
rather than silently disengaging. The degradation is logged at debug level
once per leg entry (gated on the previously published plan still having
brake authority), not per tick.

ORBIT's ring correction keeps `g = 0` deliberately: its tangential v_circ
term already balances the well on the ring, and the correction is a
bounded nudge the hold loop re-issues every tick.

## Alternatives considered

Recorded in the spike: current-position feedforward (optimistic exactly
when it matters - the crash mechanism, softened), integrating the pull
over the brake arc (exact but three-branch potential math that per-tick
replanning makes redundant), and a bare minimum-standoff clamp (fixes
geometry, not dynamics; that half is queued as task 20260710-202408).

## Known limits

- Grazing legs (goal outside the well, path passing near it) have their
  worst pull mid-path, not at the rest point; under-budgeted, but the
  failure is a sloppy stop, not a crash. Revisit if playtests show it.
- The flip coast estimate uses the current closing speed; gravity's
  coast-phase gain shows up as the flip marker creeping outward as the
  ship accelerates - self-correcting, and now conservative rather than
  optimistic.
