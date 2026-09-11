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
