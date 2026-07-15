# Scenario picker: a Scenarios menu modal in the mods-screen style - list + details pane, play any registered scenario

- STATUS: CLOSED
- PRIORITY: 12
- TAGS: feature,menu,scenario

User request (20260715, on seeing the new mods screen): "the mods page looks
really good! we should add a similar style page for playing custom scenarios
(new game just plays the main story) but maybe we should add a 'scenarios'
button that opens a modal that let's you choose a scenario to play and
similarly it shows details and info about it with description name image etc".

Goal: a "Scenarios" main-menu button opening a modal in the mods-screen style
(GlobalZIndex overlay, list pane + details pane): the list shows every
PLAYABLE registered scenario from `GameScenarios` (base story scenarios plus
whatever enabled mods added - demo_mod_arena, gauntlet_run once installed);
selecting one shows details (name, description, source mod?, image); a Play
button loads exactly that scenario (LoadScenario + the New-Game-style state
handoff) instead of the main story chain. New Game stays untouched.

Known design questions for /plan (capture, do not decide here):
- IMAGE: ScenarioConfig has no image metadata today (id/name/description/
  cubemap/events). Needs a small schema addition - e.g. an optional
  `thumbnail: Option<String>` asset path on ScenarioConfig (serde-defaulted,
  same back-compat discipline as ModMeta) - plus authoring for the base
  scenarios, OR phase 1 ships text-only details like the mods screen did.
- FILTERING: some registered scenarios are not player-facing (menu_ambience;
  possibly chained mid-story entries) - likely needs a `listed: bool` or
  similar flag, mirroring the catalog's `hidden` lesson (and its session-only
  semantics discussion).
- PLAY WIRING: confirm how New Game hands off (GameMode + state) and what
  "play one scenario, then what" means at the end (return to menu?).
- Reuse: the two-pane markers/systems from 142911 (nova_menu) are the
  template; consider extracting shared list+details scaffolding rather than
  copy-pasting a second screen.

Related: the 142911 mods screen (style template), spike
tasks/20260714-202515/SPIKE.md (family context), 13_screenshot_reel
(LoadScenario-directly precedent).

## Decisions (user, 20260715)

- IMAGE: add the schema field now. `thumbnail: Option<AssetRef<Image>>` on
  `ScenarioConfig` (serde-defaulted None, skip-if-None), rendered in the
  details pane as an `ImageNode`. `AssetRef<Image>` (not a bare String) so it
  authors as a path and resolves through `AssetServer` exactly like `cubemap`,
  and a mod-authored thumbnail gets whatever path handling `cubemap` gets.
  Base scenarios point at existing textures as PLACEHOLDER art (real
  per-scenario thumbnails are a follow-up task, filed below).
- FILTERING: add `hidden: bool` to `ScenarioConfig` (serde default false,
  skip-if-false), mirroring the mods-catalog `hidden` flag. The picker lists
  only `!hidden` scenarios.
- MID-CHAIN: hide continuations. Listed base scenarios = `shakedown_run` (the
  New Game / main-story start) and `demo`. Hidden = `menu_ambience` (backdrop),
  `asteroid_field` and `asteroid_next` (both reached only via NextScenario
  chaining from shakedown, not entry points). All flags are data-driven and
  trivially flippable if the curation should change.
- PLAY WIRING: reuse the New Game handoff, no `GameMode` enum change (keeps its
  exhaustive matches in nova_editor intact and it stays `Copy`). A new
  `NewGameScenario(Option<ScenarioId>)` resource overrides which scenario the
  existing `start_new_game_scenario` (OnEnter(Playing), gated `GameMode::NewGame`)
  loads: `None` -> the canned `NEW_GAME_SCENARIO_ID`, `Some(id)` -> that id.
  The Scenarios "Play" button sets `Some(id)` + `GameMode::NewGame` + Playing;
  `on_new_game` resets it to `None` so New Game always starts the story. End
  behavior is unchanged (the scenario's own NextScenario chaining / natural
  end), so "play one scenario then what" needs no new work here.
- REUSE: keep the scenarios screen INLINE in nova_menu/src/lib.rs next to the
  mods screen rather than extracting a shared two-pane abstraction. The mods
  screen is tightly coupled to private menu helpers (`button`/`themed_button`/
  `theme`/`observe`/`Selected`) and carries tab/portal/install-job complexity
  the scenarios screen does not need; a premature shared abstraction would leak
  that. Second instance is not yet rule-of-three. Consistency with the existing
  inline mods screen wins.

## Steps

### Schema (nova_scenario)

- [x] In `crates/nova_scenario/src/loader.rs` add to `ScenarioConfig`:
  `thumbnail: Option<AssetRef<Image>>` (serde `default`, `skip_serializing_if
  Option::is_none`) and `hidden: bool` (serde `default`, `skip_serializing_if`
  a local `is_false`). Derive `Default` on the struct so downstream literals
  can use `..Default::default()`. Document the RON syntax in the field docs
  (`hidden: true`, `thumbnail: Some("banner.png")` - never bare in strict RON;
  author-facing-schema-needs-syntax-doc lesson). Import `AssetRef`/`Image`.
