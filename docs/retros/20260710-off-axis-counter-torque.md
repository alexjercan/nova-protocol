# Off-axis counter-torque: recruiting laterals for a damage-shifted drive

Task: `tasks/20260709-224518`. Closes the "recruit off-axis thrusters" boundary
left by the thrust-balancing task (`docs/retros/20260709-thrust-balancing.md`,
"Scope and boundaries").

## The problem

Differential throttle balances torque *within the firing set* - the engines
already pushing toward the burn. It cannot help the most common damage case: a
ship with one centered main drive loses a side section, the COM shifts, and the
lone drive is now off-center with nothing in the firing set to trim against. It
fires at full and the PD holds the residual - or is pulled past its cap and the
ship pinwheels under its own burn.

## The model: wrench allocation with bounded lateral drift (decided with the user)

The fix generalizes `flight::balance_throttles` from the firing set to ALL live
engines: the flight computer may fire an off-axis thruster (a lateral, a retro)
purely for the counter-torque it produces about the live COM.

A recruited off-axis burn adds a sideways force the maneuver did not ask for.
Two ways to handle that were put to the user:

- **Zero net lateral (rejected)** - a hard constraint that the perpendicular
  force cancels exactly. Cleaner flight, but it needs an opposing thruster
  pair, so a ship whose damage left it exactly one usable lateral gets no help
  at all - which is exactly the damage case the task exists for.
- **Bounded drift (chosen)** - a soft penalty on the net off-axis force in the
  objective. Works with a single surviving lateral; the small drift is honest
  physics, and the autopilot's velocity-error rule corrects it over time.

### The allocation

The QP grows one term and keeps its structure:

```text
minimize   ||sum_i torque_i u_i||^2                   (net torque about the COM)
           + LATERAL_PENALTY * ||sum_i lateral_i u_i||^2   (off-axis force)
subject to sum_i forward_i u_i = demand               (deliver the commanded thrust)
           0 <= u_i <= 1                              (throttle box)
```

over ALL live engines, where the firing set (inside the burn's alignment cone)
is marked *primary*:

- A **primary** engine contributes `forward = magnitude * aligned` to the
  demand equality and only its perpendicular remainder to the penalty - the
  demand semantics of the thrust-balancing task are unchanged.
- A **recruit** (everything else) has `forward = 0` and its ENTIRE thrust
  vector in the penalty term. It is fired for its torque alone; all of its
  force is side effect, priced by `LATERAL_PENALTY`.

`LATERAL_PENALTY` has units of lever-arm squared: an off-axis engine with lever
arm `r` nulls all but `w / (r^2 + w)` of the torque it is recruited against. At
the shipped 0.05, a one-unit lever leaves ~5% residual (inside the PD's hold)
while an engine mounted nearly through the COM - huge force, no torque - is
correctly not worth firing (unit-tested both ways).

The projected-gradient solver is unchanged in shape: gradient of the two-term
objective, then the same bisection projection onto the demand equality - which
now only moves the primary engines, since recruits have zero forward
coefficient. The seed is the uniform throttle over the primary set with every
recruit dark; that seed is a stationary point whenever the net torque is
already null, so a symmetric ship returns unchanged and idle laterals are
never lit gratuitously (no effort-regularization term needed - gradient
descent simply never wanders into flat directions).

### Why recruits do not join the demand equality

The first implementation gave every engine its signed `forward` component and
let the equality see them. That died in the autopilot integration test, in an
instructive way: at full stick the demand equals the primary set's whole
capacity, so the equality has **zero slack**. One tick of bounded drift tilts
the world-frame error direction a fraction of a degree, the lateral's forward
component goes to -0.0017, and the projection - which must hold the demand
exactly - crushes the recruit straight back to zero. The balancer worked on
tick one and never again (peak recruit input 0.04 instead of 0.5).

Billing the recruit's whole force to the penalty instead keeps a saturated
demand feasible, and has a second honesty benefit: a recruited retro pays for
the thrust it cancels in the objective, instead of the mains silently
over-throttling to hide it.

### Headroom is no longer the only currency

The thrust-balancing doc's rule was "no throttle headroom, no balance". That
still holds for differential throttle, but a recruit's trim budget is **its own
throttle box**, not the firing set's headroom: a full-stick burn on the
damage-shifted single drive is now held straight, because the lateral needs no
forward slack to fire (unit test
`balance_throttles_counter_torques_even_at_full_throttle`, physics test at
full stick). The no-help floor moved: only a ship with *no usable off-axis
engine left* still pulls, exactly the pre-allocation behavior.

## Where it lives

Both input paths build the same allocation, at the same places as before:

- `autopilot_system`: every live engine is collected (not just the cone) with
  world-frame coefficients about the world COM; cone membership (with the
  existing lit-engine hysteresis) sets `primary` and the firing authority that
  caps the demand. Engines spool toward their allocated throttles; recruits
  spool like any other engine, so the counter-torque plume is visible and the
  cost is diegetic.
- `manual_burn_system`: every live *unbound* engine is collected in the
  ship-local frame (bound thrusters keep their own keys, as before);
  forward-aligned engines are `primary` and the analog burn is their demand.

## The cost stays legible

A recruited lateral spends thrust that does not go toward the goal. That cost
is not hidden anywhere: the engine's input is a real `ThrusterSectionInput`, so
its plume, hum, and impulse are the counter-torque burn, and the sideways
drift it produces is real velocity the arrival control visibly corrects. The
task's honesty note is satisfied by physics, not by a UI element; if a HUD
readout of "trim thrust" is ever wanted, the throttle vector is already
per-engine.

## Scope and boundaries (deliberate)

- **AI ships** (`input/ai.rs`) remain a separate control path, untouched, as
  with all flight-layer work so far.
- **The drift is bounded, not zero.** A hard zero-net-lateral mode (for ships
  with opposing pairs) was considered and deliberately not built; if flight
  feel ever wants it, it is one more equality in the same QP.
- **Fuel** is not modeled in the game yet; when it is, recruited thrust burns
  it like any other burn by construction (same input pathway).

## Verification

- Solver unit tests: recruit to a hand-computed optimum
  (`uL = 2 * 0.5 / (4 + 0.05)`), idle laterals stay exactly dark on a balanced
  ship, counter-torque at full throttle, and a through-the-COM lateral is not
  trusted with the torque. Existing balance tests unchanged (primary-only
  inputs reproduce the old model exactly).
- Physics test `single_drive_on_a_shifted_hull_recruits_a_lateral_to_hold_heading`:
  a full-stick burn on a damage-shifted hull (ballast 6 units off the
  centerline, lone main drive) holds heading within the centered tolerance
  with the lateral lit (input ~0.5), and the same hull without the lateral
  still pulls - the floor is unchanged.
- Physics test `autopilot_burn_recruits_a_lateral_on_a_shifted_hull`: the
  autopilot STOP on the same hull recruits the lateral through the world-frame
  path and converges to rest despite the bounded drift.
- All 38 `flight::` tests pass; workspace `cargo check` and `cargo fmt` clean.
  (Full workspace suite and clippy deferred to CI per project practice.)
