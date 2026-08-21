# Enumeration before writing

The task's first hour, done. What depends on a gun round being a physics body,
and what the enumeration found that the task did not know.

## The task premise is wrong on gravity

TASK.md says:

> **`Gravity::ZERO`** (`NovaGameplayPlugin`). There is no arc today to lose.

Global gravity is zero. Nova's OWN gravity layer is not, and turret rounds are
opted into it:

- `crates/nova_gameplay/src/gravity.rs:277` -
  `insert_gravity_affected_on_turret_round`, an `On<Add,
  TurretBulletProjectileMarker>` observer that inserts `GravityAffected`.
- `gravity.rs:795` - `a_turret_round_curves_under_a_well_and_a_gravity_free_body_does_not`,
  a shipped test that measures 3.25u of deflection against a straight-flying
  control.
- `gravity.rs:850` - a benchmark that deliberately keeps rounds on the SHARED
  force path, having measured the marginal cost at ~0.1 ms/tick over 1500
  bodies. The comment records that a lighter bullet-only path was left unbuilt
  "because the measurement did not call for it".
- The shakedown scenario authors a well
  (`nova_authoring/.../shakedown/tests/pins.rs:434`).

So rounds curve TODAY. Gravity is not future work for this task; it is existing
behaviour the conversion must carry.

This does not invalidate the task - it makes the integrate-then-cast structure
MANDATORY rather than a courtesy to a future feature.

## The trap: the test that guards the curve cannot see the break

`gravity_well_system` reads `Forces`, an avian system param backed by the force
accumulators that `RigidBody` requires. A round that stops being a body stops
matching `q_affected` and silently stops curving.

The guarding test does not catch this. `spawn_probe` (`gravity.rs:726`) builds
its own `RigidBody::Dynamic` + `GravityAffected` fixture; it never spawns a
production round. After the conversion it still passes, and it still proves
exactly what it proved before - that the marker curves a BODY.

That is the whole failure mode: green suite, rounds fly straight, nobody
notices until a playtest near a rock.

## What else touches a round

Checked, and safe as-is:

- `resolve_bullet_hit` (`turret_section/firing.rs:44`) - the collision
  observer. Replaced wholesale by the swept path. Its mirrored-event
  de-duplication and its `damage.amount = 0.0` "second contact in the same
  flush" guard both exist only because contacts arrive as events; an ordered
  hit list has neither problem.
- `ProjectileHooks` (`nova_gameplay/src/projectile_hooks.rs`) - stays for
  torpedoes. The bullet arm of it (`owner_of` reading `ProjectileOwner` off the
  collider entity itself) goes unused; `a_turret_bullet_still_ignores_the_ship_that_fired_it`
  is a fixture test like the gravity one and would keep passing on a hand-built
  body. The owner rule moves into the cast filter.
- `ai_line_of_fire_blocked` (`input/ai/guns.rs:212`) - casts with
  `SpatialQueryFilter::default()`, so today it CAN hit rounds; its predicate
  already rejects sensors, and rounds are sensors. Unchanged either way.
- `pressure_at_target` (`nova_gameplay/src/damage.rs:563`) - blast line of
  sight. `ray_hits` can return rounds today; the loop only acts on
  `SectionMarker` health, so rounds are already ignored. Unchanged.
- `insert_projectile_render` / bullet audio cue / `stress_bullets` /
  `stress_point_defense` / `system_turret_gunnery` / `wfc_arena` - all keyed on
  `TurretBulletProjectileMarker` or `On<Add, ...>`, neither of which changes.
- `nova_probe` snapshot `ordnance_record` (`snapshot.rs:951`) - reads
  `Transform`, not `Position`. Unchanged.
- `TransformInterpolation` - `bevy_transform_interpolation` eases plain
  `Transform` in `FixedFirst`/`FixedLast` and needs no rigid body, so a round
  whose `Transform` is written in `FixedUpdate` keeps its smooth render for
  free. The muzzle-pop seed stays as-is.

Needs an edit:

