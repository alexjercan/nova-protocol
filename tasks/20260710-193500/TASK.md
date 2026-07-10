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
