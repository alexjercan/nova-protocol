# Scenario language/config format

- STATUS: OPEN
- PRIORITY: 0
- TAGS: v0.6.0,objectives,modding

Data-driven scenario definition. Legacy #101.

Spike: docs/spikes/20260708-161726-modding-language-and-scripting.md

Phase 1 of the modding-language direction: add `serde` to the existing config
model (`ScenarioConfig`/`EventConfig`/`EventActionConfig`/`EventFilterConfig` and
the `variables.rs` AST, which already derive `Reflect`), write a `*.scenario.ron`
Bevy `AssetLoader`, and port the built-in scenarios out of
`crates/nova_assets/src/scenario.rs` into `assets/scenarios/*.ron` to dogfood it.
RON chosen over a bespoke DSL / KDL / TOML / JSON because it maps onto the
existing Rust enums for free. Scripting (Lua/piccolo) is a separate later phase,
see the spike.
