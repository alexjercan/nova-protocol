# Review: GOTO a gravity-well body parks into ORBIT on arrival

- TASK: 20260710-195954
- BRANCH: feature/goto-parks-into-orbit

## Round 1

- VERDICT: REQUEST_CHANGES

Verified sound by the reviewer: the engine-cooling observer is not needed
on the continued-engagement path (done already required near-cold
thrusters; ORBIT's allocation drives every engine next tick; the rotation
command evolves via slew, no PD degenerate zone); the telemetry clear
works as claimed (the Orbit arm never sets it, the None-with-has_telemetry
branch removes it next tick); no Changed<Autopilot> readers or
Add-observers exist, so in-place replacement is observably equivalent to
re-insertion, with the same one-tick latency as the keypress path; hints
([O] retires, [Z] stays), flight status (AP ORBIT + ring), GRAV line and
destination anchor all follow the action switch; breakout paths remove
the component regardless of action; the AI issues only GotoPos/Stop, so
no AI-parks-forever path exists; the reworked test's panic-on-None cannot
fire spuriously in the fixture; test counts and docs claims check out.
Tests re-run by the reviewer: flight 56.

- [x] R1.1 (MAJOR) flight.rs (done branch handoff) - no proximity gate:
  the degraded no-stopping-plan state zeroes the desired velocity at ANY
  distance, so a done-at-apex (or a ship at rest outside the SOI pressing
  G at an unstoppable well) hands off to ORBIT arbitrarily far out - the
  plan block clamps the ring to the band top and the ring correction is
  deliberately gravity-blind, i.e. the same failure class the
  gravity-aware arrival was built to fix. On master these cases released.
  Gate the handoff on actually being inside the park envelope, and fix
  the docs claim that the handoff only happens at the park point.
  - Response: fixed - the Goto arm sets `goto_arrived =
    numbers.distance <= settings.arrival_standoff` (the published
    distance is surface-relative, so this is exactly the inside-envelope
    test) and the handoff requires it; the degraded far-field done
    releases as master did. Docs rewritten to state the gate and the
    degraded-case behavior.
- [x] R1.2 (MINOR) the 1200-frame station-keeping loop only asserts
  above-surface; nothing checks the ship is on or converging to the
  ring - a slow decay or outward drift would pass.
  - Response: fixed - after the settle window the test extracts the plan
    and asserts the ship rides within 15u of plan.radius.
- [x] R1.3 (NIT) the old test's at-rest assertion was dropped entirely;
  at the handoff instant the done gate guarantees near-rest and asserting
  it preserves the arrival-curve check for free.
  - Response: taken - the test asserts speed < 0.5 at the frame the Orbit
    action is first observed.
- [x] R1.4 (NIT) one frame of stale GOTO telemetry can render alongside
  the Orbit action (the telemetry insert runs before the done branch on
  the handoff tick; the clear fires next tick).
  - Response: acknowledged, left as is - one fixed tick, invisible in
    practice, and reordering the publish around the done branch would
    complicate the tick for a cosmetic non-issue. Recorded here.

## Round 2

- VERDICT: APPROVE

All four round-1 findings verified resolved against 4db66ec (the reviewer
re-ran flight - 56 pass - and re-traced the gate: computed per ship per
tick before the done branch, radius-independent boundary math correct in
both arrival branches including the surface-clamped-to-zero case, both
reachable degraded paths now release as master did, GotoPos/Stop/Orbit
never set the flag and the handoff is doubly gated on the Goto pattern).
No new findings; the near-rest assertion's 0.5 bound has real margin over
the done gate's epsilons plus one tick of gravity.
