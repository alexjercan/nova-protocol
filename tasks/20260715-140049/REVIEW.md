# Review: e2e proof for SetSkybox: load a real cubemap and assert the swap

- TASK: 20260715-140049
- BRANCH: test/skybox-swap-e2e

## Round 1

- VERDICT: APPROVE

Diff: one new integration test (`crates/nova_scenario/tests/skybox_swap_e2e.rs`)
plus the TASK.md Steps/Notes. No production code touched. The test compiles and
passes (`1 passed`, ~5m03s cold build in the worktree).

Verified independently (not just read):
- **Falsifiability.** The final assertion (`Skybox.image == swapped`) requires
  BOTH bridges to run: `apply_pending_skybox_swaps` is the only inserter of
  `SkyboxConfig` after spawn, and bcs `setup_skybox_camera` is the only inserter
  of `Skybox` (confirmed in the locked bcs rev 4c81117 `src/camera/skybox.rs`).
  Delete either and the image stays `initial`, so the wait loop times out and
  panics. This is a real regression pin, not a copy that cannot fail.
- **No vacuous pass.** `swapped != initial` is asserted explicitly (distinct
  paths -> distinct AssetIds), and the pre-swap assertion pins the camera on
  `initial` first, so the swap assertion proves an actual change - the delivery
  guard the "nothing happens" lesson asks for, inverted to "the change happened".
- **Real assets.** Loads the shipped `textures/cubemap.png` and
  `textures/cubemap_alt.png` off the real workspace `assets/` (headless IO,
  modeled on demo_scenario.rs / cubemap_meta.rs).
- **Real flush path.** The swap is fired through the real
  `SetSkyboxActionConfig::action` on the real `NovaEventWorld`, and drained by
  the real `GameEventsPlugin::<NovaEventWorld>` PostUpdate chain
  (`resource_changed` gate) - not a hand-rolled command drain.

- [ ] R1.1 (NIT) crates/nova_scenario/tests/skybox_swap_e2e.rs:122 - the action
  is invoked directly rather than through a fired `GameEvent` ->
  `EventActionConfig::SetSkybox` dispatch. That dispatch is generic bcs
  machinery (tested upstream) and the RON round-trip is covered by
  `set_skybox_action_round_trips_through_ron`, so this is in-scope for the
  applier->observer gap the task targets. Left as a considered scope decision;
  a fuller "fire mid-live-scenario" e2e could be a separate follow-up.
- [ ] R1.2 (NIT) same file - the load-FAILURE branch of
  `apply_pending_skybox_swaps` (bad path -> drop the swap, sky unchanged) is not
  exercised here. Out of scope for this task (happy-path e2e); worth a small
  follow-up test if that branch ever regresses.

Neither NIT blocks. The diff delivers the Goal; approving.
