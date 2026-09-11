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
