# Review: SetSkybox action (swap the scenario cubemap mid-scenario)

- TASK: 20260525-133017
- BRANCH: skybox-action

## Round 1

- VERDICT: APPROVE

Scope: a new `EventActionConfig::SetSkybox` swapping the scenario skybox cubemap,
authored via the `AssetRef` path layer, with a deferred install. Ran the checks:
`cargo check --workspace` green; `nova_scenario` lib tests pass (62) plus the two
new tests (`skybox_swap_waits_for_load_then_installs`,
`set_skybox_action_round_trips_through_ron`); fmt clean; clippy adds no new
warnings (the ones present are pre-existing in loader.rs/spaceship.rs).

Independently verified the load-bearing correctness claim rather than trusting
the summary: the risk is bcs's `setup_skybox_camera` (`On<Insert, SkyboxConfig>`)
doing `images.get_mut(&config.cubemap).unwrap()` - a panic on an unloaded image.
The action never inserts `SkyboxConfig` directly; `apply_pending_skybox_swaps`
inserts it only once `images.contains(&handle)` is true, which is exactly the
precondition that makes bcs's `get_mut().unwrap()` safe. Re-inserting
`SkyboxConfig` re-fires the same observer that set the initial skybox (reinterpret
guarded by `array_layer_count() == 1`, then a fresh `Skybox`), so the swap path is
the proven initial-setup path, not new rendering code.

No BLOCKER/MAJOR. Findings below are discretionary.

- [ ] R1.1 (MINOR) crates/nova_scenario/src/actions.rs (test) - the unit test
  deliberately omits bcs's `SkyboxPlugin`, so it proves the defer/brightness
  logic but not the end-to-end visible swap through bcs's observer with a real
  cubemap. That path is reuse of the proven initial-setup path (verified by
  reading the observer), so this is not a blocker, but an example or headless
  integration test that loads a real `*.cube.png` and asserts the camera's
  `Skybox.image` changed would close the gap. Reasonable follow-up.
  - Response: Accepted as a follow-up task (the risky logic is tested and the
    observer path is verified by reading, so not a blocker for this small hook).
    Filed for the e2e/example proof.

- [x] R1.2 (NIT) crates/nova_scenario/src/actions.rs:apply_pending_skybox_swaps -
  a `PendingSkyboxSwap` built in code with a bare `reserve_handle()` (never added
  to `Assets`, never server-loaded) would wait forever: `contains` stays false and
  `load_state` is `NotLoaded`, not `Failed`. Not reachable through the action
  (which always `resolve()`s to a server load, so a bad path becomes `Failed` and
  is dropped), so acceptable; worth a one-line note on the component that the
  applier only drops on a *server* failure.
  - Response: Fixed. `apply_pending_skybox_swaps` doc now spells out the drop is on
    a server-reported failure and that the action always resolves through a server
    load, so every real swap is covered.

- [x] R1.3 (NIT) two `SetSkybox` beats firing close together both start their
  loads, but the second `insert` replaces the first `PendingSkyboxSwap`, so the
  first load's result is silently ignored (last-write-wins). This is the sensible
  behavior; noting it so it is a decision, not an accident.
  - Response: Confirmed intended (last-write-wins is the natural semantics for a
    "set the sky to X" beat). No change.

### Round 1 close

Verdict stands: **APPROVE**. R1.2/R1.3 resolved in-session; R1.1 (e2e proof)
filed as a follow-up task. No open BLOCKER/MAJOR.
