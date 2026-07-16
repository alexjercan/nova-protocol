# Data-driven menu scenario roles: menu_backdrop flag + base-owned new game start

- STATUS: OPEN
- PRIORITY: 60
- TAGS: feature

## Goal

Remove the last engine-knows-content coupling: the two scenario id consts
in nova_menu. Menu backdrops become a per-scenario `menu_backdrop` flag -
multiple allowed, mods may add them, the menu picks one at random. The
new-game start becomes base-owned config - explicitly NOT moddable.
Decision from audit task 20260716-141620 (FINDINGS.md section 3) with
user direction on the trust split.

## Steps

- [ ] Add `menu_backdrop: bool` to `ScenarioConfig`
      (crates/nova_scenario/src/loader.rs, next to `hidden` :55-61), same
      serde-default treatment; mirror the hidden round-trip serde tests
      (:1585-1624).
- [ ] Add `new_game_scenario: Option<String>` to `BundleManifest`
      (crates/nova_mod_format/src/lib.rs:69+), serde-defaulted; document
      the literal RON syntax in the field doc
      (`new_game_scenario: Some("shakedown_run")` - strict RON needs the
      Some(), author-facing-schema lesson).
- [ ] New resource `NewGameStart(Option<String>)` in nova_scenario next
      to `GameScenarios`. In `register_bundles`
      (crates/nova_assets/src/lib.rs:488+), set it from the
      `new_game_scenario` of bundles whose catalog decl has `base: true`
      ONLY (`entry.decl.base` is available at :507); warn and ignore a
      non-base bundle declaring it. Unit-pin the base-only honoring (a
      non-base bundle's declaration is ignored and warned).
- [ ] Author the data: `menu_backdrop: true` on the menu_ambience builder
      (crates/nova_assets/src/scenario.rs) + regenerate RON via
      `cargo run -p nova_assets --bin gen_content` in the SAME commit;
      `new_game_scenario: Some("shakedown_run")` in the hand-authored
      `assets/base/base.bundle.ron`.
- [ ] nova_menu backdrop path: replace `MENU_AMBIENCE_SCENARIO_ID` -
      `load_menu_ambience` (:971) collects all registered scenarios with
      `menu_backdrop: true` and picks one at random via
      `GlobalEntropy<WyRand>` (add bevy_rand to nova_menu Cargo.toml,
      copying nova_gameplay's target-specific wasm_js pattern;
      EntropyPlugin is registered in production by
      nova_gameplay/src/plugin.rs:48, tests add it themselves). None
      flagged = no backdrop, no panic (replaces the panic at :973-974).
- [ ] nova_menu new-game path: replace `NEW_GAME_SCENARIO_ID` -
      `start_new_game_scenario` (:3030-3059) fallback chain: picked id if
      registered -> `NewGameStart` id if registered -> first listed
      (!hidden) scenario -> log error, load nothing.
- [ ] Delete both consts; update menu tests (picker fixtures :5160+,
      ambience tests around :3200-3760, new-game tests :5291+); add a
      seeded test that the random pick always lands in the flagged set
      (and a multi-backdrop pick reaches more than one over draws).
- [ ] Grep production code for remaining scenario id literals - must be
      none (content-to-content refs inside builders/RON are fine).
- [ ] Docs (keep-docs-in-sync): wiki authoring guide
      (web/src/wiki/dev/guide-author-scenario.md) documents
      `menu_backdrop`; the mod-format/bundle doc documents
      `new_game_scenario` and its base-only trust rule.
- [ ] Verify: `cargo check --workspace --all-targets`, `cargo fmt`, run
      the touched loader/menu/assets/mod_format tests (crate-solo lesson
      for nova_scenario/nova_menu).

## Notes

- Explicitly rejected (user): a scenario flag or any last-mod-wins
  overlay for the new-game start; non-base mods must not control it.
- `hidden` stays orthogonal: backdrops remain hidden from the picker.
- Depends on: 20260716-155823 (gen_content bin for the RON regeneration).
