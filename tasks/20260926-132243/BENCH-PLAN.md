# Docking bench runs

The upright runs below did not reproduce the blast. The pitched-over runs
in [Pitched-over captures](#pitched-over-captures-reproduced) did.

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

## Pitched-over captures (reproduced)

Run 2026-09-26 at e9df06335 (docs-only after 51b0c8799), same Sprout,
same build, one game at a time, headless step mode. Flags as above plus
`RUST_LOG=info,nova_ship=debug,nova_gameplay::integrity=trace`, so every
contact impulse and every impact tally is in game.log. Artifacts:
`/tmp/nova-docking-roll-tXmroo/<run>/` (not durable).

Owner live run (from the owner's message; the full paste was not available
to this worker): `cargo run --features dev -- --scenario-file
crates/nova_bench/scenarios/docking_warship_tender.content.ron`, warship
turned about 180 deg about its X axis (belly up), DOCK at 11:43:51.823,
`sever_disconnected_structures: 1494v0 split into 2 bodies` at .861,
`impacts: 260 contacts for 348226.97 damage` at .886, 18 sections destroyed.

A spawn rotation cannot give this pose: the attitude command follows the
camera rig, which starts at identity
(`crates/nova_gameplay/src/transform/point_rotation.rs:27`, `:74`), so a
rolled spawn slews back. A mouse pitch turns the rig about its own right
axis (`point_rotation.rs:119-123`), so `PITCH_OVER=180` in
`bench/dock_pilot.py` flies the owner's pose from the unchanged B2 fixture.

| Run | Fixture | Pilot | Pose at DOCK | Gap / facing / rel | Joint target error | Result |
| --- | --- | --- | --- | --- | --- | --- |
| R1 | spawn rolled 180 (deleted) | script, beam | slewed off by the rig | never eligible | - | fixture failure |
| R0 | B2 | script, beam, pitch 0 | upright | 10.0 m / 0.1 / 4.50 | 0.00 deg | clean, 0 contacts |
| R2 | B2 | script, beam, `PITCH_OVER=180` | belly up, nose aft | 9.9 m / 1.1 / 4.50 | 23.96 deg | 258 contacts, 117975 dmg, 17 nodes |
| R3 | `bench/docking_warship_tender.tender-yaw0.ron` | as R2 | belly up (hull up -Y) | 9.9 m / 0.1 / 4.50 | 0.00 deg | clean, 0 contacts |
| R4 | B2 | script, `PITCH_OVER=45` | - | never eligible (elevation not squared) | - | pilot failure, untested |

R2 in detail (`bench/window.py`, `bench/section_diff.py`,
`bench/joint_basis_check.py`):

- t3542, last tick before DOCK: player 4.50 m/s, tender at rest, no spin,
  full health on both. No `contact impact` line precedes the connection line
  in game.log: no pre-joint contact.
- The connection line is followed by 258 `contact impact` lines and ONE
  tally: `integrity: destroyed 17 nodes (reinforced_hull_section x16,
  torpedo_section x1)`, `impacts: 258 contacts for 117974.59 damage`.
- t3544: the tender moved 13 m and turned at 320 dps (94 m/s); the player
  took 1366.7 hull damage. The player lost plates at cells x 1..2, z -6..-2
  and `torpedo_starboard`, forward of the collar; the tender lost 10 plates
  aft of its collar. That is the tender swung about the anchor into the
  warship's starboard bow.
- From t3545 the pair holds a relative rotation 0.01 deg from avian's joint
  target and 23.97 deg from the capture pose. All six upright captures
  (C2 to C8, R0) have 0.00 deg between the two.

Cause (code, confirmed by an independent review):

- `crates/nova_ship/src/sections/docking_section/connection.rs:197-216`
  builds `FixedJoint::new(first, second).with_anchor(midpoint)
  .with_basis(Rotation(first_rotation))`: global frames.
- avian3d 0.7.0 `src/dynamics/joints/mod.rs:1131` converts a global basis
  with `basis * rot.inverse()`. The solver uses `rot * local_basis`
  (`src/dynamics/solver/xpbd/joints/shared/fixed_angle_constraint.rs:49,54`),
  so the correct value is `rot.inverse() * basis`. The two agree only when
  the two hull rotations commute.
- Frame 1 is correct (identity). Frame 2 becomes `rot1 * rot2^-1` for
  `rot2^-1 * rot1`. Warship pitched 180 about X against a tender yawed 12 deg
  about Y: 24 deg. The joint drives the tender through it in one step.
- Jointed bodies still collide (no `JointCollisionDisabled` in any Nova
  crate), so the swept hulls meet under the joint impulse.
- The anchor conversion (`mod.rs:957-961`) is correct.
- Every docking unit test and the docking example capture with hull 1 at
  identity (`crates/nova_ship/src/sections/docking_section/tests.rs:78-83`),
  which commutes with anything. They cannot see this.

Log order: contact damage lands in `FixedPostUpdate`
(`crates/nova_gameplay/src/integrity/core.rs:110`), the sever runs in
`Update` after `IntegritySystems`
(`crates/nova_ship/src/sections/integrity.rs:198-203`), and the tally prints
in `Last` (`core.rs:113`). The owner's sever line before the impacts line is
one frame's result of the same contacts, not a cause.

Replay: `bench replay R2/audit.jsonl` is `replay_mismatch`. It drifted
0.2 m and the DOCK tap met a 10.1 m gap (refused). The closed-loop script
is the reproducer, not the replay.

Not settled: the entity 1494v0 in the owner's sever line, and the owner's
exact pair and relative pose. Real-time play was not run, because the
headless step reproduced the failure.

## Local-frame fix

Fix based on e9df06335, same Sprout, build and flags as the pitched-over
runs. Artifacts: `/tmp/nova-docking-localframe-fix/`
(`<run>/`, `<run>.stdout`, `unit-*.log`; not durable).

Change (the cause lines above describe the pre-fix code):

- `docking_section/port.rs`: `DockingPorts::body_rotation(root) -> Option<Quat>`
  is now `body_pose(root) -> Option<(Vec3, Quat)>` (avian `Position`,
  `Rotation`).
- `docking_section/connection.rs` `on_docking_connection_request`: read both
  root poses after the candidate poses, refuse on a missing pose, and build
  `FixedJoint::new(first, second)
  .with_local_anchor1(r1^-1 (m - p1)).with_local_anchor2(r2^-1 (m - p2))
  .with_local_basis2(r2^-1 r1)`, with `m` the face midpoint and basis 1 left
  at identity. Avian skips its global-to-local conversion for all-local
  frames (`avian3d-0.7.0/src/dynamics/joints/fixed.rs:259-265`). The helm
  seed uses the rotations already read. No gate or collision change.
- Proof: `a_dock_between_a_pitched_and_a_yawed_hull_holds_the_capture_pose`
  (`docking_section/tests.rs`). Hull 1 pitched 180 deg about X, hull 2 yawed
  12 deg about Y, one step after DOCK: world anchors from both joint frames
  on the face midpoint, world bases equal. Before the fix it failed with
  `23.999985 deg off` (`unit-before.log`, anchors passed). After: ok, and
  all 33 `sections::docking_section` tests pass
  (`unit-docking-module-final.log`).

Scripted bench, same commands and env as R0, R2 and R3 above. "Joint error"
is the settled relative rotation against the capture pose
(`bench/joint_basis_check.py`, 10 ticks after the tap). Peak tender motion
covers the 6 ticks after the tap.

| Run | DOCK tap | Gap / facing / rel | Joint error | Peak tender | Contacts after DOCK | Damage | Sections at end |
| --- | --- | --- | --- | --- | --- | --- | --- |
| R2 before | t3542 | 9.9 m / 1.1 / 4.50 | 23.97 deg | 94.1 m/s, 320.4 dps | 262 `contact impact` lines (258 in the capture tally), 17 nodes destroyed | player 1366.7 hull, tender hp 9990 -> 7906.7 | player 155 -> 148 |
| R2 after | t3537 | 10.0 m / 1.1 / 4.50 | 0.01 deg | 3.54 m/s, 1.87 dps | 0 | 0.0 | 155 / 51, unchanged |
| R0 before | t3322 | 10.0 m / 0.1 / 4.50 | 0.00 deg | 3.50 m/s, 2.05 dps | 0 | 0.0 | unchanged |
| R0 after | t3316 | 10.0 m / 0.1 / 4.50 | 0.00 deg | 3.55 m/s, 1.80 dps | 0 | 0.0 | unchanged |
| R3 before | t3794 | 9.9 m / 0.1 / 4.50 | 0.00 deg | 3.54 m/s, 1.92 dps | 0 | 0.0 | unchanged |
| R3 after | t3799 | 10.0 m / 0.1 / 4.50 | 0.00 deg | 3.54 m/s, 1.97 dps | 0 | 0.0 | unchanged |

- R2 after: both roots hold the rotations of R2 before at the tap
  (identical quaternions). The pair's relative rotation stays 0.00 deg from
  the capture pose and 23.96 deg from the old avian target through t3637.
  Both roots end connected at 3.41 m/s, 0.01 dps, full health, no
  `sever_disconnected_structures` line, no `integrity: destroyed` line, and
  no release line.
- R0 and R3 after match their before runs: the fix leaves commuting
  captures unchanged.
- The R2 tap is 5 ticks earlier than before (t3537 vs t3542, gap 10.0 vs
  9.9 m). The closed-loop script and the build differ, so the pilot met the
  gate on a different tick. Same pose, same gate values within 0.1 m.

Remaining uncertainty:

- One seed and one scripted pair per case. The owner's live run (real-time
  frames, the exact pair behind entity 1494v0) was not repeated after the
  fix.
- Jointed bodies still collide. A capture at the edge of the facing cone
  can still bring hull plating into contact under a correct joint; this
  fix removes only the wrong joint target.
- No replay parity: the R2 audit replay drifted and missed DOCK before the
  fix, and it was not rerun.
