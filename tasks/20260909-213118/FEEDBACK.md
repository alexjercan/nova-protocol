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
