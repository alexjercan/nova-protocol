# Spike: reduce RON duplication - prototype+modifications + multi-file bundles

- DATE: 20260714-110502
- STATUS: RECOMMENDED
- TAGS: spike, modding, scenario, refactor

## Question

The generated built-in RON is huge and repetitive: `shakedown_run.scenario.ron`
is ~1480 lines, mostly ships re-inlining full section configs. How should the
authoring format let a modder reference content by id and apply deltas, instead of
restating everything - without changing the runtime `ScenarioConfig` the loader
produces? A good answer names a concrete authoring model (references +
modifications + where catalogs live + bundle shape), stages it, and lowers cleanly
to the existing runtime types.

## Context

The RON format shipped (docs/modding-ron-format.md): scenarios are `ScenarioConfig`
serialized, with `AssetRef` for asset paths and `BindingInput` for bindings.
Runtime `ScenarioConfig` is the resolved target and must not change; all authoring
sugar is resolved at load in `nova_modding`.

The duplication is concrete. A ship's section is a full `SpaceshipSectionConfig`:

```
(
  id: "controller", position: (0,0,0), rotation: (0,0,0,1),
  config: (
    base: (id: "basic_controller_section", name: "Basic Controller Section",
           description: "A basic controller section for spaceships.", mass: 1.0, health: 100.0),
    kind: Controller(( verbs: (...), render_mesh: Some("gltf/..."), ... )),
  ),
)
```

The entire `config:` block is identical every time a ship places that section type
- across the two ships in `asteroid_field`, the two in `shakedown`, etc. The
section catalog already exists in code (`build_sections` /
`SectionMeshRefs`, `crates/nova_assets/src/sections.rs`) as ~5 named prototypes
(`basic_controller_section`, `basic_hull_section`, turret, torpedo, thruster); the
RON just doesn't reference it, it inlines a copy. Ships also don't yet apply any
per-section deltas - so today's dedup win is almost entirely "reference by id"; the
modification vocabulary is for the authoring flexibility the user wants next
(disable a verb, retune mass), not for reproducing current behavior.

## Options considered

### Axis A - how a ship references a section

- **A1 Prototype-by-id, no mods.** `(id: "controller", position: .., prototype:
  "basic_controller_section")`. Collapses the `config` block to one word. Biggest
  single win; trivial to lower (look up prototype, clone, set transform). Cons: no
  per-placement tweaks.
- **A2 Prototype-by-id + modifications (recommended).** A1 plus an optional
  `modifications: [..]` list of deltas. A small `Modification` enum with the ops the
  game actually has semantics for - `Rename(String)`, `SetMass(f32)`,
  `SetHealth(f32)`, `DisableVerb(FlightVerb)`, `SetRenderMesh(AssetRef)`,
  `SetBindings(Vec<BindingInput>)` - plus room to grow. Wesnoth's `[modifications]`/
  `[effect]` is the prior art: reference a unit, layer effects. Cons: a vocabulary
  to design and maintain; keep it a closed enum (pure data, wasm-safe), not a
  scripting escape hatch (that is the piccolo phase, 20260708-162010).
- **A3 serde `#[serde(default)]` field-omission (complementary, not a substitute).**
  Give config fields `Default` and `#[serde(default, skip_serializing_if)]` so an
  authored file omits unchanged fields. Cuts per-field noise (drop `health: 100.0`)
  even for non-ship content (asteroids, beacons), and needs no reference system.
  But it does NOT dedup the shared catalog - each ship still repeats the non-default
  fields. Cheap partial win; stack it under A2, do not rely on it alone.
- **A4 Do nothing.** The parity test keeps the generated files faithful, but they
  stay unauthorable by hand and grow with every ship. Rejected - the user asked.

### Axis B - where prototypes live

- **B1 Section catalog as data (recommended)**: `assets/sections/*.ron`, named
  `SectionConfig` prototypes, loaded into `GameSections` via `nova_modding`
  (replacing/supplementing the code `build_sections`). Scenarios reference catalog
  ids; a scenario may also define its own local prototypes that overlay the base
  catalog by id (scenario-local wins). Fully moddable.
- **B2 Keep the catalog in code**, reference ids into the code-built `GameSections`.
  Less loader work, but sections stay uncodeable by mods - contradicts "sections as
  RON files" and the injection goal.
- **B3 Whole-ship prototypes too**: a ship prototype (`assets/ships/*.ron` = a
  `SpaceshipConfig` template of prototype-referencing sections) that a scenario
  places by id with ship-level mods. Natural extension of B1 one level up; the
  Wesnoth `unit` analogy is really ship-level. Recommended as a second phase.

### Axis C - bundle shape

- **C1 Single file with an internal prototypes block.** Keep one `*.scenario.ron`
  but add a top-level `prototypes:`/`sections:` section the body references. No
  folder, no manifest. Simplest; dedups within a file but not across scenarios.
