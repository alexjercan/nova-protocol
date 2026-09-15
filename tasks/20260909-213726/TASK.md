# Systems ranges: flight legs, gravity wells and the AI patrol

- STATUS: CLOSED
- PRIORITY: 70
- TAGS: v0.14.0, testing, examples, flight, ai

## Goal

Add only the composed flight, gravity, patrol and helm-order proof still missing
after the v0.14.0 simulation sweep. Do not repeat focused physics tests or the
live AI combat and evade ranges that landed after this task was written.

Rules: `examples/systems/README.md`. Each new range gets its `[[example]]`
block and roster slugs in `crates/nova_probe_cli/tests/catalog_drift.rs`, and
lands in the existing `world` shard from `20260909-213100`. A range that finds a
defect fixes it in the same lane and records it here.

Use both `block_skiff` and `block_carrier` only where hull scale is the subject.
A gravity substrate or AI lifecycle range uses the smallest suitable shipped
fixture instead of staging a carrier without a scale-dependent claim.

## Re-check, 2026-09-12

Reviewed commits and current code from `v0.13.0` through `27010fb39`, then
compared this task with the completed hull-size sweeps and the other open sprint
tasks.

Work landed or was confirmed after this task was written:

- The flight physics suite already proves GOTO and GotoPos arrival, STOP braking
  and release, ORBIT hold for a full lap, gravity-aware arrival, action
  capability loss, and camera handback continuity.
- `c6a181bd3` added `system_ai_combat`. It proves on two combat scales that an
  engaged AI hull is flown by its flight computer and settles at standoff.
- `593505fc0` added `system_ai_evade`. It proves the threat-driven three-leg
  weave on `block_picket` and `block_warship`, the largest shipped combatant.
  `block_carrier` is an `AINonCombatant`: it cannot acquire a fight and cannot
  enter Evade.
- `ff89a64c3` moved sized-body detours into generic flight navigation and added
  physics proof that patrols on both reference hull radii clear a rock with the
  mover's own hull included.
- Patrol physics tests already prove a route physically reaches one waypoint
  and turns onto the next.
- `f3f745791` moved AI combat onto the flight computer. Focused tests prove one
  suitable engine cluster is selected instead of every engine, torque-nulling
  allocation, and lateral recruitment on a damage-shifted hull.
- Gravity tests already prove inward pull, SOI release, well-removal cleanup,
  overlapping-well hysteresis, orbit hold, and the opt-in policy for piloted
  ships and torpedoes.
- Ship-order and scenario-tracker tests already prove order completion,
  interruption, resumption, cancellation and failure, including exactly one
  event for each report.
- `loop_helm_orders` drives an order interruption and resumption as a capture,
  but remains a screenshot producer with no systems-range outcomes.

The current unstaged simulation-defect work belongs to `20260909-214706` and is
not counted as landed proof. Preserve it.

## Decisions

### Scope and ownership

- Keep `system_flight_legs`, but make its subject the composed shipped-app verb
  chain and active-action replacement. Do not add separate roster claims for
  mechanics already pinned by focused flight tests.
- Keep `system_gravity_wells` only for production-round curvature and a composed
  live handoff between wells.
- Do not make the lead pip's current straight solution a contract. This task
  neither asserts that it misses nor makes it gravity-aware. A gravity-aware
  aiming policy needs its own decision and task.
- A ship's dominant well is not always the instantaneously strongest well. The
  incumbent remains until a challenger exceeds the existing hysteresis margin.
  Do not remove or bypass that hysteresis.
- Keep `system_ai_patrol` only for the composed return from combat to an existing
  patrol and for physical leash recovery.
- Do not stage Evade on `block_carrier`. Evade remains owned and fully proved by
  `system_ai_evade` on valid combatants.
- Drop the proposed `system_engine_groups`. Its AI migration purpose is complete
  and its allocation claims have focused proof. Add only a focused test for
  live-set recomputation after a drive is lost if an exact test is still absent.
- Keep `system_helm_orders` as formal live proof of authored order interruption
  and resumption. `loop_helm_orders` stays an assert-free capture.
- `system_ai_patrol` owns an AI's built-in `AIPatrolRoute` lifecycle.
  `system_helm_orders` owns a scenario-authored `ShipHelmOrder`; do not merge
  the two state machines or treat one as proof of the other.

### Sprint composition

