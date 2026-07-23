# NOTES - expose player_speed as a reserved scenario variable

## What shipped

A second engine-owned reserved scenario variable, `player_speed`, alongside
`scenario_elapsed`. It holds the PLAYER ship's live speed in units/second (the
magnitude of its avian `LinearVelocity`) and is readable from any expression
filter as `Term(Factor(Name("player_speed")))`. This is the reusable engine
half of the ch3 speed-provocation goal (umbrella 20260723-143503); the ch3
content consumes it in task 20260723-143603.

### Diff surface (crates/nova_scenario/src)

- `loader.rs`
  - `PLAYER_SPEED_VAR = "player_speed"` const with a rustdoc contract mirroring
    `SCENARIO_ELAPSED_VAR` (engine-written, read-only for authors, fail-closed).
  - `is_reserved_engine_var(name)` - one predicate over both reserved vars, so
    the two lint rules that consume it cannot drift from each other.
  - `track_player_speed` system: player-scoped query
    (`With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>`, the same scoping
    `track_player_locks` uses) for `&LinearVelocity`; inserts
    `velocity.length() as f64`, or `0.0` when there is no player ship.
  - Registered inside `register_clock_and_pulse` as
    `(tick_scenario_clock, track_player_speed, fire_on_update).chain()` under
    the ONE existing `scenario_is_live && Unpaused` gate - so the pulse's speed
    gates see this frame's value, and pause-freeze / teardown-reset come for
    free from the shared gate + `world.clear()`. No second run-condition site.
  - Prelude re-exports `PLAYER_SPEED_VAR` next to `SCENARIO_ELAPSED_VAR`.
- `lint.rs` - both reserved-var rules now call `is_reserved_engine_var`:
  the undefined-variable rule EXEMPTS it (engine-set, needs no `VariableSet`),
  and the `VariableSet` rule ERRORS on an authored write (the engine overwrites
  it every frame). The write-error message generalized from "reserved engine
  clock" to "reserved engine variable '<key>'".

## Why this design (vs the alternatives)

The scenario expression engine already reads named variables; `scenario_elapsed`
is the proven reserved-variable precedent. Injecting `player_speed` the same way
reuses the entire Expression-filter machinery, needs no new event or filter
type, and makes the ch3 trigger pure content. The rejected alternatives (a
bespoke `PhysicsPropertyFilter`, or an `OnSpeedThreshold` event carrying the
speed) were both more machinery for less generality. Recorded in the umbrella
GOAL.md Decisions section.

`world_to_state_system` (world.rs) has a documented-but-empty world->event
injection hook; `track_player_speed` is the scenario-side equivalent, registered
the same way as the other clock-derived trackers rather than folded into that
hook, to stay on the established `register_clock_and_pulse` pattern.

## Verification

- `cargo test -p nova_scenario --lib player_speed` - two tests green:
  - `loader::tests::player_speed_var_tracks_live_velocity_and_fails_closed`:
    drives a player ship through 5 -> 0 -> 10 u/s and asserts the readout
    follows; a co-resident AI ship burning at 50 u/s the whole test never leaks
    (player-scope pin); pause FREEZES the readout even as the velocity changes
    underneath; despawning the player fails closed to 0.0. Built on the REAL
    `register_clock_and_pulse` registration (production-faithful-rigs), not a
    synthetic hand-seeded rig.
  - `lint::tests::player_speed_reads_are_clean_and_writes_are_errors`: gating on
    `player_speed` lints clean; a `VariableSet` onto it is an error (parity with
    the clock test; pins the second reserved key).
- fail-first A/B: with `track_player_speed` removed from the chain the tracker
  test FAILS (readout absent -> reads 0.0 -> the `== 5.0` assertion trips),
  proving the assertions exercise the mechanism (would-it-fail-without-it).
- `cargo check -p nova_scenario -p nova_assets` clean;
  `cargo doc -p nova_scenario --no-deps` warning-free.

No probe here: this variable is invisible in-game until a scenario consumes it;
the gameplay-visible behaviour (and its probe) lands with the ch3 content task.

## Reflection

Went smoothly - the `scenario_elapsed` precedent gave an exact template for
every piece (const, tracker, shared gate, both lint rules, the two test shapes).
The one snag was a rustdoc warning: a `pub const`'s doc intra-doc-linked the
private `track_player_speed` fn; downgraded to a plain code span, matching how
`SCENARIO_ELAPSED_VAR` refers to its private tick fn in prose. Lesson for next
time: a public item's rustdoc cannot `[link]` a private symbol without a
`cargo doc` warning - use a code span for private references.
