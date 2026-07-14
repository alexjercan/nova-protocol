# Spike: reduce RON duplication - prototype+modifications model + multi-file scenario bundles

- STATUS: OPEN
- PRIORITY: 60
- TAGS: v0.6.0,modding,scenario,spike,refactor

Spike only - do NOT implement. Follow-up to the RON format landing
(133028, docs/modding-ron-format.md).

## Problem

The generated built-in RON is heavily duplicated. Every spaceship inlines full
`SpaceshipSectionConfig`s, and every section repeats `name`, `description`, mass,
health, render meshes, etc. `shakedown_run.scenario.ron` is ~1480 lines / 55 KB and
`asteroid_field.scenario.ron` ~640 lines largely because two ships each re-list
their whole section catalog. A hand-authoring modder should never copy that. The
config tree serializes today, but it is not yet a pleasant *authoring* surface.

## What the user wants explored

1. **Prototype + modifications (Wesnoth-style).** Reference a thing by id and apply
   deltas instead of restating it. E.g. a `controller` section prototype exists once;
   a ship says "use `controller`, with modifications: disable the ORBIT verb" rather
   than inlining the whole `ControllerSectionConfig`. Same for whole ships (a
   `pirate` ship prototype + tweaks). Design the modification vocabulary (set-field,
   disable-verb, override-binding, add/remove section, ...) and how it composes.
2. **Sections (and other content) as their own RON files.** A section catalog authored
   as data (`sections/*.ron`), not built in `crates/nova_assets/src/sections.rs`.
   Scenarios reference catalog sections by id; a scenario may ALSO define its own
   sections and inject them into the game (scenario-local prototypes overlaying the
   base catalog).
3. **Scenarios as multi-file bundles.** Rather than one monolithic
   `*.scenario.ron`, a scenario becomes a folder/bundle of files that inject sections,
   ships, levels, objectives, events, etc. - "a scenario is a bundle of RON files (or
   whatever format we pick later)". Explore the bundle layout, a manifest, how the
   `AssetLoader` loads a directory, id resolution/namespacing across files, and load
   order/overlay rules.

## What a good answer produces

- A recommended authoring model (prototype registry + modification ops + reference-by-id)
  and how it lowers to the existing runtime `ScenarioConfig`/`SpaceshipConfig`/
  `SectionConfig` (the runtime tree need not change - this is an authoring layer on top,
  resolved at load in `nova_modding`).
- A recommended bundle/multi-file layout + manifest + a Bevy folder/multi-asset loading
  approach, and how ids resolve across catalog vs scenario-local definitions.
- A migration note: how the current flat generated RON files map onto the new model, and
  whether the format stays RON or a friendlier surface is warranted (revisit the KDL /
  custom-DSL option the modding-direction spike 20260708-161726 parked).
- Prior art to weigh: Wesnoth WML (units + `[modifications]`/`[effect]`), Bevy asset
  preprocessing / `LoadContext` labeled sub-assets and dependency loading, and bevy
  scene/prefab-style overlays.
- Seed the implementation tasks (do not build them here).

## Constraints

- Runtime `ScenarioConfig` stays the resolved target; the authoring model is a layer
  that lowers to it, so nothing downstream of the loader changes.
- Keep it wasm-safe and pure-data (no code execution - scripting is the separate piccolo
  phase, 20260708-162010).
- Dovetails with the editor scenario-builder (20260714-081703): whatever authoring model
  wins should be what the editor reads/writes.

Related: docs/modding-ron-format.md, tasks/20260714-083224 (RON detailed design),
tasks/20260714-091336 (crate boundary), tasks/20260714-103622 (ScatterObjects - an
example of a declarative primitive that cut duplication).
