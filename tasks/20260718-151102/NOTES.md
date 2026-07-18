# RCS error-relative mode: design record

This doubles as the spike output for the RCS spike's Fork 4 deferred question
(tasks/20260718-122508/SPIKE.md): how to let the autopilot use RCS for ORBIT
station-keeping when the ship moves faster than the fine-adjust cap.

## The problem

`rcs_burn_system` capped the ABSOLUTE along-axis speed: for each ship-local
axis it computed `along = velocity.dot(world_axis)` and pushed only while
`along` was below the cap in the commanded direction. That is correct for the
player fine-adjust (accelerate up to a small ceiling) and for the STOP/GOTO
terminal settle (brake to rest, i.e. cap relative to zero). But an ORBIT moves
at `v_circ = sqrt(mu/r)` ~= 2.5-6 u/s, well above the 2 u/s cap, so:

- a prograde trim gated to zero (already past the cap), and
- a retrograde trim BRAKED the orbit instead of trimming it.

RCS as built could not express "add a small correction while already moving at
orbital speed". This is why task 20260718-122932 shipped only the terminal
settle and split ORBIT-via-RCS out to this task.

## The fix: a reference velocity the cap is measured against

Add an optional `RcsReference(Vec3)` component (world-frame velocity, on the
ship root). `rcs_burn_system` now caps `(velocity - reference)` along each axis
instead of `velocity`:

```rust
let reference = reference.map(|r| r.0).unwrap_or(Vec3::ZERO);
...
let along = (velocity - reference).dot(world_axis);
```

The whole design turns on one identity: **absent or zero reference reproduces
the old absolute cap byte-for-byte** (`v - 0 == v`). So the player fine-adjust
mode and the STOP/GOTO terminal settle - neither of which sets a reference -
are completely unchanged. Only the autopilot's ORBIT branch writes a non-zero
reference, and only it sees the new behavior.

The autopilot writes `reference = desired` (the orbital velocity) while
station-keeping, so the burn caps the RESIDUAL `v - v_orbit` (the trim) at the
cap. A small prograde/retrograde correction now has full headroom; the trim
fades as the residual does (the proportional `error / cap` command handles
convergence, exactly as the STOP settle does around rest).

## Autopilot gating

Two RCS branches now share one command formula (`error / rcs_cap`), differing
only in the cap's reference frame:

- `use_rcs_settle` (unchanged): `desired ~= 0` and `|v| < cap`. Reference is
  zero. Brakes the last meters to rest.
- `use_rcs_orbit` (new): `is_orbit` and `error_speed < cap`. Reference is
  `desired`. Trims a sub-cap residual on a fast orbit.

`is_orbit` is set only in the `AutopilotAction::Orbit` match arm, so the trim
never touches GOTO's cruise leg (whose steady-state velocity could otherwise
be mistaken for a sub-cap "trim" and hand the main drive off mid-transit). The
handoff is automatic: while the residual is above the cap (spin-up from rest,
or a large ring correction) the main drive does the work as before; once the
ship is near orbital velocity the residual drops sub-cap and RCS takes the
trim, with `demand` zeroed so the two never double-push.

## Cleanup (shared-primitive-clear-on-handoff)

`RcsReference`, like `RcsIntent`, is a side-effecting component any driver acts
on. It is:

- written EVERY autopilot tick (`desired` when trimming, `Vec3::ZERO`
  otherwise), so a stale orbital reference never lingers into a settle; and
- zeroed in `on_autopilot_removed_cool_engines` on disengage, so it never
  silently rebases the player's next absolute-cap nudge.

The `orbit_rcs_reference_clears_on_disengage` test pins the off-ramp.

## Alternatives considered

- **A second sibling burn system for the error-relative mode.** Rejected: it
  duplicates the per-axis taper math and the verb/mass/COM plumbing, and would
  drift out of sync with the player path. One system with a reference term is
  simpler and keeps the two modes provably consistent.
- **A mode flag/enum on RcsIntent.** Rejected: the thing that actually differs
  between the modes is a physical quantity (the reference velocity), not a
  discrete mode. Modelling it as that quantity makes "player" and "settle" the
  natural `reference = 0` case rather than a separate branch.

## Known limitations / follow-ups

- The trim/main-drive handoff at `error_speed == cap` has no hysteresis; if a
  playtest shows chatter at the boundary, add a small dead-band. Not observed
  in the headless orbit-hold test.
- The terminal-creep limitation from task 20260718-122932 (STOP settles to the
  deadband, not stop_speed_epsilon) is unchanged and still tracked there.
