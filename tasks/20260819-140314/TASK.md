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

### NOT blocked - the density scare was a misread

Recorded, then retracted the same day. A claim that authored ships carried
densities of 70-350 against a default of 1.0 - giving the cargoa ~660x the
inertia of a standard hull - was built on reading `part()`'s 7th positional
argument as mass. It is HEALTH
(`crates/nova_authoring/src/base_content/ships/shared.rs:101`), and `PartSpec`
has no density field.

Every section in shipped content is `mass: 1.0` - all 32 in the generated
`assets/base/**/*.content.ron`. Sections already derive mass from collider
volume at density 1.

So the cargoa is STRUCTURE-bound like everything its size. Measured, not
estimated - see the readings below: mass 15.86, `I_yy` 35.73, r 2.76 u / 27.6 m,
flip 2.10 s at 8 G against the 1-1-1's 1.55 s, with 14.8x of torque headroom.
That is the shape this model wants, and it lands for free.

`20260819-173840` survives only as a cleanup: delete the vestigial
`SectionConfig.mass` field, which is a density named mass and is 1.0 everywhere.
It does not block this task.

### CALIBRATION - decided 2026-08-19

`load_limit` = **8 G = 78.48 m/s2**. Crossover target **10 u = 100 m**: below it
structure binds, above it torque binds.

**The scale is load-bearing and was nearly missed.** Nova runs at 1 world unit =
10 METRES (`turret_section/aim.rs:21`, "Nova's scale is 100 u = 1 km"). A G limit
is a real acceleration, so the ceiling is

```
alpha_ceiling = load_limit / (r_units * METRES_PER_UNIT)
```

and dropping the conversion makes every ship ten times sharper than intended. An
earlier draft of this calibration did exactly that and landed on 2 G, which at
true scale flips a 1-1-1 in 3.10 s against today's 5.00 s - it would not have
fixed the complaint that opened the task.

**`METRES_PER_UNIT` exists, but not where physics can reach it.** Corrected
2026-08-19: it is `crates/nova_ui/src/units.rs:13`, `= 10.0`. The earlier claim
here that no constant existed was wrong.

It is however declared DISPLAY-ONLY - `units.rs:8`: "World-space transforms,
physics, content RON and AI tuning keep raw world units; only the strings a
player reads pass through here" - and `nova_ship` does not depend on `nova_ui`
(checked both manifests). So this work still needs the scale on the physics
side, by PROMOTING that constant into a crate both can see, never by declaring a
second one. Two constants that must agree is the same fault as a re-typed id
(`20260819-131004`), and this one goes wrong silently by a factor of ten.

No existing leaf serves both: `nova_ship` sees `nova_events`, `nova_ui` does
not. Picking the home is part of the implementation.

Expected flip times at 8 G, bang-bang 180:

| ship | r | flip | binds |
|---|---|---|---|
| 1-1-1 | 1.5 u / 15 m | 1.55 s | structure |
| racer yacht | 2.18 u / 21.8 m | 1.87 s | structure |
| cargoa corvette | 2.76 u / 27.6 m | 2.10 s | structure |
| cargob hauler | 2.93 u / 29.3 m | 2.17 s | structure |
| mid hull (hypothetical) | 5 u / 50 m | 2.83 s | structure |
| capital (hypothetical) | 15 u / 150 m | 8.14 s | TORQUE |

The three named craft are MEASURED (below). `r` runs from the measured COM to
the OUTER FACE of the furthest section - the rule that gives the 1-1-1 its
1.5 u. State it: to the section's CENTRE the cargoa reads 2.25 u and to its
furthest CORNER 2.88 u, which is 1.90 s to 2.15 s in flip time. Half the
bounding box, 2.45 u, is a different quantity again - the COM sits 0.24 u aft
of the geometric centre - and is not used.

Today every one of them flips in 5.00 s. So everything that ships sharpens by
2.3x to 3.2x and capitals get 1.6x SLOWER - that second half is the retune cost,
and it is intended: a capital should be a barge.

### MEASURED - the shipped fleet's mass properties

