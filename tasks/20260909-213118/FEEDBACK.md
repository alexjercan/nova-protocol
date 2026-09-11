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

Lock range enters the table with the signature item: before it, every ship root
was lockable out to the observer's whole cap, so the column was the cap and not
a reading about the hull.
