# Findings: scenario id and mod references in source code

Audit date: 2026-07-16. Method: grep sweeps over `src/`, `crates/`,
`examples/`, `tests/`, `webmods/`, `assets/`, plus reading the relevant
modules. Line numbers are as of commit a8dad52e.

## 1. The .rs -> .ron pipeline for base content (confirmed)

The suspicion is correct, with one twist: the converter is a test.

How it works:

- The base scenarios and the section catalog are authored as Rust builders:
  - `crates/nova_assets/src/scenario.rs` - `menu_ambience`, `asteroid_field`,
    `asteroid_next` (612 lines)
  - `crates/nova_assets/src/scenario/shakedown.rs` - `shakedown_run`
    (2146 lines)
  - `crates/nova_assets/src/sections.rs` - `build_sections` (section catalog)
- `nova_assets::scenario_generation` (`crates/nova_assets/src/lib.rs:48-115`,
  `#[doc(hidden)]`) wraps them: `build_scenarios`, `build_section_content`,
  `build_scenario_contents`, plus a deterministic `pretty_config` so the RON
  output is diff-friendly.
- `crates/nova_assets/tests/content_ron_parity.rs` is BOTH the generator and
  the guard: if `assets/base/**/<id>.content.ron` does not exist, the test
  WRITES it and passes; on every later run it re-serializes the builders and
  asserts byte equality with the committed file. Regeneration procedure is
  "delete the file and re-run cargo test".
- At runtime the builders are never called. The game loads
  `assets/mods.catalog.ron` -> `assets/base/base.bundle.ron` -> the committed
  `*.content.ron` files through the real asset loaders (`nova_modding`), and
  `register_bundles` routes items into `GameSections` / `GameScenarios`.
  So yes: the game plays the .ron variant, the .rs is authoring-time only.

The twist / inconsistency: `assets/base/scenarios/demo.content.ron` is the
exception. It is hand-written RON with no builder and no parity guard; the
`demo_scenario` integration test exercises it instead. So base content today
has TWO authoring styles.

### Is this ok, or should we write .ron directly?

Honest answer: the Rust-authoring half is good, the "a test writes the
assets" half is the weak part.

What the .rs builders genuinely buy:

- Type safety at authoring time. `shakedown_run` is a 2146-line builder;
  a field rename or enum change in `ScenarioConfig` is caught by the
  compiler. In hand-RON it is caught at load time (or by a test) with a
  serde error pointing at a line in a generated-looking file.
- Composition: constants, loops, shared sub-builders (gate sequences,
  repeated asteroids, shared ship rigs). The equivalent hand-RON would be
  thousands of lines of copy-paste that drift apart.
- The committed RON stays reviewable because serialization is deterministic.

What it costs:

- Two sources of truth with a parity test as the sync mechanism, and a
  surprising regeneration UX (deleting an asset file so a TEST regenerates
  it). Tests that write files as a side effect are a footgun: a fresh
  checkout missing the file silently "passes" the first run.
- Dogfooding gap: the repo's own story is "the base game is just a mod",
  and mods (demo mod, gauntlet) author RON by hand. Base content skipping
  the mod-authoring path means the authoring experience we ship to modders
  is not the one we live with daily. Only `demo.content.ron` walks that path.
- The builders compile into the shipped lib despite being dead at runtime
  (minor, they are `#[doc(hidden)]` and small relative to the binary).

Verdict: keep authoring complex scenarios in Rust - for content this size
the compiler is the right editor, and the loaders still get exercised on the
committed RON, which is what actually matters for the modding pipeline. But
move generation out of the test:

- Make an explicit generator (a small bin, e.g.
  `cargo run -p nova_assets --bin gen-content`, or an xtask) that writes the
  files unconditionally.
- Keep `content_ron_parity` as a pure guard: assert-only, never write, fail
  with "run the generator" when drifted. CI then catches a forgotten
  regeneration instead of a test quietly creating files.
- Optionally: hand-author genuinely simple scenarios in RON (the demo
  scenario already is), reserve builders for procedural/large content. That
  narrows the dual-source surface without giving up type safety where it
  pays.

Writing everything in .ron directly is defensible purity but a real
maintainability loss for shakedown-sized content; not recommended.

## 2. "gauntlet" references (a portal mod named inside the core repo)

Full list. The good news first: ZERO production (non-test) code references
gauntlet. Every hit is in test code or test fixtures:

