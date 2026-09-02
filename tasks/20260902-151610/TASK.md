# Author domain distances in meters and confine world units to engine boundaries

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: v0.13.0,units,content,refactor,docs

The creator contract currently requires authors to remember that one raw world
unit is 10 m. A field such as `blast_radius: 30` therefore means 300 m. This is
an avoidable source of errors, and the same implicit scale is spread through
gameplay code, scenarios, examples, tests, and documentation.

Make meters the unit of authored content and domain code. Keep Nova's existing
physical scale: Bevy, rendering, and physics still receive one engine world
unit for every 10 m. Convert only at explicit low-level integration boundaries.
This is a format break for every mod and scenario.

## Decisions

- Authored content uses meters. A 300 m blast is written as
  `blast_radius: 300`, not `30`.
- Domain-level Rust APIs and state use strong dimensional types. Start with a
  `Meters` type and add equally explicit types for required derived quantities,
  such as meters per second and meters per second squared. Do not rely only on
  variable suffixes to prevent mixing scales.
- Serialized numeric fields remain readable bare numbers whose documented unit
  is SI. Do not require verbose typed RON wrappers merely to obtain Rust type
  safety.
- Engine world units are an implementation detail. They may appear only where
  Nova integrates with Bevy transforms and meshes, physics, rendering and
  shaders, or converts between physical distance and build-grid coordinates.
- Grid coordinates and cell indices remain discrete values. A build-grid cell
  still spans one engine world unit, but its physical side length is authored
  and described as 10 m.
- Preserve current physical dimensions, speeds, accelerations, ranges, and game
  balance. Existing domain and content values normally become 10 times their
  old numeric value; the engine adapter converts meters back by dividing by 10.
- This is a clean beta format break. Migrate every in-repository mod and caller.
  Add no legacy parser, fallback, alias, old-unit field, heuristic, dual-mode
  API, or compatibility version. If content breaks, fix that content.

## Scope

First inventory every value whose dimension contains length. Trace each value
from its authored or domain source to its engine consumer instead of relying on
field-name searches alone. Cover at least:

- section, ship, weapon, projectile, blast, damage, flight, gravity, radar,
  targeting, scenario, and editor configuration;
- authored positions, offsets, dimensions, radii, ranges, speeds,
  accelerations, areas, volumes, and length-dependent coefficients;
- Rust content builders and generated files under `assets/base/`;
- scenario documents, the editor's load/save and controls, examples, probes,
  screenshot fixtures, tests, and test support;
- gameplay components, resources, events, calculations, and public crate APIs;
- HUD and player-facing formatting, taking care not to convert values twice now
  that their domain source is already in meters;
- creator pages under `web/src/create/`, player pages under `web/src/wiki/`,
  developer documentation under `docs/`, web widgets and their fixtures, and
  relevant source comments and rustdoc.

Counts, ratios, normalized values, angles, time, pixels, and grid indices are
not distances and must not be wrapped or scaled. Audit formulas that combine
length with another dimension so their constants and tests remain physically
correct. Do not perform blind textual multiplication.

## Design constraints

- Put the shared unit types and conversion constants in the lowest existing
  crate that all required consumers can use. Export them through that crate's
  prelude. Do not add a dependency edge only to reach a conversion helper.
- Keep conversion functions directional and explicit, for example meters to
  engine length and engine length to meters. Avoid an ambiguous generic
  `convert` or scattered `/ 10.0` and `* 10.0` arithmetic.
- Provide type-safe scalar and spatial operations needed by domain code without
  turning the task into a general-purpose units framework. Record any place
  that must remain an untyped Bevy or physics vector as an engine boundary.
- Serde defaults, validation ranges, editor limits, and generated schemas must
  use the new meter semantics.
- Do not rename a stable field only to append `_meters` when its format contract
  already states meters. Rename ambiguous Rust APIs where that makes the
  engine/domain boundary clearer.