- `nova_debug/src/sections.rs:78` - `draw_turret_bullet_projectile` queries
  `(&Position, &LinearVelocity)`. `Position` is avian's and disappears with the
  body.
- `NEUTRALIZED_BULLET_MASS`, `AngularInertia::from_shape`, `Mass`, `Sensor`,
  `CollisionEventsEnabled`, `ActiveCollisionHooks::FILTER_PAIRS`,
  `Collider::sphere(0.05)`, `RigidBody::Dynamic` - all leave the spawn bundle.
  The neutralized mass and the angular inertia exist only to keep avian quiet
  about a body it should never have had.

## The decision this needs

Rounds need the well acceleration without `Forces`. Two shapes:

### (a) Generalise `gravity_well_system` to drive non-bodies too

One system keeps owning gravity for ships, torpedoes and rounds. The affected
query splits into a body arm (`Forces`) and a velocity arm (the round's own
velocity component), or the system writes an acceleration component that each
integrator consumes.

Blast radius: touches the force path that ships and torpedoes ride. Keeps
`DominantWell` and its hysteresis on rounds.

### (b) The round integrator calls the pure helpers directly - RECOMMENDED

`well_accel` and `dominant_well` are already pure functions, extracted for
exactly this reason (the module doc: "so the well-force core stays
game-agnostic"). The swept step calls them and adds the acceleration itself.

This is the "lighter no-`DominantWell` path" `gravity.rs:850` says was left
unbuilt. Building it now is free, because the round integrator has to exist
regardless.

Blast radius: none. `gravity_well_system` is untouched; ships and torpedoes
keep the exact path they have. `insert_gravity_affected_on_turret_round` and
its observer are deleted.

Costs: rounds lose `DominantWell`, so they lose SOI-switch HYSTERESIS in
overlapping wells - each tick picks the strongest pull outright. Nothing reads
`DominantWell` on a round (the HUD and the ORBIT verb read it on the player
ship), and a round lives ~2s, so this is a difference no one can observe. Say
so in the doc rather than pretending the paths are identical.

Bonus the task's estimate does not count: rounds leave `q_affected` entirely,
so the ~0.1 ms/tick gravity cost and the `DominantWell` insert/remove archetype
churn go with them.

## Amendment to the DoD

Add: **a round still curves under a well, proved on a PRODUCTION round.**

The existing gravity test cannot prove it (see the trap above). The range that
covers it must fire a real turret near a well and measure deflection, or the
conversion ships a silent behaviour loss with a green suite.

---

# Accepted: (b), the round integrator calls the pure helpers

Owner's call, 2026-08-21, on the fork above. Reasoning recorded with it: one
place for gravity is attractive for a future n-body model, but the shared thing
that matters there is the MATH, and `well_accel` / `dominant_well` already are
that shared place. Only force APPLICATION differs. Refactor when n-body
actually arrives rather than build for it now.

DoD amended with: a round still curves under a well, proved on a production
round.

## What the conversion had to solve that the task did not foresee

### 1. A sweep has no notion of a contact BEGINNING

`CollisionStart` fired once when a pair started overlapping. A cast fired fresh
each step re-finds any collider the round is still INSIDE, and a section is
routinely thicker than one step's travel - a 4-unit plate at 100 u/s takes
three steps to cross. A Pierce round bit the same plate three times and dealt
60 where it authored 20.

`RoundBitten` is the memory that restores once-per-round. It is a ring, because
the oldest entry is the one the round has travelled furthest past.

Caught by the ported pierce tests, which is the argument for porting them
rather than rewriting them: three of the twelve failed on the first run and
each named the exact arithmetic that had moved.

### 2. Three firing tests built an app where avian moved the round

`bullet_stream_stays_linear_at_high_ship_velocity`,
`first_rendered_frame_attaches_the_bullet_to_the_eased_muzzle` and
`fire_rate_above_the_tick_rate_keeps_its_true_cadence` each assemble
`PhysicsPlugins` and `shoot_spawn_projectile` by hand. They needed
`NovaRoundPlugin`, or nothing moved the rounds they fired.

The stream test also read avian's `Position` for the RAW stream. A round has
none; the raw pose is now `TranslationEasingState.end`, which is what the sweep
integrated at the close of the last fixed tick.

## Measurement: what is comparable and what is not

**The body-count regime floor cannot gate these two arms.** The change removes
~770 bodies by design, so a floor calibrated on the old arm selects nothing on
the new one. `stress_point_defense` holds a SCRIPTED saturation rather than
fighting to the death, so the same beat is the same workload in both arms and
the range's own counts are the comparable readings.

Note `assert_the_battery_connected` and `assert_the_sky_filled` are
`#[cfg(feature = "debug")]`. A range binary built without it runs and prints
but asserts NOTHING - both arms have to be debug builds or the comparison is
between a graded run and an ungraded one.

---

# BLOCKING: point defence regressed, and the perf win is NOT yet shown

## The regression

`probe run stress_point_defense --correctness-only`, same host, same flags:

| | baseline `d7e46ebe` | swept |
| --- | --- | --- |
| range verdict | **OK**, 22 s | **FAIL**, 112 s |
| torpedoes shot down | 8 | **0** |
| rounds spent | 3661 | 3452 |
| hits per round | **0.0022** | 0 (0.0006 on one lighter run) |
| peak rounds in the sky | 2419 | 2421 |
| colliders | 2623 | rounds no longer counted |

The saturation invariant HOLDS - peak rounds 2421 against the documented
2,419-2,421 band, so the battery fires the same stream at the same scale. What
changed is that the stream stopped connecting. `assert_the_battery_connected`
is the range that catches it, and it is doing its job.

## Leading hypothesis: the old hit test was much more generous than a cast

Avian's `NarrowPhaseConfig::default_speculative_margin` is `Scalar::MAX` and
nova does not override it. A round was therefore a body whose contact test was,
in effect, "did the swept AABB overlap, and is the distance under an unbounded
margin" - `CollisionStart` fires for PREDICTED contacts, not just touching
ones. Against a small fast torpedo that is a far wider envelope than an exact
swept sphere of radius 0.05.

So the 0.05 collider never was the round's real intercept size. The effective
size was an accident of the speculative margin, and matching the collider
exactly - which read as the conservative choice - shrank the round by a lot.

NOT established. The alternative is registration: the sweep casts the round's
segment against collider poses from after the physics step, so a fast-closing
pair is compared across a half-step offset. `a_round_intercepts_a_closing_torpedo`
passes at 400 u/s vs 60 u/s, which argues against staleness being the whole
story, but does not rule it out at PD geometry.

## What the perf measurement actually shows: nothing yet

- `stress_bullets`: peak rounds identical at 1616 in both arms. Frame times
  overlap completely (baseline p50 1.06-1.29 ms, swept 1.11-1.25). The range is
  NOT step-bound - 764 of 900 frames run zero fixed steps - so it cannot show
  this change either way.
- `wfc_arena`: three repeats per arm, and the spread swamps the difference
  (baseline 1% low 21.8-86.0 fps, swept 31.0-71.4). Worse, the arms ran
  different lengths (4969 vs 6504 frames), so they did not measure the same
  scene - the exact confound the regime instrument exists to remove.
- `stress_point_defense`: the one step-bound scene, and the swept arm is
  SLOWER on it (112 s against 22 s). That is very likely a consequence of the
  regression rather than of the sweep: torpedoes that are never shot down stay
  in the scene, so the arm that misses measures a heavier world.

**A correction worth recording, because it was stated before it was checked.**
An earlier reading here compared a hand-built `pd_before` binary that took
14 minutes against a swept one that took 20 seconds, and read it as the cost of
round bodies. It was not: that binary never ran a single autopilot step at any
scale, including 1 mount and 1 bay. It was inert, not slow. The probe runs
above replaced it.

## The decision this needs

The round's intercept size is now an explicit number rather than an accident,
and something has to choose it. That is a gameplay call, not a refactor.

---

# FOUND IT: the sweep tests a moving round against a target frozen at one instant

The envelope hypothesis above is WRONG. The evidence that killed it is the shape
of the response to cast radius on `stress_point_defense`:

| cast radius | torpedoes shot down | hits/round |
| --- | --- | --- |
| 0.05 (shipped) | 0 | 0 |
| 0.25 | 0 | 0 |
| 0.5 | 0 | 0 |
| 0.75 | 1 | 0.0003 |
| 1.0 | **47** | 0.0071 |
| baseline, rounds as bodies | 8 | 0.0022 |

A generosity problem gives a gradual curve. This is a CLIFF between 0.75 and
1.0, which is the signature of a SYSTEMATIC offset, not a spread.

The offset is one fixed step of the TARGET's own motion:

- the fast shipped torpedo caps at 70 u/s (`base_content/sections/ordnance.rs`)
- 70 / 64 Hz = **1.094 u per fixed step**
- which is where the cliff is

`advance_rounds` runs after `PhysicsSystems::Last`, so `SpatialQuery` reads each
collider at its END-of-step pose, while the round's segment spans the whole
step. A target moving 1.09 u per step is therefore displaced by more than its
own section is wide, and every round misses it systematically. Moving the system
BEFORE the step does not fix this - it flips the sign of the same error.

The rigid-body path never had the problem: avian's broad phase sweeps BOTH
bodies and the narrow phase does a two-body continuous test. Replacing it with a
one-body-against-static cast silently dropped the target's half of the motion.

## Every observation the owner made falls out of this

- **PDC carves asteroids fine.** A rock is STATIC - zero offset.
- **The arena kills torpedoes fine.** Ships and rocks are many units across, so
  a ~1 u offset still lands inside them.
- **The point-defence range gets zero.** Torpedo sections are ~1 u, the offset
  exceeds the target, and nothing connects.
- **"the turret was lagging behind the rocket".** That is exactly what testing
  against an offset target position looks like.

Radius 1.0 "working" is a coincidence, not a fix: it does not restore the
baseline, it triples it (47 kills against 8), because a 1 u ball also collects
every near miss.

## The fix: cast in the TARGET's rest frame

The correct test for a moving round against a moving target is the round's
segment against the target's SWEPT volume - equivalently, cast the round with
the RELATIVE velocity `v_round - v_target` against the target's pose.

Velocity differs per target, so it is two stages:

1. **Candidates**: one cast with the radius inflated by the largest relative
   displacement a step can produce. Cheap, and the common case (nothing near)
   ends here.
2. **Exact**: for each candidate, read its `LinearVelocity` and re-test in its
   rest frame - origin `r0 + v_target * dt`, direction `(v_round - v_target)`,
   distance `|v_round - v_target| * dt`, filtered to that one entity.

Two casts for a near miss, one for empty space. This restores the two-body
continuous test the physics path was doing, without restoring the bodies.

Do NOT ship a tuned radius. It buys the count back on this one range by making
the round a metre wide, and it would change every other weapon interaction.

---

# The rest-frame fix is right but NOT sufficient. The range is still red.

`rest_frame_impact` now resolves each candidate in the target's rest frame,
closing at `round_velocity - target_velocity`. It is pinned by
`a_round_intercepts_a_crossing_torpedo`, which is the geometry that matters and
the one the earlier head-on test could not see: a closer's per-step displacement
lies ALONG the round's path, where being a step out changes only WHEN it hits;
across the path the same displacement is pure miss distance. Removing the fix
fails that test in 0.04 s.

Empirically the collider poses a spatial query reads are the ones from the START
of the step, not after it - the crossing test passes with no origin shift and
fails with one. The doc says so and names the test as the guard.

## What it bought, and what it did not

Raw hits on anything, same range, same scale: 176 -> 251. Torpedoes shot down:
still 0-2 against the baseline's 8, and `hits_per_round` 0.0006 against 0.0022.

Sweeping the round radius WITH the fix in place: 0.05, 0.15 and 0.3 all still
give zero intercepts under the probe. So the remaining gap is not a radius away.

## Two things still unexplained

1. **Probe runs and direct runs disagree.** The same binary, same scale: a
   direct run reaches the end with 2 intercepts, every probe run panics with 0.
   The probe adds the profile sandbox and the timeline/invariants capabilities
   and nothing else obvious. Until that is understood, no PD number from either
   path can be trusted, because one of them is measuring something else.
2. **The baseline's generosity is real and unaccounted.** Avian's
   `default_speculative_margin` is `Scalar::MAX`, so a round-as-body raised
   `CollisionStart` for PREDICTED contacts at positive separation. Part of the
   remaining 3.7x is PD having been stronger than the geometry says it should
   be. How much is the open question, and it cannot be answered while (1)
   stands.

## Status

NOT mergeable. `stress_point_defense` is red on this branch and green on master.
The perf comparison stays parked: the swept arm is still measuring a scene full
of torpedoes it failed to kill, so any frame-time number off it flatters or
punishes the wrong thing.

## Correction: there is no probe-vs-direct discrepancy

The note above claimed probe runs and direct runs disagreed. They do not. A
plain direct run fails too; three consecutive direct runs then scored 2, 2, 2.
What looked like an environment difference is a small count flickering across
the range's only hard gate, `intercepts > 0`.

The rate is stable and is the real signal:

| | torpedoes down | hits/round |
| --- | --- | --- |
| baseline, rounds as bodies | 8 | 0.0022 |
| swept, rest-frame resolve | 2 | 0.0005-0.0006 |

A consistent **3.7x deficit**, not noise. At ~3500 rounds and 0.0006 the
expected kill count is ~2, so a run landing on 0 is ordinary Poisson luck - the
range passes or fails on a coin toss while the underlying rate is unchanged.

The owner's read from watching it: the rounds "barely miss", arriving about one
frame late. One step of TORPEDO motion at ~30 u/s is 0.47 u, which is the right
order for a graze. Three reviews are out on where that step comes from.

---

# Three reviews: the trail is real, pre-existing, and NOT the 3.7x

## The off-by-one the owner saw

`update_turret_aim_point` solves the intercept for a round launched NOW.
`shoot_spawn_projectile` launches it `lead` seconds into the tick - uniform over
(0, dt] for any fire interval longer than a tick - and compensates its POSITION
so the stream stays evenly spaced, but not the aim it flies along. The round
therefore crosses the intercept `lead` late and a CROSSING target has moved on:

- 0 to 0.47 u at a 30 u/s torpedo, mean 0.23 u
- 0 to 1.09 u at 70 u/s
- ALWAYS behind, never ahead

Which is exactly "barely misses, arrives a frame late". Pre-existing, not caused
by the conversion. Fixed by advancing the target half a step (the expected
launch delay) before solving, which removes the systematic part and leaves the
+/- half-step jitter.

Measured effect on `stress_point_defense`: **none** - 0.0006 hits/round before
and after. Right fix, wrong size: it removes 0.23 u of a gap that is ~1.63 u
wide.

## What actually accounts for the 3.7x, with the number

Avian's `default_speculative_margin` is `Scalar::MAX`, which does not mean an
infinite margin - it means the relative velocity is never clamped, so the
effective margin is `dt * |v_round - v_target|`. A manifold point survives at
POSITIVE separation below that, and `touching` is just a non-empty manifold, so
`CollisionStart` fired on near misses. At point-defence closing speeds that is
**1.63 u of isotropic slop** around every target (2.03 u head-on).

Cross-path acceptance area: ~2.5 u^2 for an axis-aligned round, up to ~6.5 u^2
diagonally, against 1.21 u^2 for the exact 0.05 sphere sweep. That 2x-5x bracket
contains the measured 3.7x, and it puts the baseline's 8 kills exactly between
the exact sweep's 2 and the 1.0-radius sweep's 47.

**The 0.05 collider never was the round's intercept size.** Matching it exactly
read as the conservative choice and was in fact a large nerf. The fire gate
compounds it: `TURRET_ON_TARGET_RAD` permits a round to pass 2.9 u wide at the
PDC's 180 u range gate, a tolerance sized for a HULL, while a torpedo section is
about 1 u.

## Refuted, and it was stated too confidently before being tested

The round does NOT read an eased `Transform` as its simulation state.
`complete_translation_easing` runs in `FixedFirst` BEFORE the reset and restores
`Transform` to the raw sweep pose, and the teleport guard stays quiet because
`advance_rounds` leaves `Transform == end` exactly. Verified by adding
`TransformInterpolation` to the test fixture: all 15 tests still pass. Both
properties are load-bearing and undocumented on the sweep side.

## Fixed here

- The module doc claimed the sweep runs late because "every ship had already
  moved". Wrong, and it contradicted `rest_frame_impact` four hundred lines
  below. The start-of-step sampling is a property of avian's CHILD-collider
  sync, not of `SpatialQuery`, and the doc now says so.
- `spawn_plate` put `RigidBody` and `Collider` on ONE entity - a convention nova
  never uses. Every pierce test rode it, and the moving-plate test carried a
  0.78 u error that passed only because it lay along the line of flight. The
  fixture now builds a child collider like the game does.
- The aim's launch-delay correction above.

## Still open, and now the only thing between this and green

The intercept size is a DESIGN number and has to be chosen deliberately. Left
as-is, point defence is ~3.7x weaker than it shipped. Nothing here should be
tuned to reproduce a speculative-margin accident without that being a decision.

Known defects left unfixed, all found by review, none of them the 3.7x:

- angular velocity is ignored in `target_velocity` and in `reach`, so a section
  on a rotating hull has a blind spot in both stages;
- after the first hit in a step, the sampled pose is stale by the elapsed time,
  so second and later pierce layers are tested as if the round were leading;
- `BITE_MEMORY` bounds pierces and rejects together, so a round crossing a
  swarm can spend its budget on near misses and lose a real hit.

---

# ROOT CAUSE: the fire gate is sized for a HULL, and point defence shoots torpedoes

Measured, not reasoned. Instrumenting every near miss in the range and logging
the perpendicular distance from the round's relative path to the section it
nearly hit, over 23 224 samples:

| | perpendicular miss, world units |
| --- | --- |
| minimum | **0.962** |
| p10 | 1.624 |
| median | 2.789 |
| p75 | 3.331 |
| max | 5.156 |

**No round ever passes closer than 0.96 u to a torpedo section.** That is a hard
floor, not a tail. A torpedo section is about 1 u across, so nothing is ever
close enough to hit, and no cast geometry can recover a hit that is not there.

The closest-approach time is also 0.04-0.055 s while the step under test is
0.0156 s: when a round is nearest its torpedo it is still two to three steps
away.

## Where 0.96 and 2.79 come from

`TURRET_ON_TARGET_RAD = HULL_HIT_RADIUS / CLOSE_ENGAGEMENT_RANGE = 1.6 / 100`,
0.92 deg (`turret_section/aim.rs:19-47`). Its own doc says what it is derived
from: "the widest error that still puts the round on the THING being aimed at",
where the thing is a HULL of 1.6 u radius, graded at the close edge of a
gunfight because that is the loosest angle that still lands.

Cross-track spread that permits, at range R, is `R * 0.016`:

- 60 u -> 0.96 u (the measured floor)
- 150 u, the point-defence envelope -> 2.4 u (the measured median, 2.79)
- 180 u, the PDC fire gate -> 2.9 u

The distribution IS the cone. The gate lets a mount shoot when its barrel is
within a hull's width of the aim point, and then it is fired at something a
third of that size.

## So there is no off-by-one left

The trail the owner saw was real and is fixed (the launch-delay correction). It
was 0.23 u of a 1-3 u problem. What remains is that a PDC firing on a torpedo
is allowed to shoot while pointing up to 2.4 u wide of it, and the round then
misses by exactly that.

The body path did not aim any better. It accepted contacts at positive
separation, so a round passing a unit wide still registered. That is the whole
3.7x, and it is the accuracy the game has always visibly had - a battery that
looked like it was tracking, firing, and connecting was tracking, firing, and
being credited with near misses.

## The fix is the gate, and it is a gameplay decision

`TURRET_ON_TARGET_RAD` is a constant derived from ONE target size. It should be
derived from the target actually being engaged: `target_hit_radius / range`. A
point-defence mount on a torpedo at 150 u needs about 0.19 deg, not 0.92.

That is not a refactor. Tightening the gate cuts trigger duty, which cuts rounds
fired, which changes the very load `stress_point_defense` exists to measure -
and it changes how strong point defence feels. It needs an owner's call.

Do NOT paper over it with an intercept radius. A 1-3 u round would restore the
kill count by making every near miss a hit, which is what the old physics did
and is why nobody noticed the guns were spraying.

---

# Landed. What the measurement actually showed, once it was measured properly.

## The methodology error that produced four wrong diagnoses

`probe run --correctness-only` leaves `NOVA_PROBE` unset, so the range holds for
120 FRAMES. At ~650 headless fps that is a fraction of a second of engagement
after a 6.5 s fill. Repeated runs of ONE unchanged binary in that window span 10
to 24 kills.

Every point-defence number in the sections above came out of that window: the
"3.7x deficit", the "~1245 rounds per intercept" comparison, the "no round
passes closer than 0.96 u" miss distribution, and the fire-gate story built on
it. All of it was noise, and the tell was there the whole time - a range whose
only hard gate is `intercepts > 0` was passing or failing on a coin toss while
the rate underneath never moved.

Pinning the frame rate with the range's own instrument
(`NOVA_STRESS_PD_FRAME_MS=16` + `NOVA_PROBE=1`, ~33 s engaged at 62 fps), n=3:

| | kills | rounds | hits/round |
| --- | --- | --- | --- |
| bodies | 271 | 30 735 | 0.00883 |
| swept | 208 | 28 327 | 0.00737 |

**17% fewer hits per round, not 3.7x.** The size you would expect from removing
genuine collision generosity, and part of it is the 0.1 deg spread this branch
also adds.

## The perf result the epic was written for

Per-step physics over the saturated hold, ~2320 steps per arm:

| | bodies | swept | delta |
| --- | --- | --- | --- |
| wall, median | 5.81 ms | 4.61 ms | -21% |
| broad phase | 0.259 | 0.095 | -64% |
| narrow phase | 0.318 | 0.128 | -60% |
| solver | 0.970 | 0.924 | -5% |
| **contacts** | **1945** | **190** | **-90%** |
| **constraints** | **38** | **38** | **0%** |
| dynamic bodies | 2221 | 340 | -85% |
| p95 / p99 / max | 11.5 / 14.4 / **17.4** | 8.0 / 10.3 / **11.8** | |

D20's hypothesis, confirmed exactly: contacts fell 90% while real solver
constraints did not move at all. Those 1945 "contacts" were AABB overlap pairs
from speculative margins on fast bullets - broad and narrow phase work that
never reached the solver.

The worst step is the result that matters. 17.4 ms exceeded the 15.625 ms tick,
which is where the amplifier bites and a slow step starts owing steps to the
next frame. 11.8 ms does not.

Confidence: the swept median across two runs was 4.61 and 5.26 ms, so call the
wall-time saving 10-20% rather than a precise 21%. The structural counts are not
noisy and are the stronger evidence.

## Left open, deliberately

- **Angular velocity** is ignored in `target_velocity` and in `reach`, so a
  section on a rotating hull has a blind spot in both cast stages.
- **The derived fire gate** (`target_hit_radius / range` instead of a hardcoded
  hull radius). Discussed and deferred: the owner is content with 0.92 deg as
  fire-control uncertainty rather than turret error.
- **Track smoothing measured NEUTRAL** at every time constant from 0.08 to
  1.5 s. It is correct engineering - leading on raw per-sample velocity is
  wrong, and the target-switch reset is a real rule - but it buys nothing
  measurable here and is complexity carried on that basis.
- **No tunnelling range.** The DoD asked for one if it was cheap. It was not.
