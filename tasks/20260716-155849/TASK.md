# Data-driven menu scenario roles: menu_backdrop flag + base-owned new game start

- STATUS: CLOSED
- PRIORITY: 60
- TAGS: v0.7.0,feature

## Goal

Remove the last engine-knows-content coupling: the two scenario id consts
in nova_menu. Menu backdrops become a per-scenario `menu_backdrop` flag -
multiple allowed, mods may add them, the menu picks one at random. The
new-game start becomes base-owned config - explicitly NOT moddable.
Decision from audit task 20260716-141620 (FINDINGS.md section 3) with
user direction on the trust split.

## Steps

- [x] Add `menu_backdrop: bool` to `ScenarioConfig`
      (crates/nova_scenario/src/loader.rs, next to `hidden` :55-61), same
      serde-default treatment; mirror the hidden round-trip serde tests
      (:1585-1624).
- [x] Add `new_game_scenario: Option<String>` to `BundleManifest`
      (crates/nova_mod_format/src/lib.rs:69+), serde-defaulted; document
      the literal RON syntax in the field doc
      (`new_game_scenario: Some("shakedown_run")` - strict RON needs the
      Some(), author-facing-schema lesson).
- [x] New resource `NewGameStart(Option<String>)` in nova_scenario next
      to `GameScenarios`. In `register_bundles`
      (crates/nova_assets/src/lib.rs:488+), set it from the
      `new_game_scenario` of bundles whose catalog decl has `base: true`
      ONLY (`entry.decl.base` is available at :507); warn and ignore a
      non-base bundle declaring it. Unit-pin the base-only honoring (a
      non-base bundle's declaration is ignored and warned).
- [x] Author the data: `menu_backdrop: true` on the menu_ambience builder
      (crates/nova_assets/src/scenario.rs) + regenerate RON via
      `cargo run -p nova_assets --bin gen_content` in the SAME commit;
      `new_game_scenario: Some("shakedown_run")` in the hand-authored
      `assets/base/base.bundle.ron`.
- [x] nova_menu backdrop path: replace `MENU_AMBIENCE_SCENARIO_ID` -
      `load_menu_ambience` (:971) collects all registered scenarios with
      `menu_backdrop: true` and picks one at random via
      `GlobalEntropy<WyRand>` (add bevy_rand to nova_menu Cargo.toml,
      copying nova_gameplay's target-specific wasm_js pattern;
      EntropyPlugin is registered in production by
      nova_gameplay/src/plugin.rs:48, tests add it themselves). None
      flagged = no backdrop, no panic (replaces the panic at :973-974).
- [x] nova_menu new-game path: replace `NEW_GAME_SCENARIO_ID` -
      `start_new_game_scenario` (:3030-3059) fallback chain: picked id if
      registered -> `NewGameStart` id if registered -> first listed
      (!hidden) scenario -> log error, load nothing.
- [x] Delete both consts; update menu tests (picker fixtures :5160+,
      ambience tests around :3200-3760, new-game tests :5291+); add a
      seeded test that the random pick always lands in the flagged set
      (and a multi-backdrop pick reaches more than one over draws).
- [x] Grep production code for remaining scenario id literals - must be
      none (content-to-content refs inside builders/RON are fine).
- [x] Docs (keep-docs-in-sync): wiki authoring guide
      (web/src/wiki/dev/guide-author-scenario.md) documents
      `menu_backdrop`; the mod-format/bundle doc documents
      `new_game_scenario` and its base-only trust rule.
- [x] Verify: `cargo check --workspace --all-targets`, `cargo fmt`, run
      the touched loader/menu/assets/mod_format tests (crate-solo lesson
      for nova_scenario/nova_menu).

## Notes

- Explicitly rejected (user): a scenario flag or any last-mod-wins
  overlay for the new-game start; non-base mods must not control it.
- `hidden` stays orthogonal: backdrops remain hidden from the picker.
- Depends on: 20260716-155823 (gen_content bin for the RON regeneration).

## Close notes (2026-07-16)

What changed, beyond the planned steps:

- `menu_backdrop: bool` on ScenarioConfig (serde-defaulted, skip-if-false,
  round-trip pinned next to hidden/thumbnail); menu_ambience flagged via
  its builder, RON regenerated with gen_content (one command, one-line
  diff - the task-155823 pipeline worked as designed).
- `new_game_scenario: Option<String>` on BundleManifest, carried through
  BundleAsset by the loader; base.bundle.ron declares
  Some("shakedown_run"). register_bundles writes `NewGameStart`
  (new resource in nova_scenario, plugin-inited to None) from the BASE
  entry only, warning on any non-base declaration (shipped or
  downloaded). Pinned by two new demo_scenario tests: the shipped
  end-to-end declaration, and a synthetic base+mod catalog proving the
  enabled non-base declaration is ignored.
- nova_menu names zero scenario ids. load_menu_ambience picks a random
  `menu_backdrop` scenario (bevy_rand GlobalRng, candidates sorted by id
  before the draw so HashMap order cannot leak);
  start_new_game_scenario's chain is picked -> base-declared -> first
  listed -> nothing (each miss warns; empty registry loads nothing and
  does not panic).
- DISCOVERED hazard, fixed in scope: with moddable backdrops, a backdrop
  without a `menu_planetoid` gravity well (or NO backdrop flagged at
  all) would leave the staged camera inactive forever - and the menu UI
  renders through that camera, so the menu would brick. Two fallbacks:
  no-backdrop spawns a bare fixed camera; a well-less backdrop activates
  the camera at the scenario's own pose after a 60-frame grace
  (stage_menu_camera Local counter, reset on each fresh backdrop
  camera). The authoring guide documents the menu_planetoid contract.
- Menu tests own their fixture ids (TEST_START_ID/TEST_BACKDROP_ID,
  seeded EntropyPlugin); new tests: seeded 8-draw rotation stays in the
  flagged set and reaches both backdrops, no-backdrop degrades to a bare
  camera, fallback chain past a bad/missing declaration, manifest
  decode of the new field.
- Docs: guide-author-scenario.md (menu_backdrop + menu_planetoid
  contract), guide-make-a-mod.md (new_game_scenario + trust rule),
  CHANGELOG Unreleased (Modding & Mod Portal).

Verification: nova_menu 49/49, demo_scenario 13/13, nova_mod_format 9/9,
content_ron_parity 2/2, loader serde tests via nova_scenario paired run,
check --all-targets zero errors/unused-warnings, fmt clean. Full suite
is CI's job per the standing instruction. Cargo.lock staged with the
nova_menu dep additions (bevy_rand, rand).

Difficulties: `Single<&mut WyRand>` needs the `rand::Rng` trait in scope
for next_u32 (mirrored asteroid.rs's exact import pattern);
ScenarioConfig's exhaustive literals (broadside builder, loader test)
needed the new field - caught by check --all-targets as the lesson
predicts.

Reflection: the audit's "flag idea" survived contact with the code
almost unchanged, but the real design work turned out to be the FAILURE
paths (well-less backdrops, empty rotation, unregistered declarations) -
none of which were in the plan. Enumerating "what breaks when a mod
does this badly" belongs in planning for every mod-facing surface.
