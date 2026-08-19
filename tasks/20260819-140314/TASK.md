# Attitude control by physics: a structural ceiling and a torque budget

- STATUS: OPEN
- PRIORITY: 76
- TAGS: v0.11.0,spike,physics,ship,flight

Epic: `20260818-220812`. **SPIKE - decide and prove the model before retuning
content.** Owner wants attitude control to be physically correct.

## The complaint that started it

A 1-1-1 ship (controller, hull, thruster) "is impossible to handle, it's very
hard, turns slow and meh".

It is not a bug. `max_angular_acceleration: 0.5` rad/s2 is deliberate, and the
commit that set it (`2b03a2f8`) states the target: "0.5 rad/s2 gives a bang-bang
180 an ideal 5.0 second turn: 2 * sqrt(pi / 0.5)". Max commanded rate is
`sqrt(pi * alpha) / 2` = 0.63 rad/s, about 36 deg/s.

The real defect is that `2b03a2f8` was titled "Make controller authority
size-independent", and it succeeded: **a 1-1-1 now turns at exactly the same
rate as a hundred-section capital.** A minimal ship handles like a barge because
the system guarantees it does.

## The two halves nobody had at once

Physics: `alpha = torque / I`, and the structural load at a point `r` from the
axis of rotation is `alpha * r` tangential plus `omega^2 * r` centripetal.

- **`max_torque` (before `2b03a2f8`)** was the PROPULSIVE limit alone. Correct
  as far as it went, but with no structural cap it over-punished size, because
  `I` carries `r^2`. Big ships were unsteerable.
- **`max_angular_acceleration` (now)** is the STRUCTURAL limit alone, and flat.
  Correct as far as it goes, but size stopped mattering at all.

Neither was wrong. Each was half.

```
alpha_max = min( torque_available / I ,  a_limit / r )
             \___ propulsive ___/        \_ structural _/
```

- Small ship: `I` tiny, so `torque/I` is huge -> STRUCTURE-limited.
- Capital: `I` enormous -> TORQUE-limited.

Two different reasons for two different ships, both physical. That is the model.

## Owner decisions already made

- **`r` is measured from the CENTRE OF MASS** to the furthest section, not the
  geometric centre.
- **The flight computer is a FRACTION of the ceiling**, not a source of
  authority: `alpha_commanded = alpha_ceiling * f`, `f` in 0..1. The hull owns
  the ceiling; the computer decides how close to it you dare fly. This is
  envelope protection - a better computer models the hull more precisely and can
  ride nearer the edge without overshooting.
- Stacking follows rather than being imposed: `f_total = 1 - (1 - f)^n`.
  Asymptotic to 1.0, needs no cap, because you cannot exceed physics. The
  current "capped at twice its strongest computer" rule goes.

## What the tree already has

- `BodyRadius` is in `nova_ship::prelude`.
- `pd_controller.rs` already reads `ComputedAngularInertia`, so `I` is present
  where it is needed.
- **`flight/thrusters.rs` already solves a convex QP that NULLS net torque**, so
  engines give pure translation and the PD controller owns rotation alone. Off
  axis thrusters DO produce torque - `thruster_impulse_system` calls
  `apply_linear_impulse_at_point` - the balancer deliberately cancels it.

That balancer is the seam. Under this model a side-mounted engine is a genuine
source of turn authority, and nulling it is throwing away propulsion the ship
paid mass for. Decide whether thrusters CONTRIBUTE to the torque budget or stay
translation-only, and say why.

Note: `thruster_section.rs:361`'s comment about "a COM-centered engine torques
the ship it must not touch" is about a STALE-POSE bug, not a claim that
thrusters cannot torque. Do not read it as the latter.

## Worth taking, cheap

- **The loads combine as a VECTOR**: `sqrt((alpha*r)^2 + (omega^2*r)^2) <=
  a_limit`, not each bounded separately. A ship already in a hard turn has spent
  its margin and cannot also change rate quickly - big ships become committed to
  the turn. Falls straight out of doing it correctly.
- **The limit is PER SECTION**: `min over sections of (strength_s / r_s)`. A
  fragile part on a long boom limits the whole ship. Hull layout becomes a
  handling decision.

## Risks to size before committing

- **The envelope must be VISIBLE.** A physically correct limit nobody can see
  reads as arbitrary sluggishness - which is the report that opened this task. A
  load readout, or the ship protesting near its limit, is what turns a
  constraint into a mechanic. Without it this task can land correct and still
  feel worse.
- **Every authored ship's handling changes.** The campaign is tuned to current
  feel and `wfc_ships` generates hulls with no notion of a structural budget.
  The retune is the expensive half of this work; size it before starting.
- **A damaged ship's envelope MOVES.** `r` and `I` both change as sections die.
  Losing the nose raises `alpha_ceiling`; losing a controller lowers `f`. Partly
  self correcting, but it will read as a bug the first time a half-dead ship
  handles sharper than it did intact. Decide whether that is a feature.

## Done when

- The model is decided and RECORDED here before code moves.
- Measured turn rates for a 1-1-1, a mid hull and a capital, before and after,
  with the owner flying them - their verdict decides, not the arithmetic.
- The thruster-torque question is answered either way, with a reason.
- Shipped content is retuned or the divergence is listed and scheduled.