- The Evade cooldown bug remains owned by `20260909-213525`. Do not change its
  exit policy here.
- Invalid authored thrusters and no-live-engine disengagement remain owned by
  `20260909-214706`.
- Scenario refusal, teardown, pause and focus behavior remain owned by
  `20260909-214629`.
- Combat and destruction ranges remain owned by `20260909-213623`.

## Remaining work

### `system_flight_legs`

Stage the shipped app around one well. Run the same composed chain on
`block_skiff` and `block_carrier`.

- [x] Drive STOP, GOTO, GotoPos and ORBIT through one condition-driven chain on
      each reference hull. `956e891fc`.
- [x] Assert each hull completes the full chain through the production
      `Autopilot` and real sections. Treat individual arrival, rest and orbit
      checks as delivery guards for this one composed outcome, not as new
      subsystem contracts. Slug
      `the composed leg chain completes on both hulls`.
- [x] During the staged unobstructed GOTO, assert one coast-to-brake transition;
      do not generalize that count to disturbed or replanned flight. Slug
      `an unobstructed goto coasts before it brakes`.
- [x] Replace an active `AutopilotAction` with a second action and assert the new
      action owns the helm on the next flight tick. Slug
      `a replacement action owns the helm on the next flight tick`.
- [x] Do not repeat standalone GOTO/GotoPos arrival, STOP rest, ORBIT lap, or
      gravity-aware park claims from `flight/tests/`.
- [x] Do not repeat camera handback continuity; `camera/handback.rs` already
      pins it.
- [x] Do not call direct `AutopilotAction` replacement a `ShipHelmOrder`
      interruption. Authored orders have their own range below.

### `system_gravity_wells`

Use one piloted shipped hull and production gun rounds. Gravity acceleration is
mass-independent, so a carrier adds cost without another substrate claim.

- [x] Cross a production gun round through a live planet SOI beside an otherwise
      identical no-gravity control trajectory. DROPPED as an exact duplicate -
      see "Dropped duplicates" below.
- [x] Assert the production round curves toward the well while the control path
      remains straight. DROPPED as an exact duplicate - see "Dropped
      duplicates" below.
- [x] Fly the piloted hull from one overlapping live SOI into another and assert
      it carries exactly one `DominantWell` at a time. `2139a0582`, slug
      `a hull inside two wells is owned by one that reaches it`.
- [x] Assert the incumbent remains through the hysteresis margin and handoff
      occurs only when the challenger exceeds it. Slug
      `the incumbent well holds until a challenger clears the margin`.
- [x] Do not assert that the instantaneous strongest well always wins.
- [x] Do not repeat inward pull, SOI release, well-removal or orbit-lap proof.
- [x] Do not assert a lead-pip miss and do not implement gravity-aware lead in
      this task.

### `system_ai_patrol`

Use one armed shipped combatant that can patrol and fight. Do not use
`block_carrier` for combat lifecycle proof.

- [x] Let the AI establish an `AIPatrolRoute`, acquire a hostile, fight it, and
      observe that hostile die or become unavailable. `a7bb65040`, slug
      `a hostile takes a flying patrol off its leg`.
- [x] Assert the same route and next patrol leg resume after the target is gone;
      do not install a replacement route in the harness. Slug
      `the patrol resumes on the route it kept`. The harness never writes
      `AIPatrolRoute` or the picket's `Autopilot`; the hostile is taken out of
      the fight through `HealthApplyDamage` on its bridge.
- [x] Pull or lure the combatant beyond an authored leash, then assert it breaks
      off and physically returns inside the re-engagement band. Slug
      `the leash walks a dragged-out picket home`.
- [x] Do not repeat patrol leg advancement or sized-body detour proof.
- [x] Do not add a generic acquisition outcome already held by
      `system_ai_combat`.
- [x] Do not repeat threat-memory decay arithmetic.
- [x] Do not stage or assert Evade here.

### Focused severed-drive allocation test

- [x] Re-check the flight and thruster tests for an exact live-set update after
      a drive becomes inactive, destroyed or detached. ABSENT: the nearest
      sibling,
      `single_drive_on_a_shifted_hull_recruits_a_lateral_to_hold_heading`
      (`crates/nova_ship/src/flight/tests/manual.rs:156`), covers a hull that
      never had the second drive, not one that LOSES it.
