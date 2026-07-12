# Componentize targeting state: TargetLock + AvailableTargets on the ship root

- STATUS: OPEN
- PRIORITY: 60
- TAGS: v0.5.0, targeting, refactor, spike

## Goal

Move the player targeting state from global resources to components on the
ship root entity (`SpaceshipRootMarker` + `PlayerSpaceshipMarker`), per the
user's model: `SpaceshipPlayerTargetLock` -> a `TargetLock` component and
`SpaceshipPlayerTargetCandidates` -> an `AvailableTargets` component
(entries + pinned_until). Pure refactor: NO behavior change; every existing
test keeps passing with mechanical query rewrites only.

Migrate all consumers to query the player ship instead of the resources:
input/targeting.rs (acquisition + cycle observers), input/player.rs (turret
feed, torpedo commit, GOTO verb + hints), hud/torpedo_target.rs,
hud/target_candidates.rs, hud/edge_indicators.rs, hud/target_inset.rs,
hud/component_lock.rs.

## Notes

- Spike: docs/spikes/20260712-215733-unified-target-computer.md.
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
- Ordering: land BEFORE 20260712-215402 (the unified-list behavior change
  builds on these components).
