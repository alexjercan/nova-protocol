# e2e proof for SetSkybox: load a real cubemap and assert the swap

- STATUS: CLOSED
- PRIORITY: 15
- TAGS: v0.6.0, modding, test

Follow-up from 20260525-133017 review R1.1. The `SetSkybox` action's deferred
applier logic is unit-tested (`skybox_swap_waits_for_load_then_installs`), but the
end-to-end path through bcs's `SkyboxPlugin` observer (re-insert `SkyboxConfig` ->
new `Skybox` with the swapped cubemap) is not exercised in a nova test - it relies
on reading that the observer reuses the initial-setup path.

Add an example or headless integration test that loads a real `*.cube.png`, fires
`SetSkybox` mid-scenario, ticks until the load lands, and asserts the scenario
camera's `Skybox.image` handle changed. Model on `nova_assets/tests/demo_scenario.rs`
(real headless asset loading). Small; strengthens the modding hook to full e2e.

## Steps
- [x] Add headless integration test in nova_scenario/tests that loads a real
      cubemap and drives SetSkybox through the full chain (action -> flush ->
      PendingSkyboxSwap -> load -> applier -> bcs SkyboxPlugin observer).
- [x] Assert the scenario camera's live `Skybox.image` handle swapped, brightness
      inherited, pending tag consumed. Use the shipped `textures/cubemap_alt2.png`.
- [x] cargo fmt + run the new test green.

## Notes
- Uses cubemap_alt2.png (added this session) as the swap target; cubemap.png as
  the initial sky. Both load as 6-layer arrays via their .meta.
- Applier registered ungated in the rig (production gates it on scenario_is_live,
  an orthogonal scheduling concern).

## Outcome (CLOSED)

Added `crates/nova_scenario/tests/skybox_swap_e2e.rs`: a headless integration test
that fires the real `SetSkybox` action for `textures/cubemap_alt2.png` and asserts
the scenario camera's live `Skybox.image` swaps (brightness inherited, pending tag
consumed), driving the previously-untested `apply_pending_skybox_swaps` ->
bcs `SkyboxPlugin` observer bridge on real assets. Test green (1 passed); fmt clean.
Review APPROVEd R1 (two NITs, non-blocking: direct action() invocation vs event
dispatch, and the load-failure branch left for a follow-up).
