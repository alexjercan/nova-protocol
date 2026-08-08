# Add optional serde feature + derives to nova_scenario config types

- STATUS: CLOSED
- PRIORITY: 80
- TAGS: v0.6.0, modding, scenario, foundation

Data-driven scenario definition. Legacy #101.

Spike: tasks/20260708-161726/SPIKE.md (direction)
Spike: tasks/20260714-083224/SPIKE.md (detailed design + type audit)

Phase 1 of the modding-language direction, FOUNDATION of the v0.6.0 sprint. Scope
NARROWED after the detailed spike: this task is the mechanical half - add
`serde::{Serialize, Deserialize}` derives across the whole config tree (full type
list with file:line in the detailed spike; all types already derive `Clone, Debug`,
a subset `Reflect`, none serde today) - and, once the AssetLoader/authoring layer
(20260714-083326) exists, port the built-in scenarios (`asteroid_field`,
`asteroid_next`, `menu_ambience`, `shakedown`) out of
`crates/nova_assets/src/scenario.rs` + `scenario/shakedown.rs` into
`assets/scenarios/*.ron` to dogfood the format.

The design-heavy `*.scenario.ron` AssetLoader + asset-path->Handle authoring layer
moved to its own task (20260714-083326), because the asset-handle fields
(`cubemap`, `AsteroidConfig.texture`, `BeaconRenderConfig.color`) need real design.
RON chosen over a bespoke DSL / KDL / TOML / JSON because it maps onto the existing
Rust enums for free. Scripting (Lua/piccolo) is a separate later phase (backlog).
