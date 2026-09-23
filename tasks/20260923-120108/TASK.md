# World generator interface: a generator type, base world in nova_authoring

- STATUS: OPEN
- PRIORITY: 74
- TAGS: v0.15.0,gameplay,architecture,open-world

Third PR in the stack: #58 -> #59 (`nova-world-foundation`) -> this branch
(`nova-world-generator-interface`). Parent spike: `20260824-125938`.

## User facts

- The world generator is a TYPE the plugin is generic over, not a config enum.
- `nova_world` owns mechanisms: streaming, the feature field, placement and
  the check every description passes. It names no content.
- The base game's generator names base content, so it lives in
  `nova_authoring`. `nova_world` must never depend on `nova_authoring`.
- The uniform generator is a streaming baseline for examples, not a shipped
  world.

## Decisions

- `pub trait SectorGenerator: Clone + Debug + Send + Sync + 'static` with
  `validate(&self, WorldGeometry)` and `generate(&self, SectorGenerationInput)
  -> Result<SectorManifest, SectorFault>`. No associated types, no boxed trait
  objects.
- Validation boundary (owner, 2026-09-23, Option C): a generator returns an
  untrusted `SectorManifest` with public fields.
  `validate_manifest(SectorGenerationInput, SectorManifest) ->
  Result<SectorDescription, SectorFault>` is the only constructor of the
  trusted `SectorDescription`, whose fields are private behind read accessors.
  `PreparedSector` fields are private too, so `materialize_sector` only takes
  what `prepare_sector` built.
- `WorldConfig<G> { seed, sector_edge, active_radius, generator: G }` and
  `NovaWorldPlugin<G>`. A second, different `NovaWorldPlugin<_>` panics at
  App construction through a private non-generic marker.
- Jobs, ready payloads, `PreparedSector`, roots, stats and materialization stay
  non-generic. One shared `prepare_sector<G>`.
- Deleted with no aliases: `SectorGeneration`, `UniformAsteroidConfig`,
  `LayeredFeatureConfig`.
- `UniformAsteroids` moves to `examples/shared/world_fixture/`.
- `NovaLayeredWorld` (nova_authoring) takes no caller lists: kinds rock, metal,
  ice and carbon; `PlanetType::ALL`; designs `block_frame_tender_damaged` and
  `block_wreck_plate`.
- Renames: `PreparedAsteroidGeometry` -> `PreparedAsteroid`; `PreparedSector`
  fields `asteroids` and `planets`; `SectorAnchorage` -> `SectorShip`,
  `anchorages` -> `ships`; the feature layer is `Derelict`.
- Placement ownership (owner, 2026-09-23): each generator owns its placement
  state, retries, conventions, ids and rock draw. One generator type runs per
  App, so `nova_world` does not generalize that state. A public `SectorLayout`
  was approved and then superseded the same day; it is deleted. Generators
  keep no claimed-id set: they format ids with `sector_id` and the core check
  refuses a duplicate.
- Shared primitives stay stateless: `WorldGeometry::require_owning_edge`,
  `SectorGenerationInput::stream`, `sector_id(coord, name, index)` (formatting
  only), `sector_features`, `validate_feature_geometry`, the spacing constants
  and `bodies_clear(a_position, a_clearance, b_position, b_clearance)`. The
  core check and both generators' placement searches call `bodies_clear`, so
  the clearance formula has one owner.
- `SectorFault` is the common error type, with `Manifest { id, field, value }`
  for the core check. `generate_sector` refuses a cell with no finite centre
  before `SectorGenerator::generate` is asked (`InvalidGeometry`, the range's
  overflow claim).
- Planet config rules (owner, 2026-09-23, Option A): `nova_scenario` owns
  `PlanetConfig::validate(&self) -> Result<(), PlanetConfigFault>`, with
  public `PlanetConfigFault { field: &'static str, value: String }`. It holds
  the former `check_planet` rules: radius, `invulnerable: true`, relief, sea
  level, mass and lock signature. `check_planet` and `validate_manifest` both
  call it. `validate_manifest` maps a fault to `SectorFault::Manifest`. The
  lint now reports the first planet fault, not every one.
- Excluded: AsteroidKindId, payload schema normalization, derelict metadata in
  the ship catalog, invulnerability, floating origin, persistence, AppBuilder
  wiring, economy, AI, combat, density tuning, world_field_clouds sampling.

## Findings

