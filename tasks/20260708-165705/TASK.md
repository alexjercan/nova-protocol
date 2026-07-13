# Multi-target tracking + subtarget cycle HUD

- STATUS: CLOSED
- PRIORITY: 9
- TAGS: v0.5.0, hud, spike

Spike: tasks/20260708-165647/SPIKE.md

Phase 3. Track several lockable candidates at once (the aim-assist in
`update_spaceship_target_input` already enumerates them, but keeps only the best),
show them as a target list / bracketed subtargets, and let the player cycle the
active lock between them (key/gamepad) in addition to panning. Consumer of the
screen-projected-indicator widget (20260708-165700).

Direction: promote the transient best-pick into a maintained candidate set (a
resource or per-candidate marker), render the set, and drive an explicit cycle input
alongside the existing look-to-aim behaviour.

Design spike (20260711): tasks/20260711-163800/SPIKE.md -
candidate set resource (top-5 hostile ships, ranked), dim bracket markers via
the screen-indicator widget, CTRL+scroll cycle (plus CTRL+brackets, dpad
up/down) with a ~4 s lock pin mirroring the component pin, and new
bottom-left hint rows [SCROLL] COMPONENT / [CTRL+SCROLL] TARGET.

Narrowed (20260709) by
tasks/20260709-192358/SPIKE.md: the subtarget-cycle
half lands with the component fine-lock (tatr 20260709-192522/192523). This
task keeps the multi-target half only: maintaining and rendering the candidate
SET of lockable ships and cycling the active ship lock between them.

## Goal

The player sees the set of nearby lockable hostile ships as dim bracket
markers, and CTRL+scroll (or CTRL+brackets / dpad) deliberately moves
the active lock through that set, pinned against the aim-driven picker for a
few seconds. The bottom-left keybind cluster documents both scroll gestures.

## Steps

- [x] Candidate set resource in `crates/nova_gameplay/src/input/targeting.rs`:
      `SpaceshipPlayerTargetCandidates` (ranked `Vec<Entity>`), maintained at
      the end of `update_spaceship_target_input` from the `candidates` vec it
      already collects. Membership: hostile ships (is_hostile + is_ship
      threaded through the collected tuple); rank by angle to the aim ray,
      then distance (`rank_ship_candidates`); keep top
      `TARGET_CANDIDATE_COUNT` (const, 5); the current lock stays a member
      while it is still a ranked hostile ship, even out of the top N
      (`maintain_candidates`). Exported via the module prelude. Unit tests on
      both pure helpers.
- [x] Ship-lock pin state: `pinned_until: Option<f32>` ON the candidates
      resource (a second resource for one f32 read worse). While pinned,
      `update_spaceship_target_input` does not overwrite the lock; the pin
      clears on deadline (`TARGET_PIN_WINDOW` const, 4.0 s) or when the
      pinned entity stops being collectible (died / out of range, with the
      existing incumbent hysteresis). While pinned, candidate maintenance is
      order-stable: drop dead entries, append newcomers, no re-rank. Unit
      tests: pin holds against the cone pick, expires, dies with target,
      stable order.
- [x] Cycle input actions `TargetCycleNextInput`/`TargetCyclePrevInput` in
      targeting.rs + observers stepping the lock through the candidates vec
      from the current lock's index (wrap; no lock / non-ship lock -> next
      starts at the best candidate, prev at the worst), setting the pin. No
      focus gate. Unit tests via `step_target_lock`.
- [x] Bindings in `crates/nova_gameplay/src/input/player.rs`: the flight rig
      moved from the `actions!` macro to a named `flight_input_rig()` bundle
      using `Actions::spawn(SpawnWith(...))` so the CTRL modifier action's
      Entity is capturable. `TargetCycleModifierInput` bound to
      ControlLeft/ControlRight; `Chord::single(modifier)` sits on the
      wheel/bracket BINDING entities (binding-level conditions are supported;
      action-level would have chorded the gamepad too);
      `BlockBy::single(modifier)` on the component-cycle actions suppresses
      the plain gesture while CTRL is held. Gamepad: NEXT only on DPadUp -
      DPadDown is ORBIT and dpad left/right are the component cycle (plan
      originally said dpad up/down; the collision was caught during
      implementation, and a wrapping next reaches every candidate).
      Verified by an end-to-end test
      (`ctrl_scroll_cycles_targets_and_blocks_the_component_cycle`) driving
      the REAL rig bundle through EnhancedInputPlugin with simulated
      keyboard + wheel: plain scroll cycles components only, CTRL+scroll
      cycles targets only, releasing CTRL hands the wheel back.
