# Review: Make Demo Mod Arena a playable target-destruction challenge

- TASK: 20260715-224812
- BRANCH: feat/arena-combat

## Round 1

- VERDICT: APPROVE

Diff reviewed against master (4f05adb3): the arena RON gameplay in
`assets/mods/demo/mod.content.ron` and the new
`crates/nova_assets/tests/arena_combat.rs`. Ran `arena_combat` (3/3) and
`demo_scenario` (11/11) in the worktree devshell - the section overlay and
`demo_mod_arena` presence still hold. Independently confirmed the load-bearing
claim the win depends on: `EventHandler::filter` is `filters.iter().all(...)`
(bevy_common_systems events.rs:131), so the win gate ANDs `destroyed==3` with
`arena_done==0` - `arena_done==0` alone cannot trip it (the
`win_gate_is_one_shot_and_needs_the_full_count` test falsifies the OR
hypothesis empirically too). The OnStart wiring is already pinned by
`onstart_spawns_the_player_targets_and_seeds_the_counter` (the lesson from
20260715-224803 applied up front). No blockers; two discretionary findings.

- [x] R1.1 (MINOR) assets/mods/demo/mod.content.ron - the win gate tests
  `destroyed == 3` (exact). If any target ever fired `OnDestroyed` more than
  once, `destroyed` would overshoot 3 and the `==3` frame would be skipped
  forever -> an uncompletable arena (soft-lock). Evidence says an asteroid fires
  OnDestroyed exactly once (nova_scenario
  `destroying_an_asteroid_node_fires_on_destroyed_for_the_root`; shakedown tallies
  use `==` in-game), so this is not a live bug - but this repo has repeated
  per-collider double-fire lessons (`collisionstart-is-per-collider-pair`), the
  failure is severe, and the fix is free: change the first win filter to
  `Greater(Term(Factor(Name("destroyed"))), Term(Factor(Literal(Number(2.0)))))`
  so an overshoot still wins.
  - Response: Applied (discretionary). Win gate first filter is now
    `GreaterThan(Term(Factor(Name("destroyed"))), Term(Factor(Literal(Number(2.0)))))`.
    arena_combat still 3/3.
- [x] R1.2 (NIT) assets/mods/demo/mod.content.ron - the arena beacon carries
  `area_radius: Some(40.0)` but no OnEnter/OnExit handler consumes it, so it
  spawns an inert sensor area. Harmless (it matches the original demo beacon),
  but dropping it (or a one-word comment that it is purely visual) avoids a
  future reader hunting for the trigger that uses it.
  - Response: Applied. Beacon `area_radius` set to `None` with a comment that it
    is purely a visual centrepiece.
