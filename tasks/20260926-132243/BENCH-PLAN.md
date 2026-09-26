# Docking bench runs: no violent capture reproduced

Run 2026-09-26 at master 51b0c8799, Sprout `docking-blast-bench`, own
`target/`, one game at a time, headless (`--norender --channel step`, no
`--record`). Build: `nix develop --command cargo build --features dev`.
Common flags: seed 7, `--audit-raw`, `RUST_LOG=info,nova_ship=debug`,
`ALSA_CONFIG_PATH=empty`. pi runs: gpt-5.6-luna, thinking low. Artifacts:
`/tmp/nova-docking-bench-51b0c8799/<run>/` (audit.jsonl with raw snapshots,
score.json, game.log, agent.log; `/tmp` is not durable). Scripts in `bench/`.

## Fixture check (t0 observe, `bench/observe_once.py`)

Both loose fixtures load with no `game_errors`. `me.docking.pair` is null
until a travel lock stands. Raw positions are engine units (1 = 10 m).

- The first warship fixture (tender face 40 m off the starboard collar,
  `bench/docking_warship_tender.40m-invalid.ron`) was invalid for its purpose.
  The radar locks along the nose (`crates/nova_bench/src/pages/docking.md:31`),
  the tender sat at bearing [104.2, 0.0] (t0 observe,
  `t0-docking_warship_tender`), and the Line Warship spans cells z -9..8.5
  (about 180 m, 10.0-cell swept radius). The tender's nearest section centre
  was 5 cells out. Run B shows the lock turn ramming it.
- Fix (authoring only): tender origin (170.69, 0, 24.15), face at (140, 0, 0),
  115 m face gap, same 12 deg yaw. The tender's inner edge is 13.8 cells from
  the warship centre, about 38 m outside the sweep (rotation about the hull
  origin; centre-of-mass offset not measured). The fixture, its README entry
  and `docs/agent-bench.md` are updated. t0-B2: both ships at rest.

## Runs

| Run | Fixture | Pilot | Ended | Ticks | Turns | Dmg | Result |
| --- | --- | --- | --- | --- | --- | --- | --- |
| A | docking (tender/spar) | pi | finish gave_up | 2712 | 32 | 0 | pilot failure: closest gap 52.8 m, never eligible |
| B | warship, 40 m (invalid) | pi | finish gave_up | 1631 | 11 | 0.74 | fixture failure: lock turn rammed the tender, no DOCK |
| B2 | warship, 115 m | pi | finish gave_up | 2053 | 27 | 0 | pilot failure: closest gap 73 m; tender untouched |
| C1 | docking | script v1 | ticks | 12000 | 2336 | 0 | script failure: 11.3 m vertical offset never removed |
| C2 | docking | script | finish done | 3068 | 640 | 0 | capture at gap/speed edge, clean |
| C3 | warship | script, beam | finish done | 4201 | 714 | 0 | capture at gap/speed edge, clean |
| C4 | warship | script, bias 13.5 | finish done | 3680 | 610 | 0 | capture at facing edge, clean |
| C5 | warship | script, spin | finish done | 4206 | 717 | 0 | capture at spin edge, clean |
| C6 | warship | script, tap <= 0.5 m | finish gave_up | 7404 | 4204 | 0 | no capture: hulls touch at 1.9 m gap |
| C7 | warship | script, tap <= 1.9 m | finish done | 4309 | 822 | 0 | capture at 1.9 m, one tick before contact, clean |
| C8 | warship | script, tap after contact | finish done | 4315 | 824 | 0 | capture while touching, clean |

pi costs: A $0.034, B $0.016, B2 $0.045. Both pi pilots gave up with turns
left; neither reached a 10 m gap.

The script `bench/dock_pilot.py` (a `cmd:` agent): nose on the target, hold
`radar_hold` to the travel lock, square up on `align_bearing_deg` (target
azimuth 0 for a bow port, 90 for a starboard port, plus `FACE_BIAS`), null
the lateral offset on the RCS with `autopilot_stop` as a brake, aim the nose
at their face (bow port only), then close at `CLOSE_MPS` 4.5 with 5-tick acts
and 1-tick acts under a 14 m gap. It taps DOCK on the first eligible view
that meets `DOCK_AT_GAP` and `DOCK_MAX_REL`, then runs 180 1-tick acts and
ten 30-tick acts. `SPIN_DPS` starts a yaw turn at an 11 m gap.