- **C2 Multi-file folder bundle (the user's "bundle of files").** A scenario is a
  directory with a manifest + separate files (sections, ships, objectives, events).
  The loader loads the folder, resolves ids across files with namespacing/overlay
  rules. Most modular, matches the long-term vision, but the heaviest: needs a
  directory `AssetLoader` (or a manifest that `load_context.load`s the parts),
  id-resolution order, and collision rules.
- **C3 Cross-file references via Bevy asset deps.** A scenario `load_context.load`s
  a shared `assets/sections/catalog.ron` and references its ids. Leverages Bevy's
  dependency loading; a middle point between C1 and C2 (shared catalog across
  scenarios without a full per-scenario folder).

## Recommendation

A **layered authoring model in `nova_modding` that lowers to the unchanged runtime
`ScenarioConfig`**, staged so the biggest dedup lands first and the heavy bundle
work is gated behind it:

1. **Section catalog as data (B1 + C3).** Author the ~5 section prototypes as
   `assets/sections/*.ron` and load them into `GameSections` through `nova_modding`;
   a scenario reaches the catalog as a shared asset dependency. This makes sections
   moddable and is the reference target for step 2.
2. **Prototype references + modifications for ship sections (A2, stacked on A3).**
   A ship section becomes `(id, position, rotation, prototype: "<catalog id>",
   modifications: [..])`; the authoring layer looks up the prototype, clones,
   applies the `Modification` deltas, and produces the runtime
   `SpaceshipSectionConfig`. Add `#[serde(default)]` field omission underneath to
   trim the remaining per-field noise. Re-port the built-in ships to references +
   regenerate; the parity test guarantees the lowered result is byte-identical to
   today's runtime configs. This is where shakedown collapses from ~1480 lines.
3. **Whole-ship prototypes (B3).** Ships as `assets/ships/*.ron` templates a
   scenario places by id with ship-level mods. Second phase, same mechanism one
   level up.
4. **Multi-file scenario bundles (C2).** A scenario as a folder + manifest, once the
   prototype/catalog model is proven. Its own spike - the id-namespacing, overlay,
   and directory-loader design is the real uncertainty and should not block steps
   1-3. KEY (user): files are TYPED and merged by kind - each file declares whether
   it is a ship / section / scenario / map / etc., and loading a bundle reads ALL
   files and routes each into the appropriate collection (Wesnoth's units/scenarios/
   maps model). The base game becomes just another bundle, and a mod is a bundle
   merged on top by kind - same shape. That tagging + merge-by-kind router, not the
   folder layout, is the heart of this step's spike (see 20260714-113418).

Why this beats the runners-up: A2 is what the user asked for and Wesnoth-proves;
B1/C3 make sections moddable without the full folder-bundle machinery; keeping
runtime `ScenarioConfig` fixed means the loader-side authoring layer is the only new
surface and nothing downstream changes. It stays pure-data/wasm-safe (a closed
`Modification` enum + a resolve pass, no code execution). The heavy, uncertain part
(multi-file bundles) is deferred behind a proof, not attempted first.

Dovetails with the editor scenario-builder (20260714-081703): the editor should
author and round-trip THIS model (references + mods), not the flat inlined form.

## Open questions

- **Modification vocabulary scope.** Start with the ops the game has semantics for
  (rename/mass/health/disable-verb/render-mesh/bindings). Which others are worth it
  before real modder demand? Resolve by authoring a few real scenarios.
- **Regeneration vs hand-authoring.** The current parity generator emits flat RON.
  Does it emit the referenced form (needs the generator to know the catalog), or is
  the referenced form hand-authored and the parity guard retired for ships? Decide
  when planning step 2.
- **Bundle id namespacing/overlay** (scenario-local vs catalog, collisions, load
  order) - the core of step 4's own spike.
- **Where the catalog loader lives** - extend `nova_modding` with a `SectionCatalog`
  asset + loader, mirroring `ScenarioAsset`. Confirm when planning step 1.

## Next steps

Direction-level tasks (for `/plan` to break into steps):

- tatr 20260714-113408: section catalog as data (`assets/sections/*.ron` + loader
  into `GameSections` via nova_modding).
- tatr 20260714-113411: prototype references + `Modification` model for ship
  sections in the authoring format; re-port built-in ships; `serde(default)` field
  omission.
- tatr 20260714-113414: whole-ship prototypes (`assets/ships/*.ron`) placed by id
  with ship-level mods (phase 2).
- tatr 20260714-113418 (spike): multi-file scenario bundles - folder + manifest, id
  namespacing/overlay, directory loader (gated on the above).

## Fix record

- 20260714, step 1 (113408) landed on master (`d85c4b0`): section catalog is now
  data. `nova_modding` gained a `SectionCatalogAsset` + `*.sections.ron` loader;
  `assets/sections/base.sections.ron` (7 prototypes, generator-guarded by
  `sections_ron_parity`) loads into `GameSections` via `register_sections`; runtime
  `SectionConfig` unchanged. Editor palette + boot verified live. Reviewed APPROVE
  (out-of-context). See tasks/20260714-113408/{TASK,REVIEW,RETRO}.md. Next: step 2
  (113411) makes ship sections REFERENCE this catalog by id - the big dedup.
- 20260714, step 2 (113411) landed on master (`32031e4`): ship sections reference
  catalog prototypes + component modifications. `SpaceshipSectionConfig` = { source:
  SectionSource(Inline|Prototype(id)), modifications: Vec<SectionModification> },
  resolved at spawn against GameSections. Modifications are COMPONENTS applied by
  On<Add> observers, inert where not applicable (user's model) - DisableVerb clears
  ControllerVerbs, SetHealth/Rename. Built-in ships re-ported to prototype refs
  (withheld verbs -> DisableVerb mods); Option/Vec/HashMap fields omit None/empty via
  serde. Regenerated RON shrank (asteroid_field 643->414, shakedown 1480->1238).
  Runtime behavior preserved (verb-withhold equivalence re-derived; 12_menu_newgame +
  09_editor green). SetMass + f32/bool serde domain-defaults deferred. Reviewed APPROVE
  (2 rounds, out-of-context). Next: 113414 (whole-ship prototypes) / 113418 (typed
  multi-file bundles). Follow-up spike 123535 (verb-availability from components).
