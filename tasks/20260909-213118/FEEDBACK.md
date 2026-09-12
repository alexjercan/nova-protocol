# Gameplay feedback ledger

One row per observation. The contract, the sources and the disposition rules
are in `TASK.md`; this file is the record.

A balance row names the number, the hull and the measured before/after figure,
because a balance item is measured before it is tuned.

| date | who | where (scenario, hull) | what was seen | kind | disposition |
| --- | --- | --- | --- | --- | --- |
| 2026-09-09 | owner | HUD, block_carrier | the velocity and gravity spheres are authored at 50 m and 56 m and sit buried inside a 360 m hull | bug | task `20260909-212917`, fixed; `system_hud_shell` holds it |
| 2026-09-05 | review `20260905-231735` group I | gamepad, any hull | L2 raises weapons AND fires the torpedo tubes | bug | folded into task `20260714-001140` |
| 2026-09-08 | review `20260908-004345` | `web/src/wiki/flight-autopilot.md` | the page calls 55.2 m "its own 55.2 m hull" for a hull 85 m long | clarity | wiki fix in the docs lane |
| 2026-09-11 | sweep `20260909-213708` | targeting, block_skiff and block_carrier | cover broke the weapons lock but left the nav designation standing, and the player and the AI answered "can I see it" with two separate passes | balance | one `SensorContacts` pass per observing ship; both slots now drop on cover; hull inputs unchanged (see below); `system_lock_line_of_sight` holds it |
| 2026-09-11 | sweep `20260909-213708` | targeting, block_skiff and block_carrier | a combat lock let go on its own after thirty idle seconds, so a long quiet approach arrived unlocked and the safety went back on | balance | the idle decay and the reticle wind-down are removed; only the world or the player's tap takes a lock; hull inputs unchanged (see below); `system_lock_line_of_sight` holds it |
| 2026-09-11 | sweep `20260909-213708` | targeting, block_skiff and block_carrier | every ship root locked out to the player's whole 200 km sensor cap, so the skiff was designatable from exactly as far as the carrier and a beaten hull stayed as loud as a whole one | balance | every class derives a `LockSignature`; ships from live hull and machinery, rocks and worlds from their surface, torpedoes from their speed. Skiff 722 m / 21.7 km and carrier 1985 m / 59.5 km, from 200 km each before. `system_hull_scaling` holds it |
| 2026-09-11 | sweep `20260909-213708` | gunnery, block_skiff and block_carrier | every gun and every AI lance was graded on one fixed cone, so a mount held fire on a carrier it could not miss and spent rounds on a torpedo it could not hit | balance | a `TargetHitRadius` is published per body and the gates are `atan(hit radius / distance)`. Permitted miss at 1 km goes from 16 m for everything to 194 m on the carrier, 48 m on the skiff and 12 m on a Serpent; `system_turret_gunnery` holds it |
| 2026-09-11 | sweep `20260909-213708` | AI flight, block_skiff and block_carrier | an engaging ship wrote one throttle scalar to EVERY live thruster and steered with a rotation command of its own, so a hull with retros and laterals burned them against its own mains and turned its nose off the target to fly | balance | combat asks the flight computer for a held velocity and a facing (the generic `MatchVelocity` action); the computer clusters, balances and spools as it does for the player. Hull inputs unchanged (see below); the unit and physics pair in `ai/maneuver.rs` holds it |
| 2026-09-11 | sweep `20260909-213708` | weapons, any hull | a six-layer ceiling sat under the pierce power budget, so a gun round stopped six sections in whatever the plating cost, while the lance alone was bounded by power | balance | the ceiling is removed and power alone bounds every pierce round; a layer costing zero power stops it. Twenty 1 hp panels used to take 6 rounds' worth of travel and now take 20 of 300 power; `system_railgun_lance` holds the budget rule |
| 2026-09-12 | sweep `20260909-213708` | destruction, block_skiff and block_carrier | a severed wreck left at one flat 10 m/s, so a carrier's two halves ground against each other for the better part of twenty seconds | balance | a fragment leaves at its own containment radius over two seconds, floored at 10 m/s: 12.1 - 24.2 m/s on the skiff and 48.6 - 97.2 m/s on the carrier. `system_section_severing` holds it |
| 2026-09-12 | sweep `20260909-213708` | destruction, block_carrier | every wreck piece was given a flat kick and a flat 0.5 s of grace, so all 720 pieces of a collapse went rigid still standing inside the hull | balance | a piece is thrown out of what buried it - the structure over it, its own reach and 10 m of daylight - with kick and window scaled by the root of that: 0 of 720 now go rigid inside. `stress_hull_collapse` holds it |
| 2026-09-12 | sweep `20260909-213708` | destruction, block_skiff and block_carrier | every hull died at one authored size, so the carrier's fireball stopped 58 m inside its own wreck and the skiff was swallowed by a burst three times its size | balance | the hulk pyre is scaled by the dying root's `IntegrityEnvelope` over the 55.2 m gunship it was cut on, lengths linearly and lumens by the square: 0.87x on the skiff and 3.52x on the carrier, from 1.00x each. `system_hull_scaling` holds it |
| 2026-09-12 | sweep `20260909-213708` | destruction, block_carrier | a collapse lit six fires per frame and took them in arrival order, so a 720-cell corridor burned in six touching cells at the entry wound | balance | the frame's deaths are queued and the chain is cut from the batch - `6 * sqrt(condemned / 53)`, 6 to 48 - and spread by farthest-point sampling: 23 fires over the whole 150 m corridor, from 6 over 20 m. `stress_hull_collapse` holds it |

