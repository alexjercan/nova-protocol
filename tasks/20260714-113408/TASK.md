# Section catalog as data: assets/sections/*.ron loaded into GameSections via nova_modding

- STATUS: OPEN
- PRIORITY: 58
- TAGS: v0.6.0,modding,scenario,spike

Spike: tasks/20260714-110502/SPIKE.md

Goal (step 1 of the duplication direction): author the ~5 section prototypes
(`basic_controller_section`, `basic_hull_section`, turret, torpedo, thruster -
today built in `crates/nova_assets/src/sections.rs` `build_sections`) as
`assets/sections/*.ron` data, and load them into `GameSections` through a new
`nova_modding` catalog asset + loader (mirroring `ScenarioAsset`). Makes sections
moddable and is the reference target step 2 (20260714-113411) points at. Lowers to
the existing runtime `SectionConfig` - nothing downstream changes. `spike` until
planned.
