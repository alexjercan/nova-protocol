# e2e proof for SetSkybox: load a real cubemap and assert the swap

- STATUS: OPEN
- PRIORITY: 15
- TAGS: v0.6.0,modding,test


Follow-up from 20260525-133017 review R1.1. The `SetSkybox` action's deferred
applier logic is unit-tested (`skybox_swap_waits_for_load_then_installs`), but the
end-to-end path through bcs's `SkyboxPlugin` observer (re-insert `SkyboxConfig` ->
new `Skybox` with the swapped cubemap) is not exercised in a nova test - it relies
on reading that the observer reuses the initial-setup path.

Add an example or headless integration test that loads a real `*.cube.png`, fires
`SetSkybox` mid-scenario, ticks until the load lands, and asserts the scenario
camera's `Skybox.image` handle changed. Model on `nova_assets/tests/demo_scenario.rs`
(real headless asset loading). Small; strengthens the modding hook to full e2e.
