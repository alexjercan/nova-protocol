# Notes

Items 1-3 landed. Items 4 (torpedoes intercepting torpedoes) and 5 (reactive
dodging) deliberately not built - see the evaluation at the end.

## 1. Terminal weave

A corkscrew laid over the guidance command: `TorpedoWeave` carries a tilt
direction that spins about the command at `weave_rate` and tilts it by
`weave_angle`. Perturbation, not replacement - at zero angle the result IS the
guidance solution, so every property the PN law has survives.

The tilt direction is CARRIED as state and advanced incrementally rather than
rebuilt from a basis each step. `any_orthonormal_pair` flips branch as the
command crosses an axis, which would snap the corkscrew a half turn mid-flight.

Faded to nothing between three blast radii and the proximity fuze, and gated on
`TorpedoArming` so the sideways launch turn is left alone.

### What the measurements actually said

The two headline knobs turned out to do quite different jobs, which was not the
prior expectation:

- **`weave_angle` is the balance knob.** Rounds per intercept track it and are
  all but blind to the rate. Breaking a straight-line lead extrapolation does
  not need a FAST turn, it needs a sustained one.
- **`weave_rate` is the look knob.** At a fixed 0.44 rad the intercept cost is
  ~1245 rounds at every rate from 0.7 to 2.2 rad/s (400 u envelope), while the
  visible swing goes 23.6 u -> 6.1 u across that range.

So the rate was chosen for the picture (1.4 rad/s, ~11 u swing) and the angle
for the exchange (0.44 rad).

### The trap the pure-math rigs walked into

The first tuning pass scored the weave with a guidance-only sim whose attitude
model was a rate limiter. That model tracks the commanded cone almost perfectly;
the real body does not, because the PD attitude controller, the thrust law and
the linear drag each attenuate it. The first shipped values (2.2 rad/s) produced
a weave the sim scored at 4x cost and which was INVISIBLE on screen.

Fixed by adding `the_weave_puts_a_visible_bend_in_the_real_flight_path`, which
runs the real stack (avian, `PDControllerPlugin`, `SpaceshipSectionPlugin`)
against a stationary target at 300 u and measures the flown path: 10.6 u of
lateral swing weaving against 0.0 u straight. That test is the one to trust
when retuning; the `point_defense_cost_tests` module is a RATIO, not an
absolute.

Its rig overwrites the fresh torpedo's pose and velocity onto the line at
cruise. A bay fires along its +Y while the torpedo's nose is its -Z, so every
real launch spends ~10 u of lateral excursion turning onto course - on both
arms, monotone, and an order of magnitude wider than the weave under test.

## 2. Hard torpedo magazines

`reload: None` on both shipped bay builders. The siege bay is deliberately left
unlimited: it is dressing for a looping menu backdrop, not a combat participant.

The `menu_gauntlet` batteries relaunch every 15-24 s, so a hard six-round
magazine lasts 90-144 s - past the point the backdrop's corvette dies and the
carousel turns. No content change needed there.

## 3. Per-turret point defence

`TurretSectionArc`, solved once from the joint tree. The general question is IK;
every shipped turret is an unlimited traverse hinge carrying a limited elevation
hinge, and that shape has a closed form because traversing does not change
elevation. Trees that do not match get no component and bear anywhere.

Own-hull raycast occlusion was NOT added. `ai_line_of_fire_blocked` deliberately
treats the shooter's own colliders as transparent (the muzzle sits on its hull),
and the depression floor already IS the geometric statement of "my hull is in
the way" for a hull-mounted mount. A ray that had to ignore only the muzzle's
immediate neighbours would be both expensive and fragile.

Dwell: hold until the target dies, leaves the arc, or a rival arrives in less
than half the time to impact. Acquisition shaves ~3 degrees off the arc and
holding does not, so a torpedo drifting across the depression floor is not
picked up and dropped on alternate frames.

## Ordnance durability - still open, but the derivations were wrong

Both prior derivations (~400-500 hp from Starsector, ~2054 hp from Nova's own
spike) assume every point-defence round that is fired LANDS. The weave has
falsified that: measured effective hit rate against a weaving torpedo is ~0.8%
of rounds fired. Scaling either number by that is meaningless.

With the landed 150 u envelope, a weaving torpedo at the shipped 10 hp already
survives 3.7 s of one PDC's continuous fire and dies 39 u from its target.
Nothing in the hundreds is warranted. If playtest wants "survives one mount,
stopped by two", the number is ~15-20, not 400+.

## Not built, and why

- **Item 4 (torpedoes intercepting torpedoes)** - a new targeting mode, and the
  task says to land 1-3 first and see.
- **Item 5 (reactive dodging)** - item 1 does not fall short. At the landed
  envelope the open-loop weave already triples the ammunition an intercept costs
  and carries a torpedo from a kill at 114 u to a kill at 39 u. Closed-loop
  awareness of individual rounds would add real complexity to a mechanism that
  is already delivering. Recommend not building it.
- **The player-side auto-engage toggle** in the definition of done. Player
  turrets have no autonomous point defence at all today - they follow the
  crosshair and the combat lock - so the toggle is the visible half of a feature
  whose other half (player turrets defending themselves) does not exist yet.
  That is its own piece of work, not a switch.
