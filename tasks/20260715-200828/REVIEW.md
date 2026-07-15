# Review: Scenario picker

- TASK: 20260715-200828
- BRANCH: feature/scenario-picker

## Round 1

- VERDICT: APPROVE

Reviewed the branch diff against master with fresh eyes plus an independent
out-of-context pass (a second reviewer re-derived the load-bearing claims from
the code, not the summary). Both passes agree: the feature is correct, the
tests are non-vacuous, and no BLOCKER or MAJOR issues exist. Load-bearing
claims verified:

- Play handoff has no stale-override path: `NewGameScenario`'s only production
  writers are `on_new_game` (clears to None) and `on_scenario_play` (sets
  Some), and every production route to `Playing + NewGame` goes through one of
  them; `start_new_game_scenario` is the sole reader, gated on
  `GameMode::NewGame`. Editor `insert_resource(GameMode::NewGame)` sites are all
  `#[cfg(test)]`. Missing-id override warns and falls back, never panics.
- Same-frame refresh chain holds: the chained details refresh's run condition
  sees the list refresh's `SelectedScenarioId` write via change detection in the
  same schedule run (the established mods-screen pattern; proven by the
  default-select+details integration test that populates in one `update()`).
- No infinite refresh loop: the selection write is doubly guarded
  (`if !listed.any(...)` then `if selected.0 != first`), self-terminating.
- Thumbnail renders in production (DefaultPlugins provides `AssetServer`); the
  `Option<Res<AssetServer>>` guard only lets headless tests skip it.
- Tests would each fail if their mechanism were deleted (filter, override,
  serde defaults) - checked individually.

Findings (both non-blocking, addressed on-branch since this cycle merges):

- [x] R1.1 (NIT) crates/nova_menu/src/lib.rs:3506 - the
  `overlay_roots_carry_an_explicit_z_index` regression test pinned `ModsPanel`
  and `SettingsPanel` but not `ScenariosPanel`, so the new overlay's
  `GlobalZIndex(1)` was only screenshot-verified. Added `ScenariosPanel` to the
  test's list so dropping its z-index is caught.
  - Response: fixed - `ScenariosPanel` added to the `[(name, root)]` loop; the
    test now pins all three overlay roots.
- [x] R1.2 (MINOR) crates/nova_scenario/src/loader.rs:26 - `derive(Default)` on
  `ScenarioConfig` makes a fully-default config (handle-backed default
  `cubemap`) non-serializable; unreachable today but a latent footgun.
  - Response: fixed - added a struct doc note warning not to serialize
    `ScenarioConfig::default()` directly and why every real builder sets a path
    cubemap.
