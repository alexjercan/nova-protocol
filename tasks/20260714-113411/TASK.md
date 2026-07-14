# Prototype references + Modification model for ship sections; re-port built-in ships; serde default omission

- STATUS: OPEN
- PRIORITY: 56
- TAGS: v0.6.0,modding,scenario,spike

Spike: tasks/20260714-110502/SPIKE.md

Goal (step 2, the big dedup): let a ship section reference a catalog prototype by
id and apply deltas instead of inlining the full config. Authoring form:
`(id, position, rotation, prototype: "<catalog id>", modifications: [..])`. Add a
closed `Modification` enum (Rename/SetMass/SetHealth/DisableVerb/SetRenderMesh/
SetBindings, room to grow - pure data, no scripting). The nova_modding authoring
layer resolves prototype -> clone -> apply mods -> runtime `SpaceshipSectionConfig`.
Stack `#[serde(default)]` field omission underneath to trim per-field noise. Re-port
the built-in ships to references and regenerate; the parity test proves the lowered
result is byte-identical to today's configs (this is where shakedown collapses from
~1480 lines). Gated on the catalog (20260714-113408). `spike` until planned.
