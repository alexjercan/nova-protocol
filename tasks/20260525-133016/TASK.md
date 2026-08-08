# Add FPS and diagnostics overlay in example scenes

- STATUS: CLOSED
- PRIORITY: 55
- TAGS: v0.4.0, chore

Already partially done; finish coverage. Legacy #23.

## Resolution 2026-07-08

The FPS + version status-bar overlay is set up once in `nova_core::setup_status_ui`,
added by `AppBuilder::build()`, so every example that builds through `AppBuilder`
(01, 02, 03, 04, 06, 07, 07b, 08) and the editor example (09, via `editor_app`)
already showed it. The FPS diagnostics source (`FrameTimeDiagnosticsPlugin`) and the
`StatusBarPlugin` come in through `NovaGameplayPlugin` in that path.

The one gap was `examples/05_directional.rs`, which hand-builds its app from raw
`DefaultPlugins` + a few plugins instead of `AppBuilder`, so it never got the
overlay. Fixed by wiring it up explicitly there:

- add `FrameTimeDiagnosticsPlugin` (guarded by `is_plugin_added`) as the FPS source,
- add `StatusBarPlugin`,
- spawn `status_bar` + `status_bar_with_fps` + a version item on `Startup`.

Used the `bevy_common_systems` `status_bar_with_fps()` helper (no icon) rather than
the game's fps-icon item, since this standalone example does not load `GameAssets`.
All examples now render the FPS/version overlay. `cargo build --example 05_directional`
passes.