1. `crates/nova_mod_format/src/lib.rs:256-284` (unit tests, module starts
   at :168) - uses "gauntlet" as the fixture id when testing
   `PortalEntry`/catalog parsing. Pure fixture data; any name would do.
2. `crates/nova_menu/src/lib.rs:5180-5357` (unit tests, module starts at
   :3123) - "gauntlet_run" is the second fixture scenario in the Scenarios
   picker tests. Pure fixture; any name would do.
3. `crates/nova_portal_gen/tests/generate.rs:32` - runs the real generator
   over the real `webmods/` tree and asserts the entry `gauntlet` is
   published. This couples a core test to the existence of one specific mod.
4. `crates/nova_assets/tests/portal_install.rs` (920 lines) - uses the real
   `webmods/gauntlet` files as the fixture for the full portal
   fetch/install/enable/uninstall lifecycle, asserting on its id, meta name,
   bundle path, and that enabling registers `gauntlet_run`.
5. `crates/nova_assets/tests/gauntlet_race.rs` (259 lines) - a dedicated
   behavior test for the gauntlet mod's slalom gameplay: `include_str!`s
   `webmods/gauntlet/gauntlet.content.ron`, registers its OnEnter/OnStart
   handlers, and drives gate progression. This is the strongest instance of
   what prompted this audit: core CI testing one mod's CONTENT in depth.
6. `crates/nova_assets/tests/webmods_validation.rs` - does NOT hardcode
   gauntlet. It scans `webmods/` generically and requires every bundle to
   load recursively through the real loaders. This is the right pattern.

Note: the same pattern exists for the shipped demo mod -
`crates/nova_assets/tests/arena_combat.rs` loads the actual
`assets/mods/demo/mod.content.ron` and tests its win logic. So "deep
behavior tests over shipped content" is a house style, not a gauntlet
one-off.

### Should we do this?

Honest answer, split by category:

- Fixture names (items 1, 2): harmless. Tests needed SOME mod/scenario name
  and borrowed a real one. Renaming to neutral ids ("fixture_pack",
  "slalom_run") removes the mental coupling for near-zero effort, but this
  is cosmetic, not architectural.
- Generator/install tests using the real webmods tree (items 3, 4): a
  deliberate realism trade. Using real content means the tests break when a
  mod is renamed/removed - which cuts both ways: it catches real publish
  regressions, but it also means "delete the gauntlet mod" fails core CI.
  Better shape: run these against a committed synthetic fixture tree under
  the test's own directory, and keep ONE generic assertion over the real
  `webmods/` ("every mod dir appears in the generated catalog"), the same
  way webmods_validation already generically covers loading. Then mods can
  come and go without touching core tests.
- The dedicated content-behavior tests (item 5, and arena_combat for demo):
  your instinct is right - this is too much for a MOD. If gauntlet is meant
  to be the copy-me template for external modders (its README says exactly
  that), external mods will not get 259-line behavior tests in core CI, so
  gauntlet having one is not testing the system, it is testing content.
  What these tests actually protect splits in two:
  - Engine mechanics (OnEnter area triggers, OnDestroyed bridge, handler
    registration). These deserve engine tests with SYNTHETIC scenario
    fixtures - and partly already have them (arena_combat's header notes
    nova_scenario owns the physical OnDestroyed bridge test).
  - "The shipped showcase content still works." That is a content smoke
    test. Legitimate while gauntlet/demo are first-party showcase pieces,
    but it should be understood as such, and the generic load gate
    (webmods_validation) plus a played-through check is arguably enough.
  Recommendation: extract the engine-mechanic assertions into engine tests
  with synthetic fixtures, then either delete the per-mod behavior tests or
  consciously keep them as thin content smoke tests. Do not add such tests
  for future mods.

## 3. Scenario ids hardcoded in production code

The complete list of NON-test code that names a specific scenario id:

1. `crates/nova_menu/src/lib.rs:49` -
   `const NEW_GAME_SCENARIO_ID: &str = "shakedown_run";`
   Used as the canned New Game start and as the fallback when a picked
   scenario id is no longer registered (`start_new_game_scenario`, :3030-59).
2. `crates/nova_menu/src/lib.rs:52` -
   `const MENU_AMBIENCE_SCENARIO_ID: &str = "menu_ambience";`
   Used by `load_menu_ambience` (:971-974), which PANICS if the scenario is
   missing from `GameScenarios`.
