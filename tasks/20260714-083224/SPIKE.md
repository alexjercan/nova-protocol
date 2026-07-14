# Spike: RON scenario format detailed design + scenario-engine optimizations

- DATE: 20260714-083224
- STATUS: RECOMMENDED
- TAGS: spike, modding, scenario, perf

## Question

The v0.6.0 direction spike (`tasks/20260714-081636/SPIKE.md`) committed the sprint
to phase-1 RON modding + "optimizations for it". This spike drills one level down:
what exactly does the RON format need to serialize (and where does it get hard),
and are the seeded optimizations actually worth doing / what else is there? Output:
a concrete, code-grounded design that breaks the format work into plannable tasks
and re-scopes the optimization work so it is measured, not guessed.

## Context

Scenario config lives in `crates/nova_scenario/src/` and is built programmatically
in `crates/nova_assets/src/scenario.rs` + `scenario/shakedown.rs` into a
`GameScenarios` resource. Nothing is serialized today. Full type audit below.

### Config type tree and current derives

All of these derive `Clone, Debug`; a subset also `Reflect`; NONE derive `serde`:

- `ScenarioConfig` (loader.rs:26), `ScenarioEventConfig` (loader.rs:40)
- `EventConfig` (events.rs:11) - also `Copy, Reflect`
- `EventActionConfig` (actions.rs:21) + `VariableSetActionConfig` (actions.rs:88);
  `ScenarioObjectConfig` (actions.rs:514), `BaseScenarioObjectConfig` (actions.rs:520),
  `ScenarioObjectKind` (actions.rs:947), `ScenarioAreaConfig` (actions.rs:981)
- `EventFilterConfig` (filters.rs:13), `EntityFilterConfig` (filters.rs:30),
  `ConditionalFilterConfig` (filters.rs:103), `ExpressionFilterConfig` (filters.rs:138)
- `variables.rs` AST: `VariableLiteral` (:19), `VariableFactorNode` (:26),
  `VariableTermNode` (:58), `VariableExpressionNode` (:117), `VariableConditionNode` (:174)
- objects: `SpaceshipConfig` (spaceship.rs:71), `SpaceshipController` (:16, +Reflect),
  `PlayerControllerConfig` (:23, +Reflect), `AIControllerConfig` (:40, +Reflect),
  `SpaceshipSectionConfig` (:60, +Reflect), `AsteroidConfig` (asteroid.rs:21),
  `BeaconConfig` (beacon.rs:37), `SalvageCrateConfig` (salvage.rs:42)

### The crux: asset handles are not data-file-friendly

Most of the tree is pure data and serializes to RON trivially once `serde` derives
are added. The hard part is a small set of fields that hold live Bevy handles /
render types, which cannot round-trip through a hand-authored file as-is:

- `ScenarioConfig.cubemap: Handle<Image>` (loader.rs:35)
- `AsteroidConfig.texture: Handle<Image>` (asteroid.rs:24)
- `BeaconRenderConfig.color: Color` (beacon.rs:~78) - bevy `Color` needs a stable
  authoring form.

Today these are populated from `GameAssets` handles at build time (e.g.
`scenario.rs:57 texture: game_assets.asteroid_texture.clone()`). For a data file a
modder writes, they must be authored by path/name (`texture: "textures/rock.png"`)
and resolved to a `Handle` at load. Two implementation strategies, decide during
planning:

- **(a) Separate authoring structs.** `AsteroidConfigAuth { texture: String, .. }`
  etc., deserialized from RON, then lowered to the runtime `AsteroidConfig` inside
  the `AssetLoader` where a `LoadContext` can resolve paths to handles. Clean split
  of "on-disk shape" vs "runtime shape"; more boilerplate.
- **(b) Custom serde on the runtime types.** `#[serde(with = ...)]` / a newtype that
  (de)serializes a `Handle<Image>` as its asset path. Less duplication; but a
  `Handle` has no path at serialize time without an `AssetServer`, and deserialize
  needs the `LoadContext`, so this fights serde's context-free model.

Recommendation leans **(a)**: an authoring layer that the `AssetLoader` lowers,
because the loader is exactly where a `LoadContext` exists to turn
`load_context.load("textures/rock.png")` into a `Handle`. It also gives modders a
smaller, friendlier surface than the full runtime structs.

### Optimization reality check

The seeded "index handlers by event name" (20260525-133014) is VALID but its
speedup is small today, and the real hotspots are elsewhere. Measured facts:

- Handler counts: `menu_ambience`/`asteroid_field`/`asteroid_next` = 1 handler each;
  `shakedown` = 19 (mix of OnStart/OnEnter/OnDestroyed/OnUpdate). Dispatch is a
  linear scan over handler entities of the matching event type (the actual loop is
  in the external `bevy-common-systems` `GameEventsPlugin`, git rev
  `4a743b2...`; nova only spawns the handlers, loader.rs:486-499).
- `OnUpdate` fires every frame while a scenario is live (loader.rs:139); `OnOrbit`
  / `OnTravelLock` / `OnCombatLock` re-fire on a ~5s cadence.