- [x] If absent, add the cheapest focused test that starts with a balanced
      multi-group hull, removes one drive from the live allocation set, advances
      one flight tick, and asserts the surviving set is selected and balanced.
      `ea5bb2605` added
      `severing_a_drive_reallocates_the_burn_across_the_surviving_live_set`
      (`crates/nova_ship/src/flight/tests/manual.rs:716`), which severs through
      the production eligibility seam (`SectionInactiveMarker`).
- [x] Do not create `system_engine_groups`.
- [x] Do not repeat cluster selection, torque-nulling, lateral recruitment or
      AI flight-computer migration proof.

### `system_helm_orders`

Use a scenario-authored interruptible order. It may mirror the capture's scene,
but it must own explicit systems-range outcomes and must not turn the screenshot
producer into a correctness range.

- [x] Issue a real scenario `ShipHelmOrder` and assert it takes helm authority
      and physically moves the ordered ship along its directive. `c223887ea`.
      The order is installed by the real `PatrolShip` scenario action out of the
      same `OnStart` batch that spawns the hull - not by the harness. Slug
      `a scenario order takes the helm and flies the hull`.
- [x] Introduce a real hostile contact and assert the authored interruption
      policy removes order helm authority without replacing the stored
      directive. Slug
      `a hostile contact takes the helm and leaves the order alone`. The
      directive is compared on EVERY interrupted fixed step, not only at the
      two ends.
- [x] Clear the threat and assert the same directive and leg resume with helm
      authority restored. Slug `the cleared sky hands the same leg back`. The
      threat is cleared through `HealthApplyDamage` on the contact's bridge.
- [x] Do not add another exactly-once report assertion. Ship-order and scenario
      tracker tests already count each lifecycle report once.
- [x] Keep `loop_helm_orders` as a capture with no systems outcome roster.

## Proof and workflow

- Every new assertion has an adjacent `outcome: <slug>` marker and matching
  `catalog_drift.rs` roster slug.
- Every new example has an explicit root `Cargo.toml` `[[example]]` block.
- Register the four ranges in the existing `world` shard. Do not redesign the
  probe split here.
- Build examples with `AppBuilder`. Keep each range in its own file; do not add
  `systems/common/`.
- Use authored quantities in meters and convert only at engine boundaries.
- Never assert milliseconds. Record performance observations without grading
  them as correctness.
- Reproduce any newly found defect before changing production code. Fix it in
  the same lane and record it here.
- Run only affected crate checks and ranges. Inspect each `checks.json` and
  `report.html`.
- Run all four affected ranges green three consecutive times in the `world`
  shard before closing.
- Documentation and changelog changes are required only for a behavior defect
  found and fixed. Pure proof additions do not need a player-facing changelog
  entry.

## Closing record, 2026-09-15

Branch `world-ranges`, off `master` at `2d11c2e52`.

### Commits

| commit | subject |
| - | - |
| `ea5bb2605` | Prove a severed drive leaves the live thrust allocation set |
| `2139a0582` | Prove one well owns a hull crossing two spheres of influence |
| `956e891fc` | Prove a composed leg chain flies on two hulls |
| `9e8498769` | Hold the well crossing's beats to an honest backstop |
| `a7bb65040` | Prove a patrol keeps its route across a fight and a tether |
| `c223887ea` | Prove a helm order keeps its leg across a break-off |

### Dropped duplicates

- `system_gravity_wells`, production round curvature against a no-gravity
  control. DROPPED: `a_round_curves_under_a_well_and_flies_straight_without_one`
  (`crates/nova_gameplay/src/rounds.rs:2151`, added by `fce8b7619`) already
  integrates the same production round twice from one seed - once with the well
  and once without - and asserts the curve against the straight control. The
  range would have restaged it at far greater cost with no new claim, so
  `system_gravity_wells` keeps only the two composed well-ownership outcomes.

Nothing was dropped from `system_flight_legs`, `system_ai_patrol` or
`system_helm_orders`. The nearest existing tests were re-read and are not the
same claim:

- `a_hostile_in_detection_range_interrupts_the_patrol`
  (`crates/nova_ship/src/input/ai/passive.rs:1044`) runs a hand-driven pipeline
  on a synthetic world over a route that was assigned and never flown.
  `combat_interrupts_the_orbit_and_calm_resumes_it` (same file, line 1299) is
  the ORBIT routine, not a patrol.
