# Design / fix record - author-configurable orbit-hold & lock-refire durations

Task: 20260717-165031. Branch: feature/configurable-event-durations.
Follow-up from task 20260717-151537 (the user asked for it mid-cycle).

## What changed

Two new optional RON fields, both defaulting to the existing 5s, threaded to the
clock-derived event trackers via per-ship marker components:

- `AIControllerConfig.orbit_hold_secs: Option<f64>`
  (crates/nova_scenario/src/objects/spaceship.rs) -> inserted as the
  `OrbitHoldSecs(f64)` component on the ship at spawn (AI arm of
  `insert_spaceship_sections`) -> read by `track_orbit_holds` (loader.rs), which
  resolves `OrbitHoldSecs` else `ORBIT_HOLD_SECS`.
- `PlayerControllerConfig.lock_refire_secs: Option<f64>` -> inserted as
  `LockRefireSecs(f64)` on the player ship (Player arm) -> read by
  `track_player_locks`, which resolves it else `LOCK_REFIRE_SECS` and passes the
  window into `tick_lock_slot` (now takes a `refire_secs: f64` param).
- A shared `resolve_window_secs(override, default)` helper (loader.rs) fails
  closed: a non-finite or <= 0 override resolves to the default at runtime.
- content_lint (`check_controller_durations` in lint.rs): ERROR on a
  non-positive/non-finite `orbit_hold_secs`/`lock_refire_secs`; WARN when
  `orbit_hold_secs` is set with no `orbit` directive (can never take effect).
- Docs: web/src/wiki/dev/scenario-system.md (events table + object-kinds +
  override examples), CHANGELOG (Modding & Mod Portal).

## Why these locations

The data-model map (investigation at planning) drove the granularity:

- Orbit hold is PER-SHIP because the tracker measures per orbiting ship; the
  natural authoring home is the AI controller that already carries the `orbit`
  directive. A sibling field, NOT nesting into `orbit: Option<String>` - nesting
  would have been a breaking RON change to every `orbit: Some("id")` in the base
  and menu scenarios and the example mod. (The user's illustrative preview
  showed the nested form; the non-breaking sibling was chosen deliberately.)
- Lock re-fire is authored on `PlayerControllerConfig` (there is one player ship
  per scenario, so this is effectively scenario-scoped) instead of a new
  `ScenarioConfig` field. This was a mid-implementation pivot: a `ScenarioConfig`
  field would have broken ~15 struct literals across 5 crates (nova_scenario,
  nova_menu, nova_editor, nova_assets x2), whereas `PlayerControllerConfig` is
  symmetric with the orbit side and touched far fewer sites. It also mirrors the
  existing `speed_cap` -> `FlightSpeedCap` pattern exactly.
- Threaded via a scenario-side component rather than adding a field to
  nova_gameplay's `AutopilotAction::Orbit` enum - keeps the gameplay flight enum
  untouched; the trackers already iterate the ship entities.

## Decisions / tradeoffs

- **Default parity is the bar.** Omitted fields serialize/parse to None and
  resolve to 5.0, so no shipped scenario changes timing. All five exhaustive
  `PlayerControllerConfig` literals across the base scenarios got an explicit
  `lock_refire_secs: None` (the AI-config literals and one player literal use
  `..default()` and needed nothing).
- **Fail closed twice.** content_lint errors on a bad value (build-time gate),
  AND `resolve_window_secs` ignores it at runtime - a zero/negative window would
  otherwise fire every frame.
- **`tick_lock_slot` stays pure.** The window is a parameter, so the unit test
  drives custom cadences without any ECS.

## Bug caught along the way

`cargo check` passed but `cargo test` failed: a `#[cfg(test)]`
`AIControllerConfig { ... }` literal in spaceship.rs:449 was exhaustive and
missing the new field. `cargo check` does not compile `#[cfg(test)]` code, so
non-test literals are validated but test-only ones are not - grep ALL literal
sites (test modules included) when adding a struct field, don't trust check.
Fixed by adding `orbit_hold_secs: None`.

## Tests

- New: `orbit_hold_honors_a_per_ship_override` (loader.rs) - a 1s `OrbitHoldSecs`
  fires within ~1.2s where the 5s default is silent.
- New: `a_lock_slot_honors_a_custom_refire_window` (loader.rs) - a 2s window
  re-fires at 2s of hold.
- New: `non_positive_event_window_overrides_are_errors` and
  `orbit_hold_without_orbit_directive_warns` (lint.rs) - the fail-closed error
  and the never-takes-effect warn, plus a positive-override-lints-clean case.
- Existing lock/orbit tests updated for the new `tick_lock_slot` signature.
- `cargo test -p nova_scenario --features serde --lib` -> 101 passed, 0 failed.
  `cargo check -p nova_scenario -p nova_assets -p nova_editor` clean. `cargo fmt`
  clean. Ran with `--features serde` per lesson
  crate-solo-tests-miss-unified-features.