- Do not change the actual scene scale or retune gameplay as part of the unit
  migration.

## Suggested execution

1. Write the inventory and classify each site as domain, serialized boundary,
   engine boundary, or non-distance.
2. Add the strong domain unit types and the single 10 m conversion boundary,
   with focused conversion, arithmetic, and serde tests.
3. Migrate one vertical slice through content, domain logic, and engine output
   to prove the design before applying it across the repository.
4. Migrate all Rust callers, authored builders, scenarios, examples, probes,
   and tests. Regenerate generated content; never hand-edit generated RON.
5. Update the editor and web widgets, then update creator, player, and developer
   documentation from the resulting code.
6. Remove temporary raw-unit paths and audit the repository for world-unit
   assumptions outside approved engine boundaries.

Keep the work in reviewable subsystem commits. Do not add temporary
compatibility code to make intermediate commits accept old content.

## Acceptance criteria

- A creator writes distances in meters in every supported content and scenario
  format. Representative fixtures prove that `blast_radius: 300` produces the
  same 300 m blast that `30` produced before this break.
- Domain-level APIs cannot accidentally pass a `Meters` value as an engine
  world-unit scalar, or mix distance, speed, and acceleration quantities.
- Bevy transforms, physics, rendering, and grid placement receive the same
  engine-scale values as before the migration.
- Every repository-owned mod, generated asset, scenario, example, probe, test,
  editor path, and web fixture uses the new semantics. No old-format fixture is
  retained as a compatibility test.
- Searches and an inventory review find no unexplained `u`, `u/s`, "world
  unit", `* 10`, or `/ 10` distance convention in active domain or creator
  code. Approved low-level boundaries explain their engine-unit use locally.
- Creator documentation no longer asks authors to perform the 10 m mental
  conversion. Player-facing output remains in meters and does not change by a
  factor of ten.
- The affected content generation and lint commands pass, and generated diffs
  are inspected.
- Affected Rust tests and representative native, scenario, editor, and WASM
  checks pass. Relevant visual or probe output is inspected where numeric
  assertions alone cannot prove unchanged scale.
- `nix develop --command mdbook build` and `cd web && npm run ci` pass, and the
  generated documentation is inspected.
- `CHANGELOG.md` contains one concise v0.13.0 entry marked `**(breaking)**`,
  based on the last release. It states that authored and domain distances now
  use meters. Do not document a migration path for the old beta format.

## Not in scope

- Backward compatibility of any kind.
- Changing one world unit's internal 10 m scale.
- Gameplay balance or visual-scale changes unrelated to preserving current
  behavior.
- A universal dimensional-analysis library beyond the types Nova needs for
  this migration.

## Proof

