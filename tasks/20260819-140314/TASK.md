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

## Owner decisions - SETTLED, do not reopen

Reviewed 2026-08-19. **Exactly TWO authored knobs, and both must be readable by
a modder without a physics lesson.** That bar killed a third.

- **`max_torque`, on the controller.** How hard this computer twists the ship.
  Controllers ADD: two computers, twice the torque. No cap, no curve.
- **`load_limit`, ONE GLOBAL CONSTANT.** Not authored per section. Hull metal is
  hull metal, so there is no "which section's limit" question to answer. A
  per-section limit - `min over sections of (strength_s / r_s)`, so a fragile
  part on a long boom limits the ship - is a possible LATER refinement and is
  explicitly not in this task.
- **`max_angular_acceleration` is DELETED.** It becomes computed, not authored.
  That is the whole point: angular acceleration should depend on the ship.
- **`r` from the CENTRE OF MASS** to the furthest section, not the geometric
  centre.

### `envelope_fraction` was proposed and REJECTED

An earlier draft gave the controller a second knob - what fraction of the
structural ceiling it dares command, stacking as `1 - (1 - f)^n`. Owner: "feels
like it adds a wtf param... I want the controller to have instantly readable
knobs".

They are right, and it does not survive scrutiny either: **the structural
ceiling already caps the result.** Install enough torque and you turn at the
hull's limit; physics stops you there. A second fraction on top is a fudge
factor doing a job already done. Dropping it also deletes the `1 - (1 - f)^n`
stacking rule outright, which existed only to serve it.

So stacking needs no rule at all. Controllers add torque; structure caps it.

### Thrusters MUST NOT CHANGE

Settled. `flight/thrusters.rs` keeps its torque-nulling balancer. An off-axis
engine creating angular momentum is a legitimate player trick and stays one -
it is NOT part of the torque budget and the model does not account for it.
This closes the largest open question in the original draft.

### A damaged ship's envelope moves, and that is a FEATURE

Lose sections, `r` shrinks, the ship turns sharper. Intended, not a bug to
design around.

### `load_limit` has no physical derivation, and that is fine

1 g is a made-up test point, not a measurement. It is "how strong is spaceship
hull metal", which is a game-design choice. It is also the SINGLE dial that sets
the whole size curve - raise it and every ship sharpens, lower it and everything
commits harder - while the RATIOS between ship sizes stay fixed by geometry
whatever it is set to. One number to playtest, and the first one to tune.

### The pit this design digs, and the UI that fills it

A big enough ship genuinely cannot turn. That is correct physics and it must
stay, but it is a trap if a player finds out by flying it.

Adding controllers works - 1000 controllers is 1000x the torque - and it is not
free, because a controller has mass and mass at radius `r` adds `m*r^2` to
inertia. Wheels on the wingtips nearly cancel themselves; wheels amidships pay
off. Real spacecraft engineering, free from the model.

**But none of that is discoverable without a readout.** The build screen must
show the resulting turn rate AND which limit binds ("0.03 rad/s2,
torque-limited") while sections are being placed. Without it this design is a
pit, and the frame-rate lesson from `20260819-012130` applies: a correct number
nobody can see reads as the game being broken.

## What the tree already has

- `BodyRadius` is in `nova_ship::prelude`.
- `pd_controller.rs` already reads `ComputedAngularInertia`, so `I` is present
  where it is needed.
- **`flight/thrusters.rs` already solves a convex QP that NULLS net torque**, so
  engines give pure translation and the PD controller owns rotation alone. Off
  axis thrusters DO produce torque - `thruster_impulse_system` calls
  `apply_linear_impulse_at_point` - the balancer deliberately cancels it.

That balancer STAYS. See the settled decisions above - thrusters are out of the
torque budget and out of this task.

Note: `thruster_section.rs:361`'s comment about "a COM-centered engine torques
the ship it must not touch" is about a STALE-POSE bug, not a claim that
thrusters cannot torque. Do not read it as the latter.

## Worth taking, cheap

- **The loads combine as a VECTOR**: `sqrt((alpha*r)^2 + (omega^2*r)^2) <=
  a_limit`, not each bounded separately. A ship already in a hard turn has spent
  its margin and cannot also change rate quickly - big ships become committed to
  the turn. Falls straight out of doing it correctly, and adds no knob.

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
