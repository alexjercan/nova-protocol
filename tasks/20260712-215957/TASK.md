# Componentize targeting state: TargetLock + AvailableTargets on the ship root

- STATUS: CLOSED
- PRIORITY: 60
- TAGS: v0.5.0, targeting, refactor, spike

## Goal

Move the player targeting state from global resources to components on the
ship root entity (`SpaceshipRootMarker` + `PlayerSpaceshipMarker`), per the
user's model: `SpaceshipPlayerTargetLock` -> a `TargetLock` component and
`SpaceshipPlayerTargetCandidates` -> an `AvailableTargets` component
(entries + pinned_until). Pure refactor: NO behavior change; every existing
test keeps passing with mechanical query rewrites only.

## Steps

- [ ] In input/targeting.rs, define `TargetLock(pub Option<Entity>)`
      (Deref/DerefMut like the resource) and
      `AvailableTargets { entries: Vec<Entity>, pinned_until: Option<f32> }`
      as `#[derive(Component, ...)]`, keeping the resources' derives
      (Debug, Clone, PartialEq, Default) and doc comments (adapted).
- [ ] Confirm where `PlayerSpaceshipMarker` is defined and how it reaches the
      ship root, then attach both components to every player ship root -
      prefer `#[require(TargetLock, AvailableTargets)]` on the marker if it
      is ours; otherwise insert at the spawn/rig site. Verify in a test that
      a spawned player ship carries both defaults.
- [ ] Rewrite `update_spaceship_target_input` (targeting.rs:313): add
      `&mut TargetLock, &mut AvailableTargets` to the existing `spaceship`
      Single (targeting.rs:339-347), drop the two `ResMut` params
      (targeting.rs:349-350).
- [ ] Rewrite the remaining targeting.rs readers: `tick_lock_focus`
      (targeting.rs:606), `update_component_lock` (targeting.rs:665), and
      the four cycle observers (targeting.rs:802-803, 857-858, 874-875) to
      query the player ship instead of the resources.
- [ ] Migrate input/player.rs consumers: turret aim feed (:361), torpedo
      commit (:459), GOTO observer (:841) and the verb hint/controller reads
      (:232). Same query shape as above.
- [ ] Migrate the five HUD modules (hud/torpedo_target.rs,
      hud/target_candidates.rs, hud/edge_indicators.rs:262-263,
      hud/target_inset.rs, hud/component_lock.rs). GOTCHA: the resources
      always exist, the components only exist while a player ship does -
      HUD systems must degrade to their "no lock" path when the query is
      empty (Option<Single>/early return), matching today's None behavior.
- [ ] Delete the resource definitions (targeting.rs:72, :235) and their
      registrations (targeting.rs:91-92).
- [ ] Port the tests: replace `world.insert_resource(SpaceshipPlayerTarget*)`
      setups (e.g. targeting.rs:1025-1026, edge_indicators.rs:419-420, :462)
      with inserting the components on the spawned player ship; assertions
      stay byte-identical (this is the no-behavior-change proof).
- [ ] cargo fmt + cargo check, and run the touched test modules
      (nova_gameplay targeting + hud) - full suite runs in CI.

## Notes

- Spike: tasks/20260712-215733/SPIKE.md.
- Why components: the user wants the target computer to be entity state
  ("store the target and the available targets as components"); it unbakes
  the player-singleton assumption from 7 files and opens the door to AI
  ships running the same computer.
- Shape the components so a second slot (future travel/combat split, see the
  spike's "Combat vs travel separation" section) can be added later without
  re-plumbing every consumer.
- `SpaceshipPlayerLockFocus` and `SpaceshipPlayerComponentLock` deliberately
  STAY resources for now (spike open question; follow-up if it earns its
  keep).
- Keep the component names/Deref semantics close to the resources so the
  diff stays mechanical and reviewable.
- Ordering: land BEFORE 20260712-215402 (the unified-list behavior change
  builds on these components).

## Closure (2026-07-12, superseded - no code shipped)

Superseded by the travel/combat lock-slot model (spike
tasks/20260712-222610/SPIKE.md) before any code
was written. Componentization itself is still happening, but straight to
the end-state shape: task 20260712-223035 ports the resources directly to
TravelLock + CombatLock (+ AvailableTargets, HostileContacts) instead of a
neutral TargetLock that would need renaming one task later. The worktree
opened for this task (refactor/target-components) was removed untouched.