- [x] Candidate HUD in `crates/nova_gameplay/src/hud/target_candidates.rs`,
      following the `component_lock.rs` reconcile pattern: layer spawned/
      despawned with the player HUD in `hud/mod.rs`; one bracket per
      candidate EXCEPT the active lock; ApparentSize (min 28 px), offscreen
      Hide, dim hostile red. Bracket = hollow border on a full-size child
      node (the widget owns the indicator Node's size fields, so the border
      rides a child). Tests mirror `sync_component_markers`'s.
- [x] Hint rows: `FlightVerbHints` extended with `component_cycle` and
      `target_cycle` VerbHints (fixed labels "SCROLL" / "CTRL+SCROLL").
      Availability: component_cycle = focus complete on the lock with >= 2
      attached sections; target_cycle = a tracked candidate exists that is
      not already the lock. `ROW_VERBS` extended to 6 rows (COMPONENT,
      TARGET appended). Cluster tests extended + a new
      `cycle_hints_track_focus_and_candidates` test.
- [x] Verify: `cargo check --workspace` green, `cargo fmt --all` applied,
      all new/touched test filters green (input::targeting 40, input::player
      15, hud::target_candidates 3, hud::keybind_hints 4). Full suite runs
      in CI per project policy. Spike doc updated with a Fix record.

## Notes

- Key files: crates/nova_gameplay/src/input/targeting.rs (lock, focus,
  component cycle - the pin/step/observer patterns mirrored),
  crates/nova_gameplay/src/input/player.rs (flight rig bindings,
  FlightVerbHints), crates/nova_gameplay/src/hud/component_lock.rs
  (reconcile + highlight pattern), crates/nova_gameplay/src/hud/keybind_hints.rs,
  crates/nova_gameplay/src/hud/screen_indicator.rs (widget API),
  crates/nova_gameplay/src/relations.rs.
- bevy_enhanced_input 0.26: `Chord::single(entity)` fires only while the
  modifier action fires; `BlockBy::single(entity)` suppresses while it fires.
  Conditions attach to actions OR bindings (verified in src/condition.rs);
  binding-level Chord is what lets the gamepad binding stay unmodified.
  Native `mod_keys` on Binding was considered instead: rejected because a
  modifier-less binding still fires while CTRL is held (mod-count priority
  only matters with consume_input, which the flight rig deliberately does
  not use), so BlockBy would be needed anyway - one mechanism (the modifier
  action) beats two.
- Sibling 20260708-165704 consumes `SpaceshipPlayerTargetCandidates` (public
  via the prelude).

## Close record (20260711)

What changed: `SpaceshipPlayerTargetCandidates` resource (ranked top-5
hostile ships + `pinned_until`), pure `rank_ship_candidates` /
`maintain_candidates` / `step_target_lock` helpers, target-cycle input
actions + observers, the flight rig rewritten as `flight_input_rig()` with a
CTRL modifier action (Chord on wheel/bracket bindings, BlockBy on the
component cycle), the `target_candidates.rs` bracket overlay, and two new
hint-cluster rows fed by extended `FlightVerbHints`.

Alternatives considered: per-candidate marker components (rejected: churn +
every consumer re-derives ranking at N<=5); native Binding mod_keys
(rejected, see Notes); scroll-overflow cycling (rejected in the spike:
modal surprise destroys the focus dwell).

Difficulties:
- DPadDown was already ORBIT - the spike's dpad up/down plan collided;
  resolved as pad next-only on DPadUp.
- The end-to-end input test initially panicked: BEI finalizes its context
  registry in `App::finish`, so the test must run `app.finish()`/
  `app.cleanup()` before spawning the rig (the production app does this
  implicitly via the runner).
- MouseWheel gained a `phase: TouchPhase` field in Bevy 0.19; test
  simulation needed it.

Self-reflection: reading the bevy_enhanced_input source BEFORE writing the
bindings (condition attachment rules, mod_keys evaluation, action sorting)
avoided shipping a Chord-on-action bug that would have silently chorded the
gamepad binding; verify-first on the fiddly dependency paid off again. The
gamepad collision should have been caught at plan time by grepping existing
GamepadButton bindings - a plan step naming concrete buttons must cite the
current binding table.
