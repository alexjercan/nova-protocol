# Notes

## What was wrong

`shoot_spawn_projectile` gated firing on the safety, the section being active,
ammunition and the trigger bool. Nothing in it looked at where the barrel
pointed. A turret therefore fired the instant the trigger was held, at whatever
bearing the hinges happened to be sitting at - mid-slew, or parked against a
depression stop with the target under the hull on the far side of the ship.

The AI had half a rule of its own (`AI_FIRE_ALIGNMENT`, a 0.95 dot product =
18.2 degrees) which only decided whether to LATCH the trigger. The player path
had nothing at all: `on_turret_input` latches the bool on the key press.

## The rule

One predicate, `muzzle_on_target(forward, muzzle, aim)`, applied in
`shoot_spawn_projectile` per MUZZLE, on the raw physics pose, against the LEAD
aim point the turret actually steers to.

Per muzzle is the load-bearing part: the gate is asked once per barrel, so a
mount that cannot bear holds while its siblings on the same hull keep shooting.

It needs no second reachability test. Hinges that cannot swing onto the target
never converge, so the muzzle is never inside the cone, so it never fires -
`TurretSectionArc` keeps its existing ACQUIRE-time job (never assign a mount a
target it cannot reach) and this answers the fire-time one.

`AI_FIRE_ALIGNMENT` is gone. `on_projectile_input` now calls the same predicate,
so the AI trigger keeps meaning "this mount is shooting" for the readers of
`TurretSectionInput` without being a second, looser number.

## The tolerance, and where it comes from

`TURRET_ON_TARGET_RAD = HULL_HIT_RADIUS / CLOSE_ENGAGEMENT_RANGE = 1.6 / 100 =
0.016 rad = 0.92 deg.`

A round leaving the muzzle `e` off the aim point misses that point laterally by
`range * sin(e)`, so the widest error that still puts the round on the thing is
`asin(hit_radius / range)`. At this size the angle and its sine agree to 1e-6
rad, so the ratio IS the angle.

- `hit_radius` = 1.6 u: half the beam of a shipped corvette. The cargoa's pods
  span x -1.6..1.6; the hull is 4.9 u long by 3.2 u across by 1.6 u deep.
- `range` = 100 u: the CLOSE edge of a gunfight. Nova's scale is 100 u = 1 km
  and fights are fought at 1-2 km, so this is where the cone is widest and the
  gate loosest. That choice IS the slack the owner asked for.

Both ends of the band were checked, not just the one the constant takes:

| graded at | cone | miss allowed |
|---|---|---|
| 100 u (1 km, close edge) | 0.92 deg | 1.6 u |
| 180 u (the PDC fire gate) | 0.51 deg | 2.9 u at the full 0.92 deg |
| 200 u (2 km, round reach) | 0.46 deg | 3.2 u at the full 0.92 deg |

So a round fired at the full tolerance from the far end of the band still passes
inside a corvette's own beam. Tighter than 0.92 deg buys nothing a round can
use; looser lets the bug back in.

## Measurements

### Live: the turret range, same script, gate on vs gate off

`NOVA_AUTOPILOT=1 cargo run --example turret_section --features dev` under
Xvfb, twice: once as shipped, once with `TURRET_ON_TARGET_RAD` temporarily
raised to PI (which disables the gate without touching any other code path).
The range holds the trigger down while a gate asteroid sweeps across the front,
and prints the live aim error and the rounds in flight every 0.5 s.

Rounds in flight while the trigger was held, by sample:

| | samples | mean rounds in flight |
|---|---|---|
| gate OFF (baseline) | 17 | 41.6 |
| gate ON | 20 | 10.7 |

Rounds in flight is proportional to rounds FIRED - same lifetime, same
geometry, same script - so the gate cuts rounds spent by **74%**.

In absolute terms: the shipped PDC fires 100 rounds/s out of a 500-round
magazine, so a 5 s trigger hold used to empty the whole magazine. The same hold
now costs ~130 rounds, and a magazine lasts about four times as long.

What the baseline was spending them on is in its own log. It fired at 1.3, 1.5,
1.6, 1.8 and 2.4 degrees off the aim point continuously, and put 10 and 24
rounds in the air at 3.6 and 4.4 degrees off - all of them outside what could
land on a hull. With the gate the same samples read 0 rounds in flight above
about 1.0 deg, and 20-71 below it.

Both runs completed the script, and BOTH rounds of the range's own invariants
held with the gate on: rounds left the barrel, one connected with a range
target, and the barrel tracked the mover - twice, the second time through a
scenario reload. So the ship does not go quiet.

### Live: the player path

`NOVA_AUTOPILOT=1 cargo run --example player_path --features dev` - the real
input pipeline (raise, radar-lock, hold LMB) on a flying, maneuvering ship -
still guns its prey down, and the SCENARIO's own handlers see the kill:
"round 1 - prey destroyed, waypoint locked, GOTO closing", and again on round 2
through a scenario reload.

### The blocked mount

Pinned as a test rather than a run, because the shipped hulls carry two mounts
bolted the same way up and cannot show one bearing while the other cannot:
`a_mount_that_cannot_bear_holds_fire_while_its_siblings_shoot` puts two turrets
on one ship, one on the aim point and one pinned 40 deg off it, holds both
triggers for ten ticks, and reads the rounds back per muzzle. The bearing mount
fires; the pinned one fires nothing.

### Framerate caveat, found on the way

The gate is sensitive to framerate, because the aim solver's damping is applied
once per FRAME rather than per second (`AIM_CORRECTION_GAIN`). Steady-state
tracking lag is `rate * dt * (1 - g) / g` = `rate * dt * 1.857`. The range's
gate sweeps at up to 13.75 deg/s as seen from the muzzle, which is ~1.8 deg of
lag on the 14 fps llvmpipe harness box and ~0.43 deg at 60 fps. The measured
run matches that exactly (0.4-0.9 deg through the slow part of the sweep,
1.5-2.0 deg at its fastest). So the holding visible in the numbers above is
partly a harness artifact: a player at 60 fps tracks well inside the cone and
fires continuously. The framerate dependency predates this change; the gate is
only what makes it legible.

### The one place nothing changed: point defence

The PD cost model (`point_defense_cost_tests`, which runs the production lead
solve and the production predicate) reports the same round counts with the gate
on and off - 116 rounds for a straight torpedo, 390 for a weaving one. A mount
tracking an inbound torpedo is already on target, so it was not wasting rounds
and the gate takes none away. The savings are all on the ship-shooting side.

## What was NOT done

- No arc check in the fire path. See above: it would be a second answer to a
  question the one predicate already answers, and the two could drift apart.
- No new tolerance knob in `TurretSectionConfig`. The number is derived from
  hull dimensions and engagement ranges that live outside any one turret; a
  per-turret override would be a place for it to be wrong.
