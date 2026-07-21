# Write documentation for nova_gameplay public API

- STATUS: CLOSED
- PRIORITY: 40
- TAGS: docs,v0.8.0

## Story

As a contributor extending gameplay (a new section kind, a weapon behavior, an
autopilot verb), I want nova_gameplay's public API documented in rustdoc, so
that I can find the right component/system/plugin from `cargo doc` or the IDE
instead of reading the whole crate.

nova_gameplay is the largest gameplay crate (sections, integrity/damage,
weapons, targeting, autopilot, AI) and the one modders and contributors touch
first. Retagged from the old backlog where it had no body; part of the
rustdoc strand coordinated by 20260525-133033 - follow the conventions that
task establishes.

## Steps

- [x] Crate-level `//!` doc: what nova_gameplay owns, its main plugins, and
      how it relates to nova_core / nova_scenario / bevy_common_systems (one
      screen, linking to the architecture wiki page for depth).
- [x] Document every public module with a one-paragraph `//!` header (what
      lives here, when you would touch it).
- [x] Document the public items contributors actually reach for first:
      plugins, the SectionKind surface and section config types, damage/
      resistance types, targeting/lock components, autopilot verbs and AI
      state types. Prefer doc comments that state units, invariants and
      cross-references over restating names.
- [x] Intra-doc-link related items (config type <-> runtime component <->
      plugin that registers it).
- [x] Verify `cargo doc -p nova_gameplay --no-deps` is warning-free; consider
      `#![warn(missing_docs)]` if the crate comes out clean (per the 133033
      enforcement decision). [Considered: NOT enabled - 191 public items
      still undocumented; see close-out.]

## Definition of Done

- Crate and every public module have doc headers; the main public types and
  plugins have doc comments with units/invariants where applicable.
- `cargo doc -p nova_gameplay --no-deps` builds warning-free and the rendered
  docs let a reader navigate from crate root to any major system in two
  clicks.

## Notes

- Do not duplicate the wiki (sections.md, architecture.md): rustdoc explains
  items, wiki explains systems - link across.
- Skip local full test/clippy runs per repo policy; CI covers them.

## Close-out (2026-07-21)

### What was documented

The crate already had a strong crate-level `//!` and extensive item docs from
the earlier rustdoc strand; this pass filled the structural gaps and the
first-reach items the task named.

- Crate `//!`: verified - already names what the crate owns, its composing
  plugin, the module surface and the `bevy_common_systems` relation. Left as-is.
- Module `//!` headers ADDED (all 9 public modules/mods that lacked one, plus
  the two private torpedo submodules for completeness):
  - `plugin.rs` (composition root, dependency assert),
  - `input/mod.rs`, `input/player.rs`, `input/ai.rs` (input producers),
  - `sections/base_section.rs`, `sections/torpedo_section/mod.rs`,
  - `integrity/explode.rs`, `camera_controller.rs`, `hud/mod.rs`,
  - `sections/torpedo_section/{projectile,render}.rs` (private, cheap wins).
  Every non-test `.rs` file in the crate now opens with a `//!`.
- Item `///` docs ADDED to the previously-undocumented first-reach items:
  - plugin: `NovaGameplayPlugin` (+ `render` field), `SpaceshipSystems` variants.
  - sections: `SectionKind` (+ each variant), `SectionConfig`, `BaseSectionConfig`
    (+ id/name/description/mass/health fields), `GameSections` (+ `get_section`),
    `SectionMarker`, `SectionInactiveMarker`, `SectionRenderOf`, `base_section`,
    `SpaceshipSectionSystems`, `SpaceshipSectionPlugin.render`,
    `ControllerSectionRenderMarker`, `ThrusterSectionRenderMarker`,
    `torpedo_section` fn, `TorpedoSectionPlugin` (+ field).
  - input: `SpaceshipInputSystems`, `SpaceshipInputPlugin`,
    `SpaceshipPlayerInputPlugin`, the three weapon `Spaceship*InputBinding`
    components, `SpaceshipAIInputPlugin`.
  - camera: `NovaCameraSystems`, `SpaceshipCameraControllerPlugin`,
    `SpaceshipCameraControlMode` variants, `SpaceshipRotationInputActiveMarker`.
  - integrity: `MeshFragmentMarker`.
  Cross-refs added between config <-> runtime component <-> registering plugin
  (e.g. `SectionKind` variant -> its config -> its section plugin; the weapon
  bindings -> `flight_rig_reserved_sources`).
- The damage, flight (incl. all autopilot verbs/phases), ammo, targeting/lock,
  turret, thruster, controller, hull config/component surfaces were AUDITED and
  found already fully `///`-documented - no changes needed there.

### cargo doc result

- `cargo doc -p nova_gameplay --no-deps`: WARNING-FREE (the only remaining
  cargo warnings are the pre-existing `proc-macro-error2` future-incompat note
  and the dirty-tree note, neither a doc warning).
- `RUSTDOCFLAGS="-D warnings" cargo doc -p nova_gameplay --no-deps`: CLEAN.
  This also fixed 2 PRE-EXISTING broken/private intra-doc-link warnings
  (`AmmoReadoutDebug` linked from a non-`debug`-gated module doc;
  `flight_input_rig` private-item link in `input/player.rs`) - both un-linked
  to plain backticks.

### missing_docs

NOT enabled. Under a temporary `#![warn(missing_docs)]` the crate emits 191
"missing documentation" warnings, so turning the lint on would (with the
workspace deny) break the build. The undocumented tail is concentrated in the
HUD widgets, audio, juice, gravity, relations, settings, beacon,
objective_marker and asset_ref modules, plus the internal (non-first-reach)
items of the priority subsystems. That tail is the breadth pass
(20260525-133032) + a future missing_docs rollout. Honest count to carry
forward: ~191 public items remain undocumented.

### Self-reflection

- The task brief assumed low coverage; the crate was in fact already
  well-documented at the item level from the earlier strand. The right move was
  to audit first (fanned out to read-only agents per subsystem) and target only
  the true gaps, rather than rewrite existing docs.
- The `-D warnings` strict run was essential: my new intra-doc links tripped
  `redundant_explicit_links` several times (a label that is itself a resolvable
  path makes the explicit `(path)` redundant) and one ambiguous `reference`
  module-vs-primitive link. All fixed by dropping the redundant target or using
  `mod@`. Lesson for next doc pass: prefer bare `` [`Type`] `` and only add an
  explicit `(path)` when the label alone does NOT resolve.
- Reviewer scrutiny: the docs I wrote from reading mechanism are the section
  config/component/marker set and the input bindings. Two spots stated from
  inference rather than a traced mechanism: (a) `BaseSectionConfig::mass` as
  avian DENSITY - lifted from the adjacent `SectionCollider` doc's own
  statement and the `destructible_body(health, density)` call, believed
  correct; (b) the `Spaceship*InputBinding` "snapshotted from content
  `input_mapping`" wording - inferred from the player module's flight-rig
  reservation note, not from reading the snapshot system. Worth a glance.
