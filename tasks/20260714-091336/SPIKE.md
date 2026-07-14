# Spike: crate boundary for the modding language - `nova_modding` vs extend `nova_scenario`

- DATE: 20260714-091336
- STATUS: RECOMMENDED
- TAGS: spike, modding, architecture

## Question

The RON modding format needs to live somewhere. The user's instinct: "it would
make sense to have a separate crate `nova_*` for this and use it where needed."
Where does the declarative-format / serialization / authoring surface belong - a
new crate, or added onto the existing `nova_scenario` engine crate? And if a new
crate, what exactly does it own and which way do the dependencies point?

## Context

- `nova_scenario` is the runtime scenario ENGINE: `actions`, `events`, `filters`,
  `variables`, `world` (`NovaEventWorld`), `loader`, `objects`. It depends on
  `nova_events` + `nova_gameplay` (so it is a high-level crate, not a leaf). It has
  NO serde today (confirmed).
- Depending on `nova_scenario` today: `nova_assets` (builds scenarios in code),
  `nova_editor`, `nova_core`, `nova_menu`, `nova_debug`.
- The detailed design spike (`tasks/20260714-083224/SPIKE.md`) established: most of
  the config tree is pure data and serializes trivially; the sticky part is the
  asset-handle fields (`ScenarioConfig.cubemap`, `AsteroidConfig.texture`,
  `Handle<Image>`; `BeaconRenderConfig.color`, bevy `Color`), which need an
  authoring representation (paths/strings) lowered to handles in an `AssetLoader`
  where a `LoadContext` exists. It leaned toward "authoring structs" over custom
  serde on the runtime types.

The crate question and the authoring-strategy question are the same question: if
authoring is a separate representation, it wants its own home.

## Options considered

- **A. Everything in `nova_scenario`.** Add serde + the RON `AssetLoader` + the
  authoring wrappers directly to the engine crate. Pros: no new crate, no
  duplication. Cons: couples the runtime engine to a file format and to serde;
  bloats the crate that everything already depends on; the "modding language" has
  no identifiable home; future scripting (piccolo) would pile in here too.
- **B. New crate `nova_modding`, full parallel authoring tree.** A complete
  duplicate type tree (`ScenarioConfigAuth`, `EventActionConfigAuth`, ...) with
  serde, lowering to the runtime types. Pros: total decoupling, `nova_scenario`
  untouched. Cons: heavy duplication - every new action/filter/variable must be
  added in two places forever; high drift risk.
- **C. New crate `nova_modding` + optional `serde` feature on `nova_scenario`
  (recommended).** `nova_scenario` gains an off-by-default `serde` feature that
  `#[cfg_attr]`-derives `Serialize/Deserialize` on its PURE-DATA config types (the
  bulk of the tree - events, filters, variables, most actions). `nova_modding` is a
  new crate that (1) turns that feature on, (2) defines authoring wrappers ONLY for
  the few handle/render-type fields, (3) owns the `*.scenario.ron` `AssetLoader` and
  the authoring->runtime lowering (resolving asset paths to handles via
  `LoadContext`), and (4) exposes the plugin that registers the loader. Pros:
  minimal duplication (wrappers only where types genuinely differ), the engine's
  default build stays serde-free, and there is one clear home for the modding
  surface (and later the scripting integration). This is the standard Rust split:
  data crate carries optional serde, the format/loader crate is separate.

## Recommendation

Adopt **C**. Create `crates/nova_modding`:

- `nova_scenario`: add a `serde` feature (off by default); gate `derive(Serialize,
  Deserialize)` on the pure-data config types via `cfg_attr`. No handle fields get
  raw derives - either skip them (`#[serde(skip)]` with a runtime default) or, better,
  keep them out of the serializable surface and let the authoring wrappers own them.
- `nova_modding`: depends on `nova_scenario` (with `serde`), `bevy` (AssetLoader,
  LoadContext), `ron`, `serde`, `thiserror`. Contains:
  - authoring types for the handle-bearing configs (String asset paths / a stable
    color form) and the top-level `ScenarioConfig` container,
  - a `ScenarioLoader` implementing bevy `AssetLoader` for `*.scenario.ron` that
    parses the authoring form and lowers it to a runtime `ScenarioConfig`
    (`load_context.load(path)` for each asset ref),
  - `NovaModdingPlugin` registering the loader + the `Scenario` asset type,
  - later (phase 2): the piccolo scripting integration lands here too.
- `nova_assets`: depend on `nova_modding`; load `assets/scenarios/*.ron` into
  `GameScenarios` instead of building them in `scenario.rs`.
- `nova_editor`: depend on `nova_modding` for scenario save (serialize the authoring
  form) / load.

Dependency direction stays acyclic: `nova_modding -> nova_scenario -> {nova_gameplay,
nova_events}`; `nova_assets -> nova_modding`; `nova_editor -> nova_modding`.

Why C over B: the pure-data majority of the tree does not need a hand-maintained
twin; only the handful of handle/render fields do. Wrapping just those keeps the
duplication proportional to the actual impedance mismatch. Why C over A: it gives
the modding language a real home, keeps the engine crate's default build free of
serde/format concerns, and leaves room for the scripting phase without further
bloating `nova_scenario`.

## Open questions

- Exact split of "pure-data" vs "needs a wrapper" - resolve while adding the feature
  (the handle/color fields are the only known offenders; watch for any `Entity` or
  non-serializable bevy types in the object configs).
- Crate name: `nova_modding` (chosen - broad enough to also host scripting) vs
  `nova_scenario_format` (narrower). Going with `nova_modding`.
- Whether the editor serializes the runtime `ScenarioConfig` back to the authoring
  form, or the editor works in the authoring form directly. Decide in the editor
  task (20260714-081703).

## Next steps

Re-scope the existing modding-foundation tasks around crate C rather than seeding
new ones (same work, clearer home):

- 20260525-133029 -> "add optional `serde` feature + derives to `nova_scenario`
  pure-data config types".
- 20260714-083326 -> "create `nova_modding` crate: authoring wrappers + `*.scenario.ron`
  AssetLoader + lowering + plugin".
- 20260525-133028 -> "`nova_assets` loads `GameScenarios` via `nova_modding` from
  `assets/scenarios/`".

## Fix record

(Appended by each implementing task as it lands.)