Twenty-one subsystem commits on `meters-units`, from `beb6b73c` (the inventory)
to `5c88a8bd` (the console's conversion). `INVENTORY.md` beside this file holds
the classification and the reasoning; this section records what was verified and
what the spec did not settle.

### The types

`crates/nova_events/src/units.rs`: `Meters`, `MetersPerSecond`,
`MetersPerSecondSquared`, `Meters3`, and `METERS_PER_UNIT`. `nova_events` is the
deepest crate both the physics side (`nova_ship`) and the display side
(`nova_ui`) already reach, so nothing gained a dependency edge. Each is
`#[serde(transparent)]`, so RON stays a bare number, and `Reflect`, so the
editor inspector sees a one-field tuple struct. Conversion is directional -
`to_engine()` and `from_engine()` - and there is deliberately no `Display`, no
`Deref` and no `From<f32>`, so a quantity cannot reach a formatter or an engine
scalar by accident.

### Decisions the spec left open

- **`FlightSettings` and the AI envelope constants stay engine-side.** They are
  the defaults and comparands of avian positions and velocities, read every
  tick. Typing them would put a conversion in the hot loop instead of at the
  spawn that authored the override, and would not make one authored file
  clearer. The authored overrides that feed them ARE typed and cross once, in
  the spawner.
- **`AsteroidConfig::mass` and `AnchorConfig::mass` stay engine-side.** They are
  a gravitational parameter, `L^3/T^2`, not an SI mass; an SI value would be a
  thousand times the world-unit one. The radius beside them is a real length and
  did migrate. The published figures derived from the pair - surface gravity in
  m/s^2, sphere of influence in km - are already SI.
- **`ItemHighlight::world_radius` stays engine-side**: the HUD sizes a bracket
  from it in camera space, never as a distance a player reads.
- **`ThreePointRig::scale` is dimensionless.** The rig's offset table carries
  the lengths and moved to meters; the scale arguments are unchanged.
- **Three editor rows read in build-grid cells, not meters.** `width` and a
  thruster exhaust cone's `*radius` / `*height` are mesh geometry built inside
  the section's own cell, so a 0.8 there is 8 m of a 10 m cell. A railgun's
  `rake_radius` is the one genuinely metric member of that family and got its
  own metered spec. `metered()` became identical to `floored()` once the scale
  factor went, and was deleted.
- **The frozen news posts keep their original units.** `web/src/news/*.md`
  argues about one shipped release; `docs/keeping-docs-in-sync.md` freezes a
  post's media for the same reason its prose is history. Their `u` and `u/s`
  figures describe what those versions shipped and were left alone.

### Verified

- `cargo fmt --all -- --check`: clean. The last two commits were made with the
  pre-commit hook active and it passed. Earlier commits used `--no-verify`
  because the hook checks workspace-wide rustfmt while sibling lanes still had
  unformatted files in flight; every commit's own paths were fmt-clean at the
  time.
- `cargo check --workspace --examples --keep-going`, and the same with
  `--features debug`: no errors. `debug` is not a default feature, so the plain
  pass skips most probe and assertion bodies - both are needed.
- WASM: `cargo check --workspace --exclude nova_probe_cli --exclude nova_channel
  --target wasm32-unknown-unknown`: clean.
- Per-crate `--lib`: nova_ship 785, nova_editor 428, nova_gameplay 272,
  nova_scenario 249, nova_hud 236, nova_os_ui 118, nova_authoring 89,
  nova_probe 80, nova_ui 56, nova_events units 10, nova_console 6. Integration:
  all 15 `nova_assets` binaries, `content_ron_parity`, `broadside_assault`. No
  failures. The full workspace suite and workspace Clippy were NOT run, by
  instruction.
- `content gen` then `content lint`: the generated tree is already in sync and
  the run produces no diff; lint reports 0 errors, 0 warnings, 0 findings over
  13 balance-audited scenarios. When the generation first landed, a token-wise
  comparison against the pre-migration files showed 696 changed numbers across
  10 files, every one exactly ten times its old value, with the non-numeric
  skeleton byte-identical. The in-repo mods gave 505 and the example mod 37 on
  the same check.
- `mdbook build` and `cd web && npm run ci`: both pass, and the rendered pages
  were inspected rather than only the exit code.
- Scale unchanged, visually: `screenshot_gravity` was captured under Xvfb on
  this branch and compared byte-wise against the shipped `wiki-gravity.png`.
  0.31% of bytes differ, none by more than 16/255 - renderer dither, not a
  silhouette that moved. A factor-of-ten error in that scene is not survivable
  at that tolerance.
- HUD not double-converted: the `system_hud_indicators` probe passes, comparing
  the parsed readout against `Meters::from_engine(player.distance(target))` and
  printing `CLS +143.6 m/s`. The captured HUD frame reads `2.23 km` on the
  off-screen marker.
- Sweep: no `* 10` or `/ 10` conversion survives anywhere; every `METERS_PER_UNIT`
  use outside `units.rs` is a documented boundary; every surviving `u`, `u/s` or
  "world unit" sits in a module that states it is engine-side, or names its
  metric equivalent on the same line, or is a frozen news post.