- `leash_hysteresis_uses_a_reengage_band`
  (`crates/nova_ship/src/input/ai/behavior.rs:578`) and
  `the_leash_breaks_off_combat_beyond_its_radius` (same file, line 606) are pure
  function tests over `leash_exceeded` and `next_behavior_state`. Neither flies
  a hull out past its tether or back inside the band.
- `an_interrupted_order_resumes_from_its_own_directive`
  (`crates/nova_ship/src/flight/order.rs:1108`) calls `interrupt_ship_order` and
  `resume_ship_order` by hand on a synthetic app: no `AIOrderInterruption`
  policy, no hostile, no physics. `interrupt_ai_ship_orders`
  (`crates/nova_ship/src/input/ai/mission.rs`) had no test at all.

### Defects

None. No production behavior changed in this task: the six commits add four
`examples/systems/` ranges, one focused flight test
(`crates/nova_ship/src/flight/tests/manual.rs`), their `[[example]]` blocks and
their roster slugs. `9e8498769` only tightened a range's own step backstop. So
no changelog entry and no documentation change is owed.

### Shard membership

The probe job shards by EXAMPLE DIRECTORY, not by a hand-kept list
(`.github/workflows/ci.yaml:218-246`, matrix
`category: [screenshots, systems, playable]`). All four ranges live in
`examples/systems/`, so they are already shard members and the split needs no
edit. The `world` shard proposed when this task was written belongs to
`20260909-213100`, which is still OPEN and owns that decision; this task
deliberately did not touch the CI split.

### Three consecutive runs, final source (`c223887ea`)

Run directly, one at a time on a quiet host (load average 0.59 at the start of
the set), as
`DISPLAY=:99 cargo run --features debug probe run <range> --correctness-only`.
Every run: `process_exit` PASS, `run_completed` PASS, `reached_playing` PASS,
`invariants_held` PASS (0 violations), `log_clean` PASS (0 offending lines),
`artifacts_loadable` PASS (0 unloadable). `capture_simulated` and
`fps_within_baseline` are N/A - no range claims a frame cost.

| range | run 1 | run 2 | run 3 |
| - | - | - | - |
| `system_helm_orders` | OK 1044f 25s | OK 1039f 25s | OK 1047f 25s |
| `system_ai_patrol` | OK 3261f 62s | OK 3262f 61s | OK 3264f 62s |
| `system_gravity_wells` | OK 356f 13s | OK 357f 13s | OK 354f 13s |
| `system_flight_legs` | OK 4804f 102s | OK 4774f 102s | OK 4822f 102s |

Every cell is a `measured 6/8` verdict; `f` is the frame the run closed its
bracket on. Seconds and frame counts are reported for a reader. Nothing in
these ranges grades a time.

### Other checks

- `cargo fmt --all --check`: clean.
- `cargo test -p nova_probe_cli --test catalog_drift`: 2 passed
  (`catalog_matches_disk`, `systems_ranges_assert_their_invariant_roster`).
  `SYSTEMS_INVARIANTS` 377 -> 385 across the four ranges.
- `cargo test -p nova_ship --lib
  severing_a_drive_reallocates_the_burn_across_the_surviving_live_set`:
  1 passed, 969 filtered out.
- `RUSTFLAGS="-D warnings" cargo check --workspace --all-targets` (CI's
  `check / default features` job, no `--features debug`): exit 0. It first
  found four dead-code errors - `CROSS_END` and `CROSS_SPEED` in
  `system_gravity_wells`, `OUTBOUND_MARK` and `Subject::Carrier` in
  `system_flight_legs` - all read only from `#[cfg(feature = "debug")]` code.
  Each declaration now carries the gate its consumer already had.
- `cargo check --workspace --all-targets --features debug`: exit 0.
- Deliberately skipped: a full workspace `cargo test` and any Clippy sweep. The
  full suite exhausts memory on this box, and the repository rule is affected
  checks only.

## Done when

- The four retained ranges and the focused severed-drive check are complete or
  dropped with exact evidence recorded here.
- `system_engine_groups` and every checked duplicate remain absent.
- No range restores the strongest-well, gravity-miss, carrier-Evade, or sibling
  task behavior removed above.
- The new ranges are cataloged, rostered, assigned to `world`, and green three
  times in a row.
- Every defect found by the ranges is fixed, proved, documented where behavior
  changed, and listed here.