- [x] Add a serde back-compat unit test in loader.rs: a `Scenario` RON WITHOUT
  the two fields deserializes (defaults None/false); one WITH them round-trips
  (`hidden: true`, `thumbnail: Some("x.png")`).

### Fix every ScenarioConfig construction site (check-all-targets-for-struct-field)

- [x] Base builders `crates/nova_assets/src/scenario.rs` +
  `.../scenario/shakedown.rs`: set explicit `hidden`/`thumbnail`:
  menu_ambience `hidden: true`; asteroid_field `hidden: true`; asteroid_next
  `hidden: true`; shakedown_run `hidden: false` + a placeholder
  `thumbnail: Some("banner.png".into())`.
- [x] Add `..Default::default()` (or explicit `hidden: false, thumbnail: None`)
  to the ~16 example + editor + test literals (grep `ScenarioConfig {`), incl.
  nova_menu `dummy_scenario` and loader.rs test helpers.
- [x] Regenerate the four generated base files: delete
  `assets/base/scenarios/{menu_ambience,asteroid_field,asteroid_next,shakedown_run}.content.ron`
  and run `content_ron_parity` (writes on missing), then run it again to prove
  parity. (generate-data-from-code discipline.)
- [x] Hand-edit `assets/base/scenarios/demo.content.ron`: `hidden: false`
  (listed) + a placeholder thumbnail. Add a thumbnail to
  `webmods/gauntlet/gauntlet.content.ron` (shown by default; mod-side asset).

### Play handoff (nova_menu)

- [x] Add `NewGameScenario(pub Option<ScenarioId>)` resource (init in the
  plugin). Modify `start_new_game_scenario` to load `NewGameScenario.0` if
  `Some` (missing id -> warn + fall back to NEW_GAME_SCENARIO_ID), else the
  canned id. `on_new_game` sets it `None`.

### Scenarios modal UI (nova_menu, inline)

- [x] Markers `ScenariosPanel`/`ScenariosList`/`ScenarioRow{id}`/
  `ScenarioDetailsPanel`/`ScenarioDetailsActions`; resource
  `SelectedScenarioId(Option<ScenarioId>)`.
- [x] Add a "Scenarios" main-menu button (`button` + `observe(on_scenarios)`)
  next to Mods in `setup_menu_ui`.
- [x] Spawn the `ScenariosPanel` overlay (GlobalZIndex(1), Visibility::Hidden,
  85% panel, two panes, Back button) mirroring `ModsPanel`; reset
  `SelectedScenarioId` on entry to re-arm the refreshers.
- [x] `on_scenarios` toggles panel visibility (mirrors `on_mods`);
  `on_scenarios_back` hides it.
- [x] `refresh_scenarios_list` (run_if dirty): despawn+rebuild `ScenariosList`
  rows from `GameScenarios` filtered `!hidden`, sorted by name; each row a
  clickable ThemedButton carrying `ScenarioRow{id}` + `Selected` highlight;
  default-select the first listed scenario / repair a stale selection.
- [x] `on_scenario_row_select` sets `SelectedScenarioId` + moves the highlight.
- [x] `refresh_scenario_details` (run_if dirty): rebuild `ScenarioDetailsPanel`
  from the selected scenario - name, description, source tag (mod vs base),
  the `thumbnail` `ImageNode` if present, and a Play button in
  `ScenarioDetailsActions`. Empty fallback keeps the action container present.
- [x] `on_scenario_play` -> set `NewGameScenario = Some(id)`, `GameMode::NewGame`,
  `GameStates::Playing`.
- [x] `scenarios_list_dirty`/`scenario_details_dirty` (GameScenarios changed /
  SelectedScenarioId changed); register the chained refreshers in Update gated
  `in_state(MainMenu)`, next to the mods refreshers.

### Tests (nova_menu)

- [x] The list shows only `!hidden` scenarios (build a `GameScenarios` with one
  listed + one hidden, enter menu / run refresh, assert row ids). Fails if the
  filter is dropped.