3. `crates/nova_assets/src/scenario.rs:402,503,528` - the builders chain
   `asteroid_field` <-> `asteroid_next` via `NextScenario`/outcome ids. This
   is content referencing content (it ends up in the RON), not engine code
   knowing about content. Fine as-is.
4. Examples (`examples/12_menu_newgame.rs`, `13_screenshot_reel.rs`, etc.)
   build or embed their own scenarios and reference their own ids. Examples
   are standalone rigs; fine as-is.
5. Comments only: `nova_scenario/src/objects/asteroid.rs:696`,
   `nova_gameplay/src/hud/mod.rs:371`, `nova_gameplay/src/flight.rs:390`,
   `nova_assets/src/sections.rs:109`, `nova_debug/src/harness.rs:121,178`.
   Prose context, no coupling; they will silently rot on a rename but that
   is acceptable.

So the actual engine-knows-content coupling is exactly two constants, both
in nova_menu. Everything else is content, tests, examples, or prose.

### Should we do this? The flag idea

The precedent already exists: `ScenarioConfig.hidden`
(`crates/nova_scenario/src/loader.rs:55-61`) keeps a scenario out of the
Scenarios picker (`listed_scenarios`, `nova_menu/src/lib.rs:1635`), mirroring
the mods-catalog `hidden` flag. menu_ambience, asteroid_field and
asteroid_next already ship `hidden: true` in their RON.

Honest answer: yes, replacing the two constants with data is the right
move, and it is more than cosmetics - it turns "what is the menu backdrop"
and "what does New Game launch" into moddable/overlayable data, which fits
the "base game is just a mod" thesis. It also converts the
`load_menu_ambience` panic (a mod rename away from a crash at menu entry)
into a designed fallback.

Two viable designs:

- Per-scenario role flags, e.g. `menu_backdrop: true` on menu_ambience and
  `new_game: true` on shakedown_run, next to `hidden`. Simple, consistent
  with the existing flag. Needs a defined tie-break when several enabled
  scenarios carry the flag (catalog load order, last wins, matching the
  overlay semantics mods already have) and a defined fallback when none do
  (no backdrop; New Game falls back to the first listed scenario or
  disables).
- Bundle-level defaults: the bundle/catalog declares pointers
  (`menu_scenario: "menu_ambience"`, `new_game_scenario: "shakedown_run"`),
  last enabled bundle wins. Models "exactly one winner" better than a
  boolean scattered across scenarios and keeps ScenarioConfig lean, but adds
  a second place where scenario ids are written down.

Recommendation: per-scenario flags with last-enabled-mod-wins tie-break.
It matches the `hidden` precedent, keeps everything on the scenario, and
"a mod can replace your menu backdrop" falls out for free. Either way the
two nova_menu constants become fallback-free lookups over flags.

## 4. Mod ids referenced in code (beyond gauntlet)

- "base": `assets/mods.catalog.ron` marks it `base: true`; code only ever
  keys on the `base` FLAG (seeding EnabledMods, locking the UI row), never
  the string "base". Good.
- "demo": ships in `assets/mods/demo/`, listed in the catalog, disabled by
  default. Referenced only in tests (`demo_scenario.rs`, `arena_combat.rs`,
  `nova_modding` unit tests at lib.rs:374+, inside cfg(test) at :348). Same
  assessment as gauntlet item 5 above.
- CI: `.github/workflows/deploy-page.yaml` runs `nova_portal_gen` over
  `webmods/` generically; no mod names hardcoded. Good.

## 5. Summary of recommended follow-ups (pending direction)

1. Split content generation out of `content_ron_parity`: explicit generator
   bin/xtask writes, the test only asserts. (Section 1)
2. Add scenario role flags (`menu_backdrop`, `new_game`) beside `hidden`;
   delete the two nova_menu id constants; replace the menu_ambience panic
   with a graceful no-backdrop path. (Section 3)
3. Repoint `portal_install.rs` / `portal_gen` tests at synthetic fixture
   trees; keep one generic every-mod-publishes assertion over webmods/.
   (Section 2)
4. Fold the engine-mechanic coverage of `gauntlet_race.rs` /
   `arena_combat.rs` into engine tests with synthetic fixtures; then drop or
   consciously keep the per-mod smoke tests. (Section 2)
5. Cosmetic, lowest value: neutral fixture names in nova_mod_format and
   nova_menu unit tests. (Section 2)

None of these are started; this task is research only.
