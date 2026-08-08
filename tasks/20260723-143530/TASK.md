# ch3 speed: expose player_speed as a reserved scenario variable (engine)

- STATUS: CLOSED
- PRIORITY: 60
- TAGS: v0.8.0, scenario, feature

## Story

Expose the PLAYER's live speed to scenario content as a reserved variable
`player_speed`, so any scenario can gate handlers on how fast the player is
moving. Today the expression engine can only read scenario variables, and the
only reserved one is `scenario_elapsed`; player velocity (avian3d
`LinearVelocity` on the player ship) is invisible to content. This is the
reusable engine half of the ch3 speed-provocation goal (umbrella
20260723-143503); the ch3 content consumes it in the sibling task.

Mirror the proven `scenario_elapsed` pattern exactly (loader.rs
`tick_scenario_clock` + `SCENARIO_ELAPSED_VAR`): a tracker system that runs
each live-unpaused frame, chained AHEAD of `fire_on_update` so the OnUpdate
pulse sees this frame's value, gated by `scenario_is_live && Unpaused`. Read
the player ship's `LinearVelocity.length()` (query is
`With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>` - the same scoping
`track_player_locks` already uses in this file). No player present => insert
0.0 (fail-closed, same as `scenario_elapsed`'s None -> 0.0). Freezing under
pause and resetting on teardown come for free from the shared gate + the
existing `world.clear()`.

## Steps

- [x] Add `PLAYER_SPEED_VAR: &str = "player_speed"` next to `SCENARIO_ELAPSED_VAR`
      in `crates/nova_scenario/src/loader.rs`, with a doc comment stating it is
      a reserved, engine-written variable (authors read it, never set it).
- [x] Add a `track_player_speed` system: query the player ship for
      `&LinearVelocity`, insert `player_speed = velocity.length() as f64`;
      insert 0.0 when there is no player ship. Import `LinearVelocity` from
      `avian3d::prelude` (avian3d 0.7 is already a nova_scenario dep).
- [x] Register it in `register_clock_and_pulse` chained as
      `(tick_scenario_clock, track_player_speed, fire_on_update).chain()` under
      the existing `scenario_is_live && Unpaused` run condition, so it shares
      one gate with the clock/pulse and cannot drift between plugin and rigs.
- [x] Lint exception: add `PLAYER_SPEED_VAR` alongside `SCENARIO_ELAPSED_VAR`
      in `crates/nova_scenario/src/lint.rs` (line ~304) so content that reads
      `player_speed` in an Expression is NOT flagged as an undefined variable.
      (It must never be flagged as a set-target either - it is engine-written;
      mirror whatever guard `scenario_elapsed` gets there.)
- [x] Harness test in nova_scenario (mirror the existing loader test rigs that
      register the real clock+pulse): a scenario-live App with a player ship
      carrying a `LinearVelocity`, pump a frame, assert
      `player_speed ~= velocity.length()`; set velocity to ZERO, pump, assert
      0.0; despawn the player, pump, assert 0.0; assert the value FREEZES while
      paused (does not update) - the fail-first proof that the gate holds.

## Definition of Done

- test: `cargo test -p nova_scenario` - new test proves `player_speed` tracks
  `LinearVelocity.length()`, reads 0.0 with no player, and freezes under pause.
- cmd: `cargo check` clean; the new system shares the one clock/pulse
  registration (no second run-condition site).
- The variable is documented as reserved/engine-written and lint-exempt from
  the undefined-variable rule, so the sibling ch3 content lints clean.

## Notes

Reference: `crates/nova_scenario/src/loader.rs` `tick_scenario_clock`
(~line 412), `register_clock_and_pulse` (~line 438), `track_player_locks`
(~line 634, the player-ship query scoping), `scenario_elapsed` (~line 427,
the None -> 0.0 fallback); `crates/nova_scenario/src/lint.rs:304` (the reserved
`scenario_elapsed` exception). `world_to_state_system` in world.rs is the
documented (currently empty) world->event injection hook - the tracker is the
scenario-side equivalent for player speed.
