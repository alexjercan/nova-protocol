# GOTO arrival planning is gravity-blind: ships crash into well bodies

- STATUS: OPEN
- PRIORITY: 60
- TAGS: v0.5.0, physics, autopilot, gravity, bug, spike

## Goal

Playtest finding (user, 2026-07-10): the flight computer does not account
for the gravity well when planning the mid-turn/flip point, so a GOTO
toward (or past) a well body brakes too late - the well keeps accelerating
the ship through the plan and it crashes into the object.

Why it happens: the arrival rule `v_allowed(d) = sqrt(2 a margin d)` and
the flip point (`v*lead + v^2/(2a)`) assume the only acceleration in play
is the brake group's. Inside an SOI the well adds up to
`GravitySettings::max_surface_gravity` toward the body, which both eats
into the effective braking acceleration (braking outward against the pull)
and keeps adding speed during the un-braked lead window. The gravity spike
(docs/spikes/20260709-193147, decision 6 and Open questions) accepted this
for v1 on the strength-guardrail argument and recorded "gravity
feedforward in STOP/GOTO" as the follow-up - the playtest shows the error
is not always small near big bodies.

This is likely spike-worthy before implementation (hard-ish): candidate
directions to weigh there include (a) subtracting the dominant well's
current pull component from the brake acceleration in the arrival solve
(cheap feedforward, per-tick replanning absorbs the rest), (b) integrating
the pull over the predicted brake arc (more honest, more math), and (c) a
minimum-standoff clamp against well bodies (crash guard independent of the
solver). The ManeuverTelemetry seam means any fix automatically corrects
the FLIP marker and ETA too.

## Steps

- [ ] Thread a `gravity_along: f32` parameter through the pure arrival
  helpers in crates/nova_gameplay/src/flight.rs: `arrival_speed_limit`
  solves the modified quadratic `v*lead + g*lead^2/2 + (v + g*lead)^2 /
  (2*(a*margin - g)) = d` (closed form; returns 0 when `a*margin <= g`),
  and `goto_desired_velocity`, `goto_flip_point`, `arrival_eta`,
  `stop_rest_distance` gain the matching lead-window and effective-brake
  terms. `g = 0` must reduce every formula to the current behavior.
- [ ] Unit tests for the helpers: g=0 equals the old closed forms; g>0
  lowers the allowed speed and pushes the flip point out; `a*margin <= g`
  yields no plan (speed 0 / None); the flip identity (braking from
  v_allowed at a_eff over the remaining distance lands at the standoff)
  holds with gravity.
- [ ] In `autopilot_system`, compute `g_along` once per arrival leg:
  add `Option<&DominantWell>` to the ship query, evaluate `well_accel` at
  the GOAL position (spike: worst-point bound), project on the closing
  direction, clamp at >= 0. Feed it through `arrival_desired` (GOTO and
  GotoPos) and the STOP telemetry branch.
- [ ] Report the effective (gravity-reduced) `brake_accel` in
  ManeuverTelemetry so the FLIP marker, decel chip and ETA self-correct;
  on `a_eff <= 0` publish telemetry with `flip_point`/`eta` = None and
  `debug!` once instead of disengaging.
- [ ] Integration test (app-level, like the existing autopilot tests):
  GOTO a well body from outside the SOI at speed; assert the ship never
  penetrates `body_radius` and comes to rest within a tolerance of the
  standoff. A control run in flat space must keep today's arrival frames.
- [ ] Run the input::ai test module (autopilot_system signature changes -
  standing lesson) plus flight, hud and `cargo check --workspace
  --examples`.
- [ ] Docs: docs/2026-07-10-gravity-aware-arrival.md (what changed, the
  formula, the degradation path); close TASK.md with the resolution.

## Notes

- Spike first when picked up (/spike, then /plan): the interaction between
  the lead window, the fade band, and per-tick replanning deserves a real
  look before choosing a formula.
- Relevant code: crates/nova_gameplay/src/flight.rs (arrival_speed_limit,
  goto_desired_velocity, goto_flip_point, braking_plan closure in
  autopilot_system), crates/nova_gameplay/src/gravity.rs (well_accel,
  DominantWell).
- Related recorded deferrals: gravity spike Open questions ("Gravity
  feedforward in STOP/GOTO"); STOP inside a well completes and hands back
  control while falling (intentional, unchanged by this task).
- Repro sketch: lock the Gravity Rock (asteroid_field), GOTO it from far
  outside the SOI at speed; the ship brakes on the gravity-free plan and
  impacts the surface instead of stopping at the standoff.
