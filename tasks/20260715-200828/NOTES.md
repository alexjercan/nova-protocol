# Scenario picker - design notes

Task 20260715-200828. A "Scenarios" main-menu modal in the mods-screen style:
list of playable scenarios on the left, details + Play on the right.

## Schema additions (nova_scenario `ScenarioConfig`)

Two serde-defaulted fields, so every scenario authored before this change still
parses unchanged:

- `thumbnail: Option<AssetRef<Image>>` - an image shown in the picker's details
  pane. Typed `AssetRef<Image>` (not a bare `String`) so it authors as a path
  and resolves through `AssetServer` exactly like `cubemap`; a mod-authored
  thumbnail therefore gets the same path handling a mod's cubemap does. In
  strict RON an `Option` is written with the variant: `thumbnail: Some("x.png")`
  (never bare) - documented in the field doc and the author guide.
- `hidden: bool` (default false) - keeps a scenario out of the picker. Mirrors
  the mods-catalog `hidden` flag. `skip_serializing_if` keeps it out of the
  serialized RON when false.

`ScenarioConfig` gained `#[derive(Default)]` so the ~16 example/editor/test
literals that construct it could take `..Default::default()` for the two new
fields instead of each spelling them out. (`check-all-targets-for-struct-field`:
a new non-Default field breaks examples AND `#[cfg(test)]` code that a plain
`cargo check --workspace` skips - verified with `--all-targets` and the test
run.)

## Listing curation (all data-driven, trivially flippable)

Listed base scenarios: `shakedown_run` (the New Game / main-story start) and
`demo`. Hidden: `menu_ambience` (the menu backdrop) and `asteroid_field` /
`asteroid_next` (both reached only via `NextScenario` chaining from the
shakedown run - continuations, not entry points). The chain is
shakedown_run -> asteroid_field -> asteroid_next -> asteroid_field (loop); only
shakedown_run is an entry point. Mod scenarios (e.g. `gauntlet_run`) are listed
by default (`hidden` defaults false).

## Play handoff (no GameMode change)

The picker reuses the New Game path rather than adding a `GameMode` variant
(which would break nova_editor's exhaustive matches and its `Copy`-ness). A new
`NewGameScenario(Option<ScenarioId>)` resource overrides which scenario the
existing `start_new_game_scenario` (OnEnter(Playing), gated `GameMode::NewGame`)
loads:

- `None` -> the canned `NEW_GAME_SCENARIO_ID` (shakedown_run).
- `Some(id)` -> that scenario; a `Some(missing)` (a mod disabled between pick
  and play) falls back to the canned start with a `warn!`, never panics.

The Play button sets `Some(id)` + `GameMode::NewGame` + `GameStates::Playing`;
`on_new_game` clears it to `None` so New Game always starts the story even after
the picker was used. End-of-scenario behavior is unchanged (the scenario's own
`NextScenario` chaining / natural end), so "play one scenario then what" needed
no new work.

## UI (inline in nova_menu, mirroring the mods screen)

Kept inline next to the mods screen rather than extracting a shared two-pane
abstraction: the mods screen is tightly coupled to private menu helpers
(`button`/`themed_button`/`theme`/`observe`/`Selected`) and carries tab/portal/
install-job complexity the picker does not need. Second instance is not
rule-of-three; a premature abstraction would leak that complexity. Same
`GlobalZIndex(1)` overlay + 85% two-pane layout + dirty-refresh chain as the
mods screen (the 142911 R1.1 z-order fix is mirrored, not re-derived).

Markers: `ScenariosPanel`, `ScenariosList`, `ScenarioRow{id}`,
`ScenarioDetailsPanel`, `ScenarioDetailsActions`, `ScenarioPlay{id}`. Resources:
`SelectedScenarioId`, `NewGameScenario`. Systems: `on_scenarios`/
`on_scenarios_back` (toggle), `refresh_scenarios_list`/`refresh_scenario_details`
(chained, run on `scenarios_list_dirty`/`scenario_details_dirty` -
GameScenarios-changed or selection-changed), `on_scenario_row_select`,
`on_scenario_play`. `refresh_scenario_details` guards `Option<Res<AssetServer>>`
so headless test apps (no AssetPlugin) run it without panicking and simply skip
the thumbnail `ImageNode`.

## Deliberately deferred

- Real per-scenario thumbnail art: base scenarios point at existing placeholder
  textures (banner.png / asteroid.png / cubemap.png) just to exercise the render
  path. Follow-up: task 20260715-220011.
- Source-mod tag in the details pane ("added by mod X"): `GameScenarios` carries
  no provenance (the merge flattens base + mods into one id-keyed map), so there
  is nothing to key off without adding origin tracking. Left out of phase 1.
- Wheel-scroll on the scenario list: the list has `Overflow::scroll_y` but no
  wheel-scroll system (the mods one is `ModsList`-scoped). Listed scenario counts
  are small; revisit if a big mod set warrants it.
- Dependency display, search box: out of scope (see mods-screen deferrals).

## Verification

- nova_menu 37 tests (6 new: list-filters-hidden, default-select+details,
  row-click select, Play delivery-guard loads the CHOSEN id, override
  honored/fallback, New Game clears override). nova_scenario 66 (1 new serde
  back-compat). nova_assets 44 + demo_scenario 11 + webmods 1 + content parity 2
  green (content edits safe). `cargo check --workspace --all-targets` clean.
- Visual: Scenarios panel captured under Xvfb (real editor_app pipeline, panel
  opened by activating the button) - see the RETRO / capture note. Confirms the
  overlay renders over the menu card and the thumbnail draws.
