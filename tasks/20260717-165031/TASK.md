# Author-configurable orbit-hold and lock-refire durations (default 5s)

- STATUS: CLOSED
- PRIORITY: 42
- TAGS: v0.7.0,scenario,modding,feature

## Goal

The two clock-derived scenario-event windows are hardcoded 5s engine constants
(`ORBIT_HOLD_SECS`, `LOCK_REFIRE_SECS` in crates/nova_scenario/src/loader.rs,
task 20260717-151537). Let scenario/mod authors override each in RON, defaulting
to 5.0 when omitted (no existing content changes behavior):

- orbit-hold before `OnOrbit` fires: PER-SHIP, on the AI controller that authors
  the orbit.
- player travel/combat lock re-fire period: on `PlayerControllerConfig` (there
  is one player ship per scenario, so this is effectively scenario-scoped).
  NOTE: originally planned on `ScenarioConfig`; pivoted during implementation -
  see the Design note.

Follow-up from task 20260717-151537's retro (user-requested during that cycle).

## Design (decided from the data-model map)

- Orbit hold is authored as a NON-BREAKING sibling field `orbit_hold_secs:
  Option<f64>` on `AIControllerConfig` (crates/nova_scenario/src/objects/spaceship.rs),
  NOT by nesting it into the existing `orbit: Option<String>` (which would break
  every `orbit: Some("id")` authoring in the base/menu scenarios and the example
  mod). It mirrors the existing `leash: Option<f32>` / `speed_cap: Option<f32>`
  optional-field pattern.
- Thread the per-ship override to `track_orbit_holds` via a small scenario-side
  marker component, NOT by adding a field to `nova_gameplay`'s
  `AutopilotAction::Orbit` enum (keeps the gameplay flight enum untouched; the
  tracker already iterates ship entities and can query one more optional
  component).
- Lock re-fire is authored as `lock_refire_secs: Option<f64>` on
  `PlayerControllerConfig`, threaded via a `LockRefireSecs(f64)` component on the
  player ship (mirroring `speed_cap` -> `FlightSpeedCap`) and read by
  `track_player_locks`. PIVOT: originally planned as a `ScenarioConfig` field
  read via `CurrentScenario`, but that field breaks ~15 `ScenarioConfig` struct
  literals across 5 crates. `PlayerControllerConfig` is symmetric with the orbit
  side, touches far fewer sites, and is still effectively scenario-scoped (one
  player ship per scenario).
- Both use the repo's optional-field serde attr:
  `#[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]`.
- Fail closed on nonsense values: a non-positive override is rejected by
  content_lint (see the lint step); at runtime clamp defensively to the 5.0
  default so a bad value never produces a zero/negative window.

## Steps

- [x] Add `pub orbit_hold_secs: Option<f64>` to `AIControllerConfig`
      (crates/nova_scenario/src/objects/spaceship.rs, next to `orbit`/`leash`) with
      the optional-field serde attr and a doc comment (seconds a ship must hold the
      orbit before `OnOrbit` fires; None = engine default 5s).
- [x] Add a scenario-side component carrying the per-ship override, e.g.
      `#[derive(Component, Reflect)] pub struct OrbitHoldSecs(pub f64);` in
      loader.rs (or spaceship.rs). Register the type.
- [x] In the AI spawn arm of `insert_spaceship_sections`
      (crates/nova_scenario/src/objects/spaceship.rs ~310-321, where
      `AIOrbitDirective` is inserted), also insert `OrbitHoldSecs` when
      `config.orbit_hold_secs` is Some. Only meaningful alongside `orbit`.
- [x] In `track_orbit_holds` (loader.rs), add `Option<&OrbitHoldSecs>` to the ship
      query and use `override.map(|o| o.0).unwrap_or(ORBIT_HOLD_SECS)` as the window
      threshold; keep `ORBIT_HOLD_SECS` as the default constant. Guard: treat a
      non-finite or <= 0 override as the default (defensive; lint is the real gate).
- [x] Add `pub lock_refire_secs: Option<f64>` to `PlayerControllerConfig`
      (spaceship.rs) + a `LockRefireSecs(f64)` component inserted in the Player
      spawn arm (PIVOT from `ScenarioConfig`; see Design). Register the type.
- [x] In `track_player_locks` (loader.rs), query `Option<&LockRefireSecs>` on the
      player ship and pass the resolved window into `tick_lock_slot` instead of the
      bare `LOCK_REFIRE_SECS` constant. `tick_lock_slot` takes the window as a
      `refire_secs: f64` param so it stays pure/testable; default resolves to
      `LOCK_REFIRE_SECS` via `resolve_window_secs` (shared <=0/non-finite guard).
- [x] content_lint (crates/nova_scenario/src/lint.rs): error on an authored
      `orbit_hold_secs` or `lock_refire_secs` that is <= 0 or non-finite (fail
      closed, mirroring the existing lint style). Optionally warn if
      `orbit_hold_secs` is set on a controller with no `orbit`. Add a lint unit
      test for the reject path.
- [x] Tests (crates/nova_scenario/src/loader.rs): extend
      `orbit_hold_fires_once_per_window_and_recurs` (or add a sibling) to spawn a
      ship with `OrbitHoldSecs(2.0)` and assert it fires on the SHORTER window while
      a default ship still fires at 5s. Extend the lock test to assert a scenario
      `lock_refire_secs` override changes the echo cadence. Assert the DEFAULT path
      (None -> 5.0) is unchanged.
- [x] Docs: update web/src/wiki/dev/scenario-system.md (and/or
      guide-author-scenario.md) with the two new optional RON fields, their
      defaults, and a short example. Add a CHANGELOG.md entry (Modding & Mod Portal
      or Scenarios & Objectives) and write tasks/20260717-165031/NOTES.md.
- [x] Verify: `cargo fmt`, `cargo check -p nova_scenario`, and run the module tests
      with the serde feature: `cargo test -p nova_scenario --features serde --lib`
      (crate-solo runs miss serde unification - lesson
      crate-solo-tests-miss-unified-features). Skip the full local clippy/test
      sweep per repo policy; CI runs it.

## Notes

- Data-model map (verified by investigation during planning):
  - `AutopilotAction::Orbit { well, plan }` at crates/nova_gameplay/src/flight.rs:171
    (do NOT touch - thread via a scenario component instead).
  - `AIControllerConfig` at crates/nova_scenario/src/objects/spaceship.rs:56-85
    (`orbit: Option<String>`, `leash: Option<f32>` - the pattern to mirror).
  - RON->config->`AIOrbitDirective` conversion in the AI arm of
    `insert_spaceship_sections`, spaceship.rs:310-321.
  - `track_orbit_holds` reads `AutopilotAction::Orbit` at loader.rs ~461, window
    compare at ~471, `ORBIT_HOLD_SECS` const at ~413.
  - `ScenarioConfig` fields at loader.rs:83-127 (no existing constants block;
    mirror `thumbnail`/`hidden` optional-field pattern).
  - `track_player_locks` at loader.rs:563-605; `tick_lock_slot` at ~526 with the
    window compare at ~541; `LOCK_REFIRE_SECS` const at ~510.
  - Both constants are referenced ONLY in loader.rs.
- Backward/forward compatible: `serde(default, skip_serializing_if = ...)` means
  pre-existing scenarios parse to None and behave exactly as today.
- The user's illustrative preview nested `hold_secs` inside `orbit`; this plan
  deliberately uses a non-breaking sibling field instead (documented above).
- Behavior parity for the DEFAULT (omitted) path is a hard acceptance bar: no
  shipped scenario should change timing.