## Accepted captures (raw snapshots, 1-tick cadence)

Gate values from the view on the DOCK tick; velocities from the raw roots.
Capture envelope: 10 m, 15 deg, 5 m/s, 5 dps.

| Run | Gap | Facing | Rel | Spin | Before (player / partner) | 1st connected tick | Settled pair | Peak partner spin | Damage |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C2 | 10.0 m | 8.0 deg | 4.50 | 0.0 | 4.48 / 0 m/s | 2.28 / 2.26 m/s | 2.19 m/s, 0.52 dps | 7.08 dps, 1 tick | none |
| C3 | 10.0 m | 0.1 deg | 4.50 | 0.0 | 4.49 / 0 | 3.52 / 3.23 | 3.41 m/s | 2.03 dps | none |
| C4 | 10.0 m | 13.6 deg | 4.50 | 0.0 | 4.50 / 0 | 3.43 / 3.37 | 3.41 m/s | 0.60 dps | none |
| C5 | 9.5 m | 1.0 deg | 4.50 | 5.00 | 4.49 / 0, 4.95 dps | 3.70 / 3.68 | 3.41 m/s | 3.30 dps, damped in 5 ticks | none |
| C7 | 1.9 m | 0.1 deg | 4.50 | 0.0 | 4.50 / 0 | 3.52 / 3.20 | 3.41 m/s | 1.76 dps | none |
| C8 | 1.9 m | 0.1 deg | 0.70 | 1.50 | 3.48 / 3.25 (touching) | 3.43 / 3.37 | 3.41 m/s | 0.30 dps | none |

- Each capture logs one `on_docking_connection_request: connection ...`
  and one `track_docking_transitions ... captured` in game.log, and no
  release. Both roots report the connection on the tick after the tap.
- Warship pairs settle at 3.41 m/s. Momentum check: 185.65 / (185.65 +
  58.94) x 4.49 = 3.41. Hull health, section health, skin plate health and
  dead-section counts stay at t0 values on both ships through 480 ticks
  after capture. Contact distance holds (for example C3: 67.1 m every tick).
- Run B, non-docking: the warship turned at up to 37 dps with only
  `camera_rotate` and `radar_hold` input. By tick 121 the tender moved at
  21.1 m/s and 9.4 dps, and at 37.4 m/s by tick 181. The player took 0.74
  hull damage; the tender took none. No connection existed. This is a
  hull-sweep collision, not the reported capture failure.
- C6: skin plating meets at a 1.9 m face gap. A 4.5 m/s contact is inelastic
  (relative 4.5 -> 0.5 m/s in one tick, no damage). The pair stays eligible
  while touching. Continued RCS push slid the tender 9.7 m "up" over 3500
  ticks without damage.

## Replay

`bench replay C5/audit.jsonl --out C5-replay`: `replay_close` at tick 4206,
same health on both ships, tender distance 66.7 m recorded vs 66.6 m
replayed. The replay is close, not byte-identical, with no weapons in play.

## Verdict and remaining gaps

No violent capture was reproduced: 6 accepted captures at the gap, speed,
facing and spin edges, and at hull contact, with zero damage. This does NOT
show that the bug is absent. Untested:

- Flying the docked pair (`flight.dock_helm`), undocking (second DOCK), and
  release by damage. All runs kept the helm neutral.
- All gate edges at once. Each run pushed one or two edges.
- Vertical or roll offsets on a beam port, the warship `port_collar`,
  generated open-world derelicts, and captures near other bodies, gravity or
  combat.
- Real-time play. The bench steps one fixed tick per frame with no render.
  Frame hitches and multiple fixed steps per frame were not exercised.
  Unverified: the owner's failure may depend on them.
- Pilot-driven edge captures: both pi pilots failed before the envelope.