## Measured figures

Balance rows above cite this table. Every figure is measured live on the two
reference hulls of task `20260909-213708` with the `system_hull_scaling`
range, so a "before" number is a reading rather than a recollection.

### Hull inputs, 2026-09-11

Read from `system_hull_scaling` at `93849e0` (the sweep's baseline).

| figure | block_skiff | block_carrier |
| --- | --- | --- |
| live sections | 21 | 2081 |
| structural arm (`HullRadius`) | 47.8 m | 194.2 m |
| containment radius (`HullEnvelopeRadius`) | 48.3 m | 194.3 m |
| mass | 25 kg | 2360 kg |
| largest principal inertia | 1.131e2 | 2.195e5 |
| flight computers | 1 | 10 |
| summed computer torque | 1501 | 15010 |
| thruster sections | 2 | 4 |
| weapon sections | 0 | 0 |
| torque ceiling | 13.2772 rad/s2 | 0.0684 rad/s2 |
| structural ceiling | 1.6430 rad/s2 | 0.4042 rad/s2 |
| which binds | structure | **torque**, 5.9x inside it |

### After each behavior item, 2026-09-11

Re-read from `system_hull_scaling` after every behavior commit, skiff first and
carrier second in each cell. A figure that moves says the item touched mass,
arm or torque; a lifecycle item should read as unchanged.

| after | structural arm | torque ceiling, rad/s2 | structural ceiling, rad/s2 | which binds | lock range |
| --- | --- | --- | --- | --- | --- |
| one sensor pass (`68c3ebe`) | 47.8 m / 194.2 m | 13.2772 / 0.0684 | 1.6430 / 0.4042 | structure / torque | 200 km / 200 km |
| no idle lock decay (`5a809e5`) | 47.8 m / 194.2 m | 13.2772 / 0.0684 | 1.6430 / 0.4042 | structure / torque | 200 km / 200 km |
| derived signatures | 47.8 m / 194.2 m | 13.2772 / 0.0684 | 1.6430 / 0.4042 | structure / torque | 21.7 km / 59.5 km |
| angular hit radii | 47.8 m / 194.2 m | 13.2772 / 0.0684 | 1.6430 / 0.4042 | structure / torque | 21.7 km / 59.5 km |
| pierce power alone | 47.8 m / 194.2 m | 13.2772 / 0.0684 | 1.6430 / 0.4042 | structure / torque | 21.7 km / 59.5 km |
| launcher-safe arming | 47.8 m / 194.2 m | 13.2772 / 0.0684 | 1.6430 / 0.4042 | structure / torque | 21.7 km / 59.5 km |
| fractional thrust taper | 47.8 m / 194.2 m | 13.2772 / 0.0684 | 1.6430 / 0.4042 | structure / torque | 21.7 km / 59.5 km |
| face-gap launch floor | 47.8 m / 194.2 m | 13.2772 / 0.0684 | 1.6430 / 0.4042 | structure / torque | 21.7 km / 59.5 km |
| combat on the computer | 47.8 m / 194.2 m | 13.2772 / 0.0684 | 1.6430 / 0.4042 | structure / torque | 21.7 km / 59.5 km |

Lock range enters the table with the signature item: before it, every ship root
was lockable out to the observer's whole cap, so the column was the cap and not
a reading about the hull.

### Fire gates, 2026-09-11

The gunnery item changes no hull input, so its row above reads unchanged. What
it moves is the cone a barrel is graded on, which is a reading about the
TARGET. The gate is `atan(hit radius / distance)`; before it, every gun and
every AI lance used one fixed 0.92 deg cone.

| target | hit radius | gate at 1 km, before | after | live reading |
| --- | --- | --- | --- | --- |
| block_carrier | 194.2 m (`HullRadius`) | 0.92 deg | 11.0 deg | `system_hull_scaling` |
| block_skiff | 47.8 m (`HullRadius`) | 0.92 deg | 2.7 deg | `system_hull_scaling` |
| Serpent torpedo | 12.2 m (envelope) | 0.92 deg | 0.70 deg | 0.838 deg at 837 m, `system_borrowed_battery` |
| gunnery gate rock | 99.4 m (`BodyRadius`) | 0.92 deg | 5.7 deg | 6.82 deg at 831 m, `system_turret_gunnery` |

A commanded point keeps the fixed 0.92 deg: a mark the player invented off the
camera ray has no size to measure.

### Launch safety, 2026-09-11

The arming item changes no hull input either. What it moves is how far a
torpedo has to be from the ship that fired it before the warhead goes live.
The authored fuze is 50 m from the muzzle, which is a number about the
ORDNANCE; the new condition is `launch HullRadius + blast radius`, which is a
number about the HULL, and both must hold.

| launching hull | arm | blast radius | arms no nearer than, before | after |
| --- | --- | --- | --- | --- |
| torpedo range boat | 24.4 m | 300 m | 50 m | 324 m |
| block_skiff | 47.8 m | 300 m | 50 m | 348 m |
| block_carrier | 194.2 m | 300 m | 50 m | 494 m |

The boat row is read live off `system_torpedo_launch` (`range: torpedo fired
... arms no nearer than 324 m`, and the gate round measured a tightest arming
separation of 325.4 m against a 324.4 m clearance). The two block rows apply
the same formula to the arms measured by `system_hull_scaling` above.

One range moved with it. `system_torpedo_launch` staged its near gate at
300 m, which is inside the safety distance, and every torpedo homes on the
nearest gate - so the whole gate round went to a target the ordnance cannot
engage and died there as duds. The near gate now stands at 450 m, which is the
shortest shot a Serpent can actually take. That is the balance consequence of
the item, stated where it is visible: a torpedo bay has a minimum range, and
it is roughly its own blast radius.

### Thrust taper, 2026-09-11

The taper band a torpedo eases thrust off over was a flat 5 u/s under the
authored cruise, so its WIDTH was the same for every type and its SHARE was
not. It is now 15 percent of the type's own cruise.

| type | cruise | band before | share before | band after | share after |
| --- | --- | --- | --- | --- | --- |
| Lance | 350 m/s | 50 m/s | 14.3 % | 52.5 m/s | 15 % |
| Serpent | 320 m/s | 50 m/s | 15.6 % | 48.0 m/s | 15 % |
| Breaker | 700 m/s | 50 m/s | 7.1 % | 105.0 m/s | 15 % |
| a 60 m/s loiterer | 60 m/s | 50 m/s | 83.3 % | 9.0 m/s | 15 % |

Nothing shipped moves by more than a percentage point, which is why the live
proof is that the shipped ordnance still makes its cap rather than a changed
figure: `system_torpedo_launch` now measures the best along-nose speed as a
share of the type's own authored cruise and holds it above 85 percent, the
width of the band. It reads 88 percent on the gate round and 90 on the
crossing round, stable across three runs. The loiterer row is the case the
change is FOR, and no shipped type is slow enough to stand in for it, so that
one is proved by unit test
(`a_slow_warhead_gets_the_same_share_of_its_envelope_as_a_fast_one`).

### AI torpedo launch floor, 2026-09-11

The AI held its ordnance until the target was three blast radii away - 900 m
for the shipped warhead - measured ANCHOR to ANCHOR. That margin is supposed
to be clear space between two ships, and anchor to anchor it is not: each hull
spends its own arm out of it first.

| pair | own arm | target arm | launches no nearer than, before | after |
| --- | --- | --- | --- | --- |
| skiff vs skiff | 47.8 m | 47.8 m | 900 m | 996 m |
| skiff vs carrier | 47.8 m | 194.2 m | 900 m | 1,142 m |
| carrier vs carrier | 194.2 m | 194.2 m | 900 m | 1,288 m |

Read the other way: at the old 900 m floor two carriers had 512 m of clear
space between their faces, not 900, and the margin the number was written for
was more than 40 percent gone. Arms are the live `system_hull_scaling`
figures.

No systems range stages an AI ship with a torpedo bay - every torpedo boat on
the ranges is `SpaceshipController::None` and scripted - so the proof is the
unit pair in `ai/torpedo.rs`, as for the AI railgun commit.

### Combat actuation, 2026-09-11

The actuation item changes no hull input either. What it moves is WHICH
engines burn and where the nose points while they do. The old combat writer
put one scalar on every live thruster and wrote its own rotation command, so
the burn was an all-or-nothing broadcast gated on a single 18 deg alignment
cone, and the hull's attitude WAS its flight direction.

| what | before | after |
| --- | --- | --- |
| engines lit by a burn | every live thruster | the cluster the burn needs |
| opposed engines lit together | yes - retro and laterals with the mains | no |
| off-centre engine torque | whatever the broadcast left | nulled by the wrench allocation |
| throttle shape | 1.0 or 0.0, on an alignment gate | spooled, demand-capped by cluster authority |
| nose while flying the envelope | along the flight direction | on the target |
| brake regime | a separate "point opposite the velocity" heading | the velocity error itself |

No systems range staged an AI ship that fights when this landed - every AI
hull on the ranges was authored with a 10 m engage range and stayed parked -
so the proof is the physics pair in `ai/maneuver.rs`, which runs the real
`NovaFlightPlugin`. `system_ai_combat` stages one now; see below.
`the_burn_lights_one_cluster_not_every_engine` flies a four-engine hull
(mains, retro, two laterals) straight in at a target for 45 s and measures
the mains at 0.90 of full throttle with the other three engines at exactly
0.0; the broadcast writer put all four at 1.0.

One consequence is worth naming. The computer hands a sub-cap velocity error
to the RCS, so a ship tracking its standoff ring trims on the torque-free COM
push and keeps its nose on the target for most of a fight; the main drive
lights for the entries, the big corrections and the run-in. That is the
approved shape ("it holds facing while RCS can correct velocity"), and it is
what makes an enemy at its standoff read as crossing your bow with its guns
up rather than swinging its nose around to fly.

A ship the computer cannot fly - no live engine, or no live flight computer -
is never handed the maneuver at all. The autopilot would refuse it and
disengage, and the driver would engage it again the next frame: on
`bug_neutralized_quiet` that churned 386 engage/disengage pairs in a nine
second run before the gate went in, and zero after.

### AI engagement geometry, 2026-09-12

The three AI flight items left in the sweep - the standoff range, the orbit
speed and the jink leg - all move where a fight sits and how fast, so each
needs a reading of a live fight before it lands. `system_ai_combat` is that
reading. It stages two engagements far enough apart that neither scanner hears
the other: `block_picket` against a parked `block_skiff`, and `block_warship`
(the only capital combatant in the base fleet) against a parked
`block_carrier`. Every magazine on the range is empty, because a live fight is
over before the flying settles - the picket guts the skiff in nineteen seconds
and the warship's first salvo breaks the carrier into six bodies inside two -
and a dry gun changes nothing about how a hull flies.

Read sixty seconds after both movers commit, at `f3f7457`:

| figure | picket vs skiff | warship vs carrier |
| --- | --- | --- |
| mover arm (`HullRadius`) | 50.0 m | 118.0 m |
| target arm (`HullRadius`) | 47.8 m | 194.2 m |
| centre gap | 1,011 m | 1,015 m |
| face gap | 913 m | 703 m |
| speed | 80.0 m/s | 80.0 m/s |
| closing | +3 m/s | +5 m/s |
| facing asked for, off the line of sight | 0.0 deg | 0.0 deg |
| nose achieved, off the line of sight | 17.3 deg | 38.4 deg |
| largest live throttle | 0.000 | 0.000 |

Three rows there are the "before" of items still to land. The centre gap is the
same on both fights and the face gap is not: the standoff is written anchor to
anchor, so the larger the two hulls the less space is left between their skins,
210 m less here, and the fleet ships nothing bigger than this pair. The speed is
the same on both fights too, to a tenth of a metre per second, because the orbit
floor is a constant rather than a reading of what either hull can hold. And the
whole steady state is flown on RCS: no drive on either mover is lit while the
fight holds station.

The achieved nose angle is the honest cost of the facing hold. The facing is a
REQUEST, so the hull settles onto it at its own turn rate while the orbit keeps
moving the line of sight, and a capital hull lags it by tens of degrees. The
range records that figure and asserts only the request, because a snapshot of
the lag is a sample of an oscillation.

### The standoff became a face clearance, 2026-09-12

`AI_STANDOFF_RANGE` was an anchor-to-anchor distance, so it decided a
different fight for every pair of hulls: the same 1,000 m left 913 m of space
between a picket and a skiff and 703 m between a warship and a carrier, and a
modded pair with arms over 500 m each would have parked inside one another
with the constant satisfied. It is now `AI_STANDOFF_CLEARANCE`, a clearance
between the two hulls' FACES, and the preferred centre distance adds the
mover's and the target's live `HullRadius` on top. `AIControllerConfig` takes
an authored `standoff_clearance` beside `engage_range`; `Some(0 m)` is
meaningful and asks for contact.

`system_ai_combat`, sixty seconds after both movers commit:

| figure | picket vs skiff | warship vs carrier |
| --- | --- | --- |
| mover arm / target arm | 50 m / 48 m | 118 m / 194 m |
| centre gap, before | 1,011 m | 1,015 m |
| centre gap, after | 1,125 m | 1,320 m |
| face gap, before | 913 m | 703 m |
| face gap, after | 1,027 m | 1,008 m |
| speed | 79 m/s | 80 m/s |
| closing | +7 m/s | -8 m/s |

The face gap is what the two fights now agree on, to within 19 m of each other
and 27 m of the authored 1,000 m, and the centre gap is what moved apart by
the 195 m of extra arm the capital pair carries. A player watching a carrier
fight sees the same gap they see when a skiff fights.

The cost is paid in gun reach. `AI_STANDOFF_OUTER_EDGE` stays a face distance
so the content audit can keep grading authored prototypes against it, but it
is now necessary rather than sufficient: the round crosses the centre
distance. On the shipped fleet's largest pair that turns 1,250 m of band into
1,562 m of travel against the weakest gun's 1,800 m gate. A mod whose hulls
carry arms of several hundred metres has to author reach for them, and both
`guns.rs` and the constant say so.

### The fight is flown at the hull's own speed, 2026-09-12

`AI_ORBIT_SPEED` 8 u/s and `AI_MAX_CHASE_SPEED` 20 u/s were the same two
numbers for every hull in the game, so a picket and a capital circled at
exactly the same 80 m/s and both closed under one 200 m/s ceiling neither of
them had the drive to hold. The caps are gone. A new per-tick
`FlightAuthority` publishes what each live hull can still do - drive
acceleration over live mass, turn rate, and the flight computer's tracking lag
- and combat flight reads it:

- The radial term asks `arrival_speed_limit`, the SAME stopping rule the
  player's arrival legs are flown with, for the speed the hull can still stop
  from at its distance off the band, with the flip lead `flip_lead` computes
  from that hull's own turn rate and lag.
- The orbit term is the lower of the centripetal limit `sqrt(R * a * r)` and
  the attitude limit `R * w * r`, both taken at the STANDOFF radius, with
  `R = AI_ORBIT_AUTHORITY_RESERVE = 0.25`. The reserve is literal: holding
  that circle costs exactly `R * a` of continuous lateral thrust forever, so a
  quarter is what the fight may commit and three quarters stay free for
  closing, extending and jinking.

`system_ai_combat`, both movers settled:

| figure | picket vs skiff | warship vs carrier |
| --- | --- | --- |
| published drive | 39.3 m/s2 | 28.4 m/s2 |
| published turn rate | 56.8 deg/s | 37.3 deg/s |
| orbit speed, before | 80 m/s | 80 m/s |
| orbit speed, after | 91 m/s | 83 m/s |
| centripetal limit | 104 m/s | 97 m/s |
| attitude limit | 272 m/s | 214 m/s |
| peak approach speed | 163 m/s | 125 m/s |
| throttle at the settle | 0.000 | 0.000 |

The before column is one constant printed twice. The after column is two
readings of two hulls, and the shipped fleet is bound by its drive on both
fights: the attitude limit sits 2.6x and 2.2x above it. That limit is the
guard for the hull the drive test cannot catch - a long modded arm on weak
RCS, which can accelerate into a circle it cannot keep its guns on.

A per-second trace of the settle says the approach is now one curve:
accelerate to a peak at about 5 s, brake smoothly to 41 m/s at the band, then
spin up into the circle, with the nose inside 2.6 deg of the target the whole
way and no throttle at all once the circle is held. The circle is flown on
RCS, because a quarter of the drive at this radius is less than the RCS
already carries.

Two readings in that table need their caveat. The 60 s sample of the escort
fight in the range log is 91 m/s at 55 s but 72 m/s at 60 s, because the skiff
swings its nose across the picket at 56 s and the picket is four seconds into
an evade cycle when the range measures it; 91 m/s is the last clean Engage
sample. And both fights are still converging at 60 s - the face gap is
drifting in by 1 to 2 m/s and the speed up toward the centripetal limit as the
radial term hands its share to the orbit term. The settle is asymptotic by
construction, not a step.

### The fight is flown in the target's frame, 2026-09-12

The envelope and the jink both handed the flight computer an ABSOLUTE
velocity, so every speed the AI reasoned about was a speed over the void
rather than a speed over the ship it was fighting. Against a stationary
target that is the same number. Against a moving one it is the target's whole
velocity out: an AI asked to circle at 90 m/s around a ship running at 100
loses the ring at up to 190 m/s of closure while it holds exactly the
velocity it meant to hold, and the radial term spends the fight chasing an
error the orbit term keeps re-making.

`update_combat_flight` now reads the target's `LinearVelocity` and adds it
back after the envelope, so `ai_desired_velocity` and `ai_evade_direction`
both return a RELATIVE velocity and the computer is handed the sum. A target
with no rigid body is a fixed installation and reads as zero.

`system_ai_combat` is unchanged by this, to the metre and to the m/s on both
fights, because both of its targets hold station - which is exactly why the
range could not have caught the bug. The proof is
`the_whole_envelope_is_flown_in_the_targets_frame`: a target given 13 m/s of
its own must be matched AND circled, and the held velocity has to be the sum.

### A jink leg became a displacement, 2026-09-12

The evade cycle was three stopwatch legs: 1.2 s each, 3.6 s of cycle, flown at
a flat 20 u/s. On a capital that is a wallow. The hull turned 17 of the 90
degrees onto the leg before the timer moved it to the next one, and never
thrust at all, so the whole cycle was a hull rocking in place while a stopwatch
counted legs it had not flown.

A leg is now a DISPLACEMENT, and the clock is a backstop rather than the plan:

| | before | picket, live | warship, live |
| --- | --- | --- | --- |
| leg ends on | 1.2 s elapsed | 60 m carried | 128 m carried |
| leg speed | 200 m/s | 69 m/s | 85 m/s |
| liveness deadline | none | 5.9 s | 9.4 s |
| engage window after | 1.5 s | 5.9 s | 9.4 s |

The clearance is the hull's own `HullRadius` plus 10 m, so the leg carries the
whole ship off the line a gun is holding rather than its centre. The speed is
`sqrt(2 a d)` - what the drive has built at the instant the ship has crossed
`d` - so the leg ends the moment the displacement is made and nothing is held
for show. The deadline is what the leg takes at the hull's published limits: a
half turn at its turn rate, `sqrt(2) * speed` of velocity change at its
acceleration, and a 0.3 s burst. `AI_EVADE_SECS` and `AI_JINK_INTERVAL_SECS`
are gone; `AI_EVADE_LEGS` (3) is a count.

Two more things had to move with it.

A hull with no drive or no attitude left cannot fly a leg at all, so it is no
longer allowed into Evade, and one that loses its drive mid-cycle comes back
out. It used to enter and sit there for the whole cycle.

The refractory window is now one leg's own deadline, floored at the shipped
1.5 s. It had to scale: on the live picket the cycle grew from 3.6 s to about
9 s, and against a fixed 1.5 s window that is a ship weaving 86 percent of a
fight with its standoff orbit never seen - measured, not guessed, on a traced
range run before the window was derived.

`an_evading_hull_flies_its_legs_instead_of_waiting_them_out` is the proof on real
physics: a rig hull with 21.3 u/s2 and 2.52 rad/s flies all three legs on
achieved displacement in 2.80 s of a 5.94 s liveness budget. The pure tests
cover the plan itself, the authority gate covers the crippled hull.

`system_ai_combat` also had to stop being a duel. Its targets were parked with
`Quat::IDENTITY`, which points a nose down the plane the mover circles in, and
a hostile's nose on the mover inside `AI_THREAT_AIM_RANGE` is a threat whether
or not anyone is at its helm. With legs that now run for seconds, the picket
spent the run weaving across a fixed nose rather than flying the envelope the
range measures. The targets are parked nose-up, out of the orbit plane, and
both fights read the envelope again:

| figure | picket vs skiff | warship vs carrier |
| --- | --- | --- |
| face gap | 1,028 m | 1,034 m |
| speed | 92 m/s | 83 m/s |
| closing | +0 m/s | +0 m/s |
| nose off the target | 2.6 deg | 2.8 deg |
| throttle | 0.000 | 0.000 |

That escort row also replaces the 72 m/s recorded for the orbit item a commit
earlier: that sample landed four seconds into an evade cycle. 92 m/s is the
picket's orbit speed, and it is the figure the drive limit predicts.

### A patrol rounds a rock with its whole hull, 2026-09-12

`AI_AVOID_MARGIN` was 200 m past a body's `BodyRadius`, judged against the
mover's CENTRE line. A hull's own arm was therefore spent out of the margin,
and the bigger the ship the less of it was left. Measured on the patrol physics
harness against a 400 m rock sitting 605 m off the leg - just outside the old
block threshold, so neither hull turned:

| | block_skiff (4.0 u) | block_carrier (18.3 u) |
| --- | --- | --- |
| daylight before | 166 m | 23 m |
| daylight after | 611 m | 628 m |
| detour taken | no -> yes | no -> yes |

23 m is a carrier flank inside its own docking tolerance, on a route the author
never measured because the game promised to fly around it.

Clearance is now `BodyRadius + HullRadius + margin`, the same face-to-face sum
`resolved_arrival_standoff` already used one line away. The same authored number
is now the same visible gap on every hull: at the default 200 m the skiff turns
for a rock inside 640 m of its leg and the carrier inside 783 m.

With a rock dead on the leg, where both hulls already detoured, the rounding
widened rather than appeared: 366 m to 463 m of daylight on the carrier, 523 m
to 542 m on the skiff. The corner itself is unchanged geometry - it is pushed
past the clear-check band plus the corner's own arrival window - so the extra
daylight is the margin the flank used to pay.

The planner moved to `flight/navigation.rs` with an explicit `DetourPolicy`
(clearance, hysteresis, arrive radius) and NO defaults of its own: what is safe
for a skiff threading a belt is not what is safe for a carrier, so the caller
states it. AI patrol is the only caller today. `AIControllerConfig::avoid_margin`
authors it per ship, 0 m meaning "pass on my own skin".

### The flight computer is pinned to a hull that exists, 2026-09-12

`DEFAULT_MAX_TORQUE` was 1501, pinned by putting the structure/torque crossover
at a 100 m arm on the belief that "the largest hull in the game reaches only
29.3 m". `block_carrier` reaches 194.2 m. The constant's own doc said all four
reference hulls were structure-bound with 12x to 115x of headroom; the carrier
was torque-bound by 5.9x, inside the regime the constant declared unreachable.

Measured on `system_hull_scaling`, before and after:

| figure | block_skiff | block_carrier |
| --- | --- | --- |
| computers | 1 | 10 |
| structural arm | 47.8 m | 194.2 m |
| inertia | 1.131e2 | 2.195e5 |
| torque before | 1 501 | 15 010 |
| torque after | 9 760 | 97 600 |
| torque ceiling before | 13.2772 rad/s2 | 0.0684 rad/s2 |
| torque ceiling after | 86.3326 rad/s2 | 0.4446 rad/s2 |
| structural ceiling | 1.6430 rad/s2 | 0.4042 rad/s2 |
| binds before | structure | torque |
| binds after | structure | structure |
| headroom before | +708.1% | -83.1% |
| headroom after | +5154.7% | +10.0% |
| what the hull gets | 1.6430 rad/s2, unchanged | 0.0684 -> 0.4042 rad/s2 |
| bang-bang 180 | 2.77 s, unchanged | 13.55 s -> 5.58 s |

The skiff's ceiling does not move at all: it was structure-bound and it stays
structure-bound, now with 51x of headroom instead of 7x. The whole change lands
on the capital, which is what the knob was always for.

Ten percent and not more, because the margin is what makes the number visible.
A carrier that loses ONE of its ten computers drops to 0.4002 rad/s2 and is
torque-bound for the rest of the fight, so both regimes occur on shipped content
and a wrecked bridge costs real turn rate. That is now a claim a range holds:
`system_hull_scaling` asserts the intact carrier sits in a 5-20 percent band
over its structural ceiling and the skiff keeps at least 10x, so a retune that
drifts either way says so.

`attitude.rs` lost its extrapolated capital rig - inertia 7906 at a 150 m arm,
which no shipped hull resembles - for the measured carrier, and now proves the
crossover on the hull the constant is pinned to rather than on a number picked
to sit past it.

`system_ai_combat` is unchanged by this: its movers are `block_picket` (50 m
arm) and `block_warship` (118 m arm), both structure-bound before and after, so
the fight reads 1,029 m / 1,034 m of face gap at 92 m/s / 83 m/s exactly as it
did. The retune is worth nothing until a hull as big as the carrier flies one.

### A contact under 5 m/s is free, 2026-09-12

`MIN_IMPACT_SPEED_SQUARED` 0.1 was a 3.16 m/s floor over damage linear in the
pair's effective mass, dealt to ONE side of the pair by a `CollisionStart`
observer. It is now a universal 5 m/s `SAFE_CONTACT_SPEED`: above it, avian's
solved contact-pair impulse is scaled by the approach speed's EXCESS over the
safe speed, the dissipated energy is derived from that impulse, that excess and
the restitution, each section soaks one percent of the health it was BUILT with
before the energy term, and both sides of the contact pay. The system reads the
contact graph in `FixedPostUpdate` after `PhysicsSystems::Last` instead of
observing an event.

Measured on the new `system_collision_damage`, before and after, hit points lost
per side:

| pair | hull | closing | touching frames | before | after |
| --- | --- | --- | --- | --- | --- |
| carrier dock | block_carrier | 3.2 m/s | 10 | 0.00 | 0.00 |
| skiff touch | block_skiff | 4.0 m/s | 6 | 0.00 | 0.00 |
| skiff ram | block_skiff | 15.0 m/s | 8 | 0.00 | 0.75 |
| skiff hard ram | block_skiff | 30.0 m/s | 14 | 0.00 | 2.77 |

The before column is a live reading, and it is zero in every row. That is the
finding: the old path was driven by `CollisionStart`, and the two shipped hulls
staged here raised none between them, so a ram between two ships cost nothing at
ANY speed. The 49 hit points per contact the audit quotes was arithmetic from the
formula, not a figure any ship ever paid. The new path reads what the solver
settled on, so it does not depend on a collider having had collision events
enabled at the moment it was linked - and ship-to-ram-ship damage exists for the
first time. Both hulls keep the docking case free, which is what the item asked
for; what is new is that the ram case above it is no longer free either.

Twice the closing speed costs 3.7x the hit points (2.77 against 0.75 at 30 m/s
against 15). The bite is taken on the excess, so a ram at 3x the safe speed
spends two thirds of its impulse and a ram at 6x spends five sixths of a much
larger one.

The census sums every health pool under a root rather than the ship layer's
roll-up over sections, because at these speeds two skiffs meet scab to scab: the
outermost thing a shipped skin puts in a contact's way is a decor patch with a
pool of its own. A census that counted sections alone read a real ram as free.

The carrier pair meets nose to nose and holds ONE contact, not the hundreds a
frame the audit describes - two capitals coming alongside broadside would hold
many more. The claim the range makes is the one it can stage: ten frames of
capital-on-capital contact at a docking speed, and not a hit point spent.

### A sever parts at the size of what it cut, 2026-09-12

`SEVER_SEPARATION_SPEED` was a flat 1 u/s, so every fragment left at 10 m/s and
two halves parted at 20 m/s whatever they were. The raw speed is now each
fragment's OWN containment radius over `SEVER_CLEARANCE_SECS` (2 s), floored at
10 m/s, and the mass-weighted mean of those kicks still comes back off all of
them, so a cut moves the halves apart without moving the wreck.

Measured on `system_section_severing`, which now cuts a hull of 30 m components
so both sides of the cut are past the floor and the range reads the size rule
rather than the floor:

| figure | before | after |
| --- | --- | --- |
| the surviving hull's containment radius | 26.0 m | 26.0 m |
| the wreck's containment radius | 36.7 m | 36.7 m |
| the hull's separation speed | 10.0 m/s | 13.0 m/s |
| the wreck's separation speed | 10.0 m/s | 18.4 m/s |
| measured fracture speed | 20.08 m/s | 31.41 m/s |
| the two are clear of each other in | 3.12 s | 2.00 s |

The before column is a live reading: the range asserts the rule, so the old
constant fails it and names the figure it measured (2.0079606 u/s).

The two reference hulls are not cut by any range, so their rows apply the rule
to the containment radius `system_hull_scaling` measures. Where a cut falls
decides how much of that radius a half keeps, so each is a band: a half that
takes the whole length keeps all of it, a half cut off the middle about half.

| hull | containment radius | a half's speed, before | after | halves clear in, before | after |
| --- | --- | --- | --- | --- | --- |
| block_skiff | 48.3 m | 10 m/s | 12.1 - 24.2 m/s | 2.4 - 4.8 s | 2.0 s |
| block_carrier | 194.3 m | 10 m/s | 48.6 - 97.2 m/s | 9.7 - 19.4 s | 2.0 s |

A fragment under 20 m of containment radius is unchanged, because the floor is
exactly the speed the old constant gave: a cockpit shard or a single cell still
leaves at 10 m/s. Everything the floor does not bind clears in two seconds by
construction, which is the whole of the change: the carrier's halves stop
grinding against each other for the better part of twenty seconds, and nothing
smaller than the skiff moves at all.

The hull inputs read unchanged after the item, as a lifecycle change should:
arm 47.8 m / 194.2 m, torque ceiling 86.3326 / 0.4446 rad/s2, structural ceiling
1.6430 / 0.4042 rad/s2, both structure-bound, lock range 21.7 km / 59.5 km.

### A piece is thrown out of what buried it, 2026-09-12

`PIECE_KICK` was a flat 20 to 50 m/s and `CHUNK_GRACE_SECS` a flat 0.5 s, so
every piece was given 10 to 25 m of travel before it went rigid, whatever it
was standing in. A piece is now given the distance IT has to cross - the
structure over it (`IntegrityEnvelope`, published by the layer that owns the
body), plus its own reach, plus 10 m of daylight - with both the kick and the
window multiplied by the square root of that over 10 m, so the slowest piece is
exactly outside as its window runs out.

Measured on `stress_hull_collapse`, one siege slug through a block capital, 720
corridor cells shed in one flush:

| figure | before | after |
| --- | --- | --- |
| pieces that went rigid still inside the hull | 720 of 720 | 0 of 720 |
| deepest piece, buried | 97.2 m | 97.2 m |
| deepest piece, kick | 20 - 50 m/s | 124.7 m/s |
| deepest piece, grace window | 0.50 s | 1.70 s |
| deepest piece, travel before it goes rigid | 10 - 25 m | 212.3 m |
| shallowest piece, buried | 19.0 m | 19.0 m |
| shallowest piece, kick | 20 - 50 m/s | 53.0 m/s |
| shallowest piece, grace window | 0.50 s | 0.97 s |

The "720 of 720" is a live reading: the range now claims that every piece is
clear of what buried it before it goes rigid, and the old pair fails that claim
on every piece it threw, including the ones off the skin. That is the finding.
Nothing in the old figures was about the body: a plate 97 m inside a capital
and a plate on its outer face were given the same shove and the same window,
and both landed inside the wreck for the solver to push out.

The two reference hulls are read the same way, from the containment radius
`system_hull_scaling` measures. A piece at the centre is the worst case, and a
one-cell section reaches 8.7 m:

| hull | containment radius | deepest burial | kick before | kick after | window before | window after |
| --- | --- | --- | --- | --- | --- | --- |
| block_skiff | 48.3 m | 48.3 m | 20 - 50 m/s | 51.8 - 129.4 m/s | 0.50 s | 1.29 s |
| block_carrier | 194.3 m | 194.3 m | 20 - 50 m/s | 92.3 - 230.7 m/s | 0.50 s | 2.31 s |

The balance consequence is visible and is meant to be: wreckage off a capital
now leaves fast. A section from the middle of a carrier crosses its own hull in
about two seconds instead of hanging in it, which is what a hull coming apart
looks like, and what the grace window was always supposed to buy.

The hull inputs read unchanged again: arm 47.8 m / 194.2 m, torque ceiling
86.3326 / 0.4446 rad/s2, structural ceiling 1.6430 / 0.4042 rad/s2, both
structure-bound, lock range 21.7 km / 59.5 km.

### A hull burns at the size of the hull, 2026-09-12

`HULK_PYRE` is one authored look, cut against a shipped gunship of 55.2 m, and
every hull in the game died at exactly that size. The look is kept and the
dying root's `IntegrityEnvelope` now says how big to draw it: every LENGTH is
multiplied by the hull's containment radius over 55.2 m, and the flash by the
square of it, because lumens stand in for a burning surface. Durations and
particle counts are not scaled - a bigger ship does not burn for longer, and
the count is what `PYRE_FRAME_CAP` was written against.

Measured on `system_hull_scaling`, which now kills both reference hulls where
they are parked and reads the scale off each fireball:

| hull | containment radius | scale before | after | live reading |
| --- | --- | --- | --- | --- |
| block_skiff | 48.3 m | 1.00x | 0.87x | `outcome: a hull burns at the size of the hull` |
| block_carrier | 194.3 m | 1.00x | 3.52x | same claim, same run |

The "1.00x before" is a live reading too: with `hulk_scale` neutered to the old
constant the range fails on the skiff at `lit at 1.00x`, which is the finding.

What the scale moves, from the authored figures:

| figure | authored (gunship) | block_skiff, after | block_carrier, after |
| --- | --- | --- | --- |
| core peak quad | 13.0 m | 11.4 m | 45.8 m |
| ejecta reach (top speed x longest life) | 136 m | 119 m | 479 m |
| flash peak | 60 Mlm | 45.9 Mlm | 743 Mlm |
| flash range | 1 700 m | 1 488 m | 5 984 m |
| burn time | 0.65 s | 0.65 s | 0.65 s |

The number that decides whether a death reads is the reach against the hull's
own size. It was 2.82x on the skiff and 0.70x on the carrier - a burst nearly
three times the wreck on one, and one that stopped 58 m short of the wreck's
own ends on the other. It is now 2.46x on every hull by construction, which is
the gunship's own figure: the debris leaves the silhouette, and a ship that
merely broke still looks different from one that was destroyed.

The balance consequence is a look, not a capability: nothing about a death's
damage, timing, piece count or budget moved. A carrier death is much brighter,
which is the intent - it is the largest thing the game can destroy.

The hull inputs read unchanged: arm 47.8 m / 194.2 m, torque ceiling 86.3326 /
0.4446 rad/s2, structural ceiling 1.6430 / 0.4042 rad/s2, both structure-bound,
lock range 21.7 km / 59.5 km.
\n
### The chain of fires is the size of the collapse, 2026-09-12

`PYRE_FRAME_CAP` was a flat six deaths per frame, spent on the first six the
destruction pass raised. A collapse condemns its whole hull in one flush and
the pass walks the section graph, so those six were six fires in the corner the
walk started in - a puff at the entry wound whatever the wreck's size.

Compartment deaths are now QUEUED, and the frame's batch is spent when it is
whole: `clamp(ceil(6 * sqrt(condemned / 53)), 6, 48)` fires, picked by
farthest-point sampling over where the deaths happened. 53 is the gunship's own
section count, the hull the death look was cut on, so an ordinary frame lights
the six it always did. A hull root's fireball is never queued and never
dropped.

Measured on `stress_hull_collapse`, one siege slug through a block capital, 720
corridor cells shed in one flush:

| figure | before | after |
| --- | --- | --- |
| condemning frames | 1 | 1 |
| deaths in that frame | 720 | 720 |
| fires lit | 6 | 23 |
| corridor the chain walked | 20 m of 150 m | 150 m of 150 m |
| closest two fires | 10 m | 31.6 m |

10 m apart is one build-grid cell: the old six burned in six cells that touched.

The two reference hulls are read from the live section counts
`system_hull_scaling` measures, for the frame a whole hull lets go at once:

| hull | live sections | fires before | after |
| --- | --- | --- | --- |
| block_skiff | 21 | 6 | 6 |
| block_carrier | 2 081 | 6 | 38 |

The skiff is deliberately unchanged. It has fewer sections than the hull the
chain was tuned on, and a needle coming apart in six fires is the read that was
already right - the floor is there to keep it.

The ceiling is a GPU bound and not a look: every lit death allocates its own
pair of per-instance buffers in the frame it is born.

The hull inputs read unchanged: arm 47.8 m / 194.2 m, torque ceiling 86.3326 /
0.4446 rad/s2, structural ceiling 1.6430 / 0.4042 rad/s2, both structure-bound,
lock range 21.7 km / 59.5 km.