Read out of avian on 2026-08-19: each hull spawned as its authored
`SectionCollider::Cuboid` children at `ColliderDensity(1.0)` under one
`RigidBody::Dynamic`, then `ComputedMass` / `ComputedCenterOfMass` /
`ComputedAngularInertia`. Reproduces a parallel-axis sum to 4 dp, so the two
methods agree and neither is an assumption. The collider is the AUTHORED BOX,
not the GLB - the mesh is render-only.

| hull | colliders | mass | `I_yy` | r (face) | binds | headroom |
|---|---|---|---|---|---|---|
| 1-1-1 | 3 | 3.00 | 2.50 | 1.50 u / 15.0 m | structure | 115x |
| racer | 7 | 8.28 | 10.86 | 2.18 u / 21.8 m | structure | 38x |
| cargoa | 9 | 15.86 | 35.73 | 2.76 u / 27.6 m | structure | 15x |
| cargob | 9 | 18.95 | 48.79 | 2.93 u / 29.3 m | structure | 12x |

Cargob and the racer are UNREMARKABLE: same pattern, same density, no surprises.

### `max_torque` is NOT yet a number - and no hull can settle it

The crossover fixes `max_torque` only once `I` at 10 u is known, and the largest
hull in the game is 2.93 u. A provisional 1501 falls out of
`I(r) = 2.5 * (r/1.5)^3.5`, whose anchor is exact - three unit cubes in a line
about their COM is 2.5 - but whose **3.5 exponent is a guess**. All three
shipped hulls come out HEAVIER than that curve predicts (1.18x, 1.69x, 1.87x),
so **1501 is a floor, not an estimate.**

**`I` is not a function of `r`, so there is no exponent to find.** Anchored on
the 1-1-1 the shipped hulls want p = 3.94, 4.36, 4.43, and a single p = 4.33
holds all four to 14 %. Drop the anchor and fit the three against each other and
the slope is 5.04 - a 3.9x disagreement in `max_torque` at the crossover. `I` is
the second moment of where the mass sits: the cargoa's fuselage is 43 % of its
mass and 16 % of its yaw inertia, its nose 16 % of the mass and 31 % of the
inertia. **No inertia formula belongs in the code.** avian computes it exactly
and `pd_controller.rs` already reads `ComputedAngularInertia`; the `I(r)` curve
is a drawing aid for the explainer's plates and nothing else.

What the gap does NOT touch: every shipped hull is structure-bound, so its flip
time contains no `I` at all and `load_limit` alone fixes it. Only the
hypothetical capital end waits, and it moves the safe way.

### `load_limit` has no physical derivation, and that is fine

1 g is a made-up test point, not a measurement. It is "how strong is spaceship
hull metal", which is a game-design choice. It is also the SINGLE dial that sets
the whole size curve - raise it and every ship sharpens, lower it and everything
commits harder - while the RATIOS between ship sizes stay fixed by geometry
whatever it is set to. One number to playtest, and the first one to tune.

### The readout is the EDITOR's job, not this task's

Recorded 2026-08-19 in the editor epic `20260812-131912`, under "the engineer's
NOVA OS". The build screen owes an engineer's panel - turn rate, torque, which
limit binds, mass, thrust, power - the way Factorio puts craft time and power
draw on the machine itself. The attitude readout is that panel's first tenant.

This task therefore does NOT build the readout. It must not land in a release
where nothing shows the number, because a correct limit nobody can see reads as
arbitrary sluggishness - which is the report that opened this task.

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

- `pd_controller.rs` already reads `ComputedAngularInertia`, so `I` is present
  where it is needed, exact, and needs no formula.
- `BodyRadius` is in `nova_ship::prelude` but does NOT serve as `r`. It is the
  geometric radius of a scenario OBSTACLE (`flight/state.rs:7`), derived for
  asteroids and left unset on ships - "fine for ships and debris". The
  COM-to-furthest-face arm has to be derived; do not reach for this component.
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
- The RELEASE this lands in shows the ceiling and the binding limit on the build
  screen. Built by `20260812-131912`, not here - but not shipped without.