- Per-frame costs that dominate over the O(handlers) scan: string equality in
  `EntityFilterConfig` (filters.rs:38-101, e.g. `id_value == id` at :52) and the
  recursive `VariableConditionNode::evaluate` cloning `VariableLiteral`s
  (variables.rs:194-230), both hit by shakedown's per-frame milestone handlers.

So with today's built-ins, none of this is a real bottleneck. What changes the
calculus is the whole point of the sprint: **modding opens the door to large
third-party scenarios** with hundreds of handlers and dense per-frame filters. The
optimization work is insurance for that world, and it must be **measured before and
after** so we do not optimize blind. Hence a benchmark task gates the rest.

### Editor lowering (for the scenario-builder task)

`crates/nova_editor/src/lib.rs` already holds the ship as a
`PlayerSpaceshipConfig { sections: HashMap<Entity, SpaceshipSectionConfig>, inputs:
HashMap<Entity, Vec<Binding>> }` resource (lib.rs:196), and `test_scenario()`
(lib.rs:202-379) already lowers it into a full `ScenarioConfig` procedurally
(collecting sections at :315, wrapping in `SpaceshipController::Player`, emitting a
`ScenarioObjectConfig::Spaceship`). So "save the current editor state to
`*.scenario.ron`" is mostly: serialize that already-constructed `ScenarioConfig`
through the same authoring layer the loader consumes; "load" is the reverse, feeding
a parsed `ScenarioConfig` into the editor instead of the hardcoded procedural setup.
This confirms the scenario-builder task (20260714-081703) is a composition problem,
not a rewrite - but only once the format + authoring layer (below) exist.

## Options considered

- **Add serde to runtime types + custom handle serde (b).** Fewer types, but fights
  serde's context-free model at both ends (no path at serialize, needs LoadContext
  at deserialize). Rejected as the primary path.
- **Authoring structs lowered by the AssetLoader (a).** Extra structs, but the
  handle resolution lands exactly where the `LoadContext` lives and gives modders a
  smaller surface. Recommended.
- **Optimize now vs measure-then-optimize.** Optimizing now is premature (built-ins
  are 1-19 handlers, microsecond costs). Measure-then-optimize with a synthetic
  large-mod benchmark is correct and directly answers the user's "check if the
  optimizations are ok". Recommended.

## Recommendation

Split the format work along its natural seams and gate the optimization work behind
a benchmark:

**Format (strand 1 of the sprint):**
1. `serde` derives across the whole config tree (mechanical) - keep on 20260525-133029.
2. An **authoring layer + `*.scenario.ron` AssetLoader** that resolves asset paths
   (cubemap, textures) to handles via `LoadContext` and lowers authoring structs to
   runtime configs - the design-heavy piece, its own task (NEW 20260714-083326).
3. **Port built-ins** (`asteroid_field`, `asteroid_next`, `menu_ambience`,
   `shakedown`) into `assets/scenarios/*.ron` to dogfood - on 20260525-133029.
4. **Load `GameScenarios` from the RON assets** instead of code - 20260525-133028.

**Optimizations (strand, measured):**
5. **Benchmark + profile** scenario dispatch with a synthetic many-handler /
   dense-filter scenario, to get a baseline and prove where the time goes (NEW
   20260714-083331). Gates the two below; answers "are the optimizations ok".
6. **Index handlers by event name** - 20260525-133014, now explicitly gated on the
   benchmark and noting the dispatch loop lives in `bevy-common-systems`.
7. **Hot-path: intern entity-filter string keys + cache/compile condition eval**
   (NEW 20260714-083339), only if the benchmark shows they matter.

## Open questions

- Authoring strategy (a) vs (b) - resolve in the AssetLoader task once serde derives
  are in and the handle-resolution ergonomics are visible in real code.
- Does indexing require a patch to `bevy-common-systems` (the dispatch loop is
  there), or can nova group handlers on its side? Resolve in the benchmark task.
- Section blueprints (`sections.rs` has a "load from JSON" stub) - fold into the
  same authoring layer or defer? Scope when planning 20260714-083326.
- Do we keep `variables.rs` at all once scripting (piccolo, backlog) lands, or does
  Lua replace the AST? Out of scope here; revisit at phase 2.

## Next steps

Format:
- tatr 20260525-133029 (p80) - serde derives + port built-ins to `assets/scenarios/*.ron`.
- tatr 20260714-083326 (p75, NEW) - authoring layer + `*.scenario.ron` AssetLoader
  (asset-path -> Handle resolution).
- tatr 20260525-133028 (p70) - load `GameScenarios` from the RON assets.

Optimizations (measured):
- tatr 20260714-083331 (p45, NEW) - scenario dispatch benchmark + profile baseline.
- tatr 20260525-133014 (p40) - index handlers by event name (gated on the benchmark).
- tatr 20260714-083339 (p35, NEW) - hot-path: intern filter string keys + cache
  condition eval (gated on the benchmark).

## Fix record

(Appended by each implementing task as it lands.)