- The `anchorage` -> `derelict` slug seeds the layer noise and node streams, so
  the derelict spheres moved. The old example home `(-2, -2, 2)` no longer
  owned a derelict ship in its window. The home moved to `(2, 1, 2)`: a
  planetoid in `(0, 0, 0)`, three derelicts in `(4, 3, 0)`, and cells that no
  feature sphere reaches.
- `PlanetType::ALL` makes Volcanic (relief 0.06) the widest planetoid, so the
  layered edge floor is about 8,480 m, not the 8,440 m barren-rock floor.
- `nova_authoring` is not built for wasm (root `Cargo.toml` target gate), so
  `NovaLayeredWorld` does not exist on wasm. No wasm build streams a world yet.

## Delivery

- Branch `nova-world-generator-interface`, stacked PR against
  `nova-world-foundation`.

## Verification

At delivery, on the uncommitted tree over `074402138`:

- `cargo test -p nova_world`: 19 unit tests and 1 doctest pass, including
  `a_malformed_generator_answer_is_refused_before_preparation` (nine bad
  `SectorManifest`s refused by `prepare_sector`).
- `cargo test -p nova_authoring --lib world`: the layered edge floor test
  passes.
- `cargo check --features debug` on the five world examples; `cargo clippy
  -p nova_world -p nova_authoring --tests` clean; `cargo doc -p nova_world
  --no-deps` clean under `-D warnings`; `cargo fmt --check`, `git diff
  --check`.
- `cargo test -p nova_probe_cli --test catalog_drift` and
  `scripts/check-probe-suites.py` pass.
- `probe run --correctness-only`: `system_world_sectors`, `world_sectors`,
  `world_features`, `world_field_slices` and `world_field_clouds` all verdict
  OK with a clean log. The range reports 250 manifests identical over three
  walks, the overflow cell refused as `InvalidGeometry` before spawn, 6
  spheres (2 asteroid, 1 planet, 3 derelict), 26 objects in 125 cells with 23
  same-cell pairs clear, 1 planetoid and 3 inert derelict ships live.
- Not run: the workspace test suite, workspace Clippy, and the full probe
  fleet. Headless probes do not prove appearance.

Planet config rules, on the uncommitted tree over `e96a16cc6`:

- Before: with the `validate_manifest` call removed, the new NaN-mass case in
  `a_malformed_generator_answer_is_refused_before_preparation` failed.
  `prepare_sector` built a `PreparedSector` with `mass: Some(NaN)`.
- After: `cargo test -p nova_world --lib` passes 19 tests. The test refuses
  ten bad manifests, and the NaN mass is `Manifest { field: "mass" }`.
- `cargo test -p nova_scenario --lib planet` passes 25, including
  `a_planet_authored_with_impossible_figures_is_an_error`.
  `cargo test -p nova_authoring --lib world` passes 2.
- `cargo clippy -p nova_scenario -p nova_world --tests` is clean. `cargo doc
  -p nova_world --no-deps` is clean under `-D warnings`. `cargo fmt --check`
  and `git diff --check` pass.
- Not run: the world probes. `NovaLayeredWorld` planets have a radius of 600
  to 1,200 m and no overrides, so the new call does not refuse them.

Field shot reframe (owner, 2026-09-23, Option A): the field capture stood at
the home centre, where every feature ring is past the 10 km far plane, so the
picture was near black. It now stands 1,500 m outside the rim of the asteroid
sphere nearest the home centre, 400 m above its equator plane, looking at its
centre. The target comes from the home window's `SectorFeatureSpheres`, and
the run panics if the window reaches no asteroid sphere. Planetoid and
derelict shots are unchanged.

- `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 cargo run --example world_features
  --features debug` on the RTX 3060 Ti (Vulkan, `DiscreteGpu`): exit 0, no
  panic or ERROR lines. The log names `feature_asteroid_0_0_1`, radius
  55,461 m, centre (-4,636, -24,292, 106,258) m. The eye streams window
  (1, -1, 2).
- Field PNG, rows below the readout, 960x1057: before 0 amber ring pixels,
  mean luminance 1.61, 0.04% of pixels over 40. After 3,043 amber pixels over
  954 columns and 841 rows, mean luminance 2.88, 0.74% over 40. Two full-width
  amber lines and one curved amber arc are visible. The lower line is the
  target sphere's equator, as the eye math predicts.
- Planetoid and derelict PNGs are the same framing as before (planetoid mean
  luminance 62.27; one derelict hull centred).
- Artifacts: after `/tmp/wf-proof-final/` (`run.log`, three PNGs); before
  `/tmp/claude-world-features-130043/world-features-field.png`.

## Done when

- The PR is open against `nova-world-foundation` with the checks above green.
- No old generator names remain outside task history.