- [x] Selecting a row updates `SelectedScenarioId` (and the details refresh
  renders that scenario's name).
- [x] Play sets `NewGameScenario`, `GameMode::NewGame`, transitions to Playing,
  AND (delivery guard) `start_new_game_scenario` fires `LoadScenario` for the
  CHOSEN id - not the canned one.
- [x] `start_new_game_scenario` honors the override: `Some(id)` loads that id;
  `None` loads NEW_GAME_SCENARIO_ID; a `Some(bogus)` falls back with a warn.
- [x] `on_new_game` resets `NewGameScenario` to `None` (so New Game after using
  the picker still starts the story).

### Visual verification (render-output-eyeball, UI variant)

- [x] Drive the real menu under Xvfb, open the Scenarios panel, screenshot, and
  Read the PNG to confirm layout / z-order over the menu card / thumbnail
  renders. (Mirrors the 142911 verification.)

### Docs

- [x] `tasks/<id>/NOTES.md`: schema additions + RON syntax, the
  NewGameScenario-override handoff design, the listing curation + why, the
  placeholder-thumbnail decision + follow-up pointer.
- [x] Update CHANGELOG and the scenario/mod schema reference doc
  (web/src/wiki/dev/*) with the new fields.
- [x] File a follow-up tatr task: real per-scenario thumbnail art/generation
  (e.g. via the screenshot-reel infra) to replace the placeholders.

## Notes

- Anchors (from an Explore pass): `ScenarioConfig`
  crates/nova_scenario/src/loader.rs:28; `GameScenarios` loader.rs:22;
  `start_new_game_scenario` nova_menu/src/lib.rs:2110 (gated at :147);
  `on_new_game` :1167; `on_mods`/`on_mods_back` :1206/:1213; `ModsPanel` spawn
  :856-1030; markers :368-434; `refresh_mods_list` :1481; `refresh_mod_details`
  :1866; dirty conds :1275/:1292; `on_mod_row_select` :1251; `button` :2149;
  base scenario builders + `build_scenarios` nova_assets/src/lib.rs:73;
  content parity test crates/nova_assets/tests/content_ron_parity.rs.
- Chaining: shakedown_run -> asteroid_field -> asteroid_next -> asteroid_field
  (loop). Only shakedown_run is a New Game entry point.
- `AssetRef` (crates/nova_gameplay/src/asset_ref.rs) (de)serializes as a bare
  path string; `Option<AssetRef<Image>>` authors as `Some("path.png")`.
- check-all-targets: verify with `cargo check --workspace --all-targets` and
  `cargo test --workspace --no-run` (a new non-Default field breaks examples
  AND `#[cfg(test)]` code that plain check skips).

## Close-out

Shipped the Scenarios picker end to end. Design detail lives in NOTES.md; this
is the what/why/evidence summary.

- What changed: `ScenarioConfig` gained `thumbnail: Option<AssetRef<Image>>` and
  `hidden: bool` (serde-defaulted, `Default`-derived); base builders set the
  flags and shakedown a placeholder thumbnail; the four generated
  `*.content.ron` were regenerated and demo/gauntlet hand-edited. nova_menu got
  a Scenarios main-menu button + a two-pane overlay (list of `!hidden`
  scenarios, details pane with name/description/thumbnail/Play) mirroring the
  mods screen, plus a `NewGameScenario` override so Play reuses the New Game
  handoff without a `GameMode` change. ~16 example/editor/test literals took
  `..Default::default()`. CHANGELOG + author guide updated.
- Alternatives considered: a `GameMode::Scenario(id)` variant (rejected -
  breaks `Copy` and nova_editor's exhaustive matches; the override resource is
  cleaner); a shared two-pane abstraction extracted from the mods screen
  (rejected - not rule-of-three, would leak tab/portal complexity); a bare
  `String` thumbnail (rejected - `AssetRef<Image>` gets the same path handling
  as `cubemap`, which matters for mod-authored thumbnails).
- Difficulties: the serde back-compat test first asserted `thumbnail: Some(..)`
  with a space; `ron::to_string` is compact (`thumbnail:Some(..)`) - fixed the
  assertion. The ~16 literal sites were edited by a brace-counting script
  (verified: each inserted exactly once, both literal shapes checked, workspace
  `--all-targets` clean).
- Evidence: nova_menu 37 (6 new), nova_scenario 66 (1 new serde), nova_assets 44
  + demo_scenario 11 + webmods 1 + content parity 2, nova_editor 12 - all green;
  `cargo check --workspace --all-targets` clean; `cargo fmt` clean. Visual: the
  Scenarios panel captured under Xvfb through the real `editor_app` pipeline
  (panel opened by activating the button, shakedown row selected) - the overlay
  renders over the menu card and the thumbnail draws at 16:9. Screenshot saved
  as `scenarios-panel.png` next to this file.
- Reflection: mirroring the 142911 mods-screen structure (markers, dirty-refresh
  chain, GlobalZIndex, selection repair) made this fast and low-risk - the
  z-order fix came for free. The `Option<Res<AssetServer>>` guard on the details
  refresh was the one non-obvious bit: headless test apps have no AssetPlugin,
  so an unconditional `Res<AssetServer>` would have panicked every menu test
  that enters MainMenu. Caught it at design time by remembering the mods details
  refresh takes no AssetServer.
- Deliberately deferred (see NOTES.md): real thumbnail art (task 20260715-220011),
  source-mod provenance tag (GameScenarios has no origin), list wheel-scroll,
  dependency/search UI.
