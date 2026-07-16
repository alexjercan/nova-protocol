# Gate the OnUpdate scenario pulse (fire_on_update) on Unpaused so it stops during any pause

- STATUS: OPEN
- PRIORITY: 40
- TAGS: v0.7.0,scenario,polish

## Goal

`fire_on_update` (crates/nova_scenario/src/loader.rs:252) fires the `OnUpdate`
scenario pulse every frame gated only on `scenario_is_live`, NOT on
`PauseStates::Unpaused`. So while the game is paused (the ESC pause menu OR the
new outcome frame, task 20260716-214919), the pulse keeps firing and any
`OnUpdate` handler action still applies via the pause-independent PostUpdate
`state_to_world_system`. In practice the world values those handlers key on are
frozen so nothing NEWLY fires, but a handler whose predicate is already true
re-runs its action every frame under pause. This is a pre-existing gap shared
by both pause paths (surfaced by the outcome-pause review R1.1).

## Direction

- Gate the pulse on the pause state as well:
  `fire_on_update.run_if(scenario_is_live.and(in_state(PauseStates::Unpaused)))`
  (nova_scenario already depends on `PauseStates` - `on_next_input` reads it).
- Check the other `scenario_is_live`-only Update systems in the same plugin
  (`track_orbit_holds`, `track_player_locks`, `apply_pending_skybox_swaps`,
  loader.rs:259-273) - decide per system whether it should also freeze under
  pause (most track world state that is frozen anyway; skybox swap is cosmetic).
  Do not blanket-gate; walk each one.
- Pin with a test: an OnUpdate handler whose predicate is already true does NOT
  re-fire its action while `Paused` (delivery-guarded: it DOES fire while
  Unpaused). `messagereader-needs-resource-guard-in-tests` /
  `run-system-once-always-changed` apply - use an App-driven rig across frames.

## Notes

- Discovered during task 20260716-214919 review (see its REVIEW.md R1.1).

