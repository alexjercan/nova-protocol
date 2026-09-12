# Systems ranges: flight legs, gravity wells and the AI patrol

- STATUS: OPEN
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

- [ ] Drive STOP, GOTO, GotoPos and ORBIT through one condition-driven chain on
      each reference hull.
- [ ] Assert each hull completes the full chain through the production
      `Autopilot` and real sections. Treat individual arrival, rest and orbit
      checks as delivery guards for this one composed outcome, not as new
      subsystem contracts.
- [ ] During the staged unobstructed GOTO, assert one coast-to-brake transition;
      do not generalize that count to disturbed or replanned flight.
- [ ] Replace an active `AutopilotAction` with a second action and assert the new
      action owns the helm on the next flight tick.
- [x] Do not repeat standalone GOTO/GotoPos arrival, STOP rest, ORBIT lap, or
      gravity-aware park claims from `flight/tests/`.
- [x] Do not repeat camera handback continuity; `camera/handback.rs` already
      pins it.
- [x] Do not call direct `AutopilotAction` replacement a `ShipHelmOrder`
      interruption. Authored orders have their own range below.

### `system_gravity_wells`

Use one piloted shipped hull and production gun rounds. Gravity acceleration is
mass-independent, so a carrier adds cost without another substrate claim.

- [ ] Cross a production gun round through a live planet SOI beside an otherwise
      identical no-gravity control trajectory.
- [ ] Assert the production round curves toward the well while the control path
      remains straight.
- [ ] Fly the piloted hull from one overlapping live SOI into another and assert
      it carries exactly one `DominantWell` at a time.
- [ ] Assert the incumbent remains through the hysteresis margin and handoff
      occurs only when the challenger exceeds it.
- [x] Do not assert that the instantaneous strongest well always wins.
- [x] Do not repeat inward pull, SOI release, well-removal or orbit-lap proof.
- [x] Do not assert a lead-pip miss and do not implement gravity-aware lead in
      this task.

### `system_ai_patrol`

Use one armed shipped combatant that can patrol and fight. Do not use
`block_carrier` for combat lifecycle proof.

- [ ] Let the AI establish an `AIPatrolRoute`, acquire a hostile, fight it, and
      observe that hostile die or become unavailable.
- [ ] Assert the same route and next patrol leg resume after the target is gone;
      do not install a replacement route in the harness.
- [ ] Pull or lure the combatant beyond an authored leash, then assert it breaks
      off and physically returns inside the re-engagement band.
- [x] Do not repeat patrol leg advancement or sized-body detour proof.
- [x] Do not add a generic acquisition outcome already held by
      `system_ai_combat`.
- [x] Do not repeat threat-memory decay arithmetic.
- [x] Do not stage or assert Evade here.

### Focused severed-drive allocation test

- [ ] Re-check the flight and thruster tests for an exact live-set update after
      a drive becomes inactive, destroyed or detached.
- [ ] If absent, add the cheapest focused test that starts with a balanced
      multi-group hull, removes one drive from the live allocation set, advances
      one flight tick, and asserts the surviving set is selected and balanced.
- [x] Do not create `system_engine_groups`.
- [x] Do not repeat cluster selection, torque-nulling, lateral recruitment or
      AI flight-computer migration proof.

### `system_helm_orders`

Use a scenario-authored interruptible order. It may mirror the capture's scene,
but it must own explicit systems-range outcomes and must not turn the screenshot
producer into a correctness range.

- [ ] Issue a real scenario `ShipHelmOrder` and assert it takes helm authority
      and physically moves the ordered ship along its directive.
- [ ] Introduce a real hostile contact and assert the authored interruption
      policy removes order helm authority without replacing the stored
      directive.
- [ ] Clear the threat and assert the same directive and leg resume with helm
      authority restored.
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
