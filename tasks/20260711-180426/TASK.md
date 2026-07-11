# Main menu: MainMenu state, bottom-right panel UI, mode wiring

- STATUS: OPEN
- PRIORITY: 45
- TAGS: v0.5.0,ui,menu,spike

## Goal

Give the game a real front door. Add a `MainMenu` variant to `GameStates`
(Loading -> MainMenu -> Playing), implement the menu in a new `nova_menu`
crate with bevy_ui: a small panel anchored bottom-right with the title
"Nova Protocol" and buttons New Game / Sandbox / Settings / Exit. Existing
examples keep working.

## Steps

- [ ] Add a `MainMenu` variant to `GameStates` between `Loading` and
      `Playing` in `crates/nova_gameplay/src/lib.rs:46`, with a doc comment
      saying the menu crate owns it. Audit the only two
      `OnEnter(GameStates::Playing)` hooks (nova_core `AppBuilder::build`,
      nova_editor `editor_plugin`) for assumptions that Playing follows
      Loading directly.
- [ ] Add a `GameMode` resource in `crates/nova_gameplay/src/lib.rs` next to
      `GameStates` (same shared-vocabulary rationale):
      `enum GameMode { #[default] Sandbox, NewGame }`, exported via the
      prelude.
- [ ] Create the `crates/nova_menu` crate: Cargo.toml (deps: bevy,
      nova_assets, nova_gameplay, nova_scenario; workspace lints/edition),
      `src/lib.rs` with `NovaMenuPlugin` and a `pub mod prelude`. Add it to
      the workspace `members` in the root Cargo.toml and as a dependency +
      prelude re-export in `crates/nova_core`.
- [ ] In `NovaMenuPlugin`, on `OnEnter(GameStates::MainMenu)` spawn a menu
      camera with skybox + post-processing (mirror the scenario camera at
      `crates/nova_scenario/src/loader.rs:162`, minus SfxListenerMarker) and
      the UI root, both tagged `DespawnOnExit(GameStates::MainMenu)`.
      (Task 20260711-180455 later replaces this camera with the ambient
      scenario; keep the spawn isolated so it is easy to swap.)
- [ ] Build the panel with bevy_ui in the editor sidebar style
      (`crates/nova_editor/src/lib.rs:363`): absolute-positioned node with
      `right`/`bottom` insets, "Nova Protocol" title, buttons New Game,
      Sandbox, Settings, Exit (Exit gated `#[cfg(not(target_arch =
      "wasm32"))]`), using `Button` + `observe` click observers.
- [ ] Wire the buttons: New Game sets `GameMode::NewGame` and
      `NextState<GameStates>::Playing`; Sandbox sets `GameMode::Sandbox` and
      Playing; Settings toggles a placeholder sub-panel (title + Back, same
      DespawnOnExit root); Exit sends `AppExit`.
- [ ] New Game scenario load: in nova_menu, `OnEnter(GameStates::Playing)`
      gated on `GameMode::NewGame`, trigger
      `LoadScenario(GameScenarios["asteroid_field"])` (it already contains
      the canned player ship; pattern in `examples/03_scenario.rs:33`).
- [ ] Gate the editor: in `crates/nova_editor/src/lib.rs:46`, enter
      `ExampleStates::Editor` on `OnEnter(GameStates::Playing)` only when
      `GameMode::Sandbox`. Default `GameMode` is Sandbox, so editor apps
      that skip the menu behave exactly as today.
- [ ] `AppBuilder` wiring in `crates/nova_core/src/lib.rs`: add a
      `with_main_menu(bool)` builder flag defaulting to on only for the
      default (editor) app; when on, add `NovaMenuPlugin` and make the
      `OnEnter(GameAssetsStates::Loaded)` hook set `MainMenu` instead of
      `Playing`. Examples using `with_game_plugins` therefore skip the menu
      with zero changes.
- [ ] Keep `examples/09_editor.rs` smoke harness green: it shares
      `editor_app` with the binary, so under the harness env (BCS_AUTOPILOT /
      BCS_SHOT) add a debug-feature system that auto-advances MainMenu ->
      Playing with `GameMode::Sandbox`. Verify how the harness drives states
      before coding (plan-from-the-system rule).
- [ ] Tests: add an integration test (or extend the smoke harness) that
      builds the app with the menu enabled, steps to MainMenu, and asserts
      (a) New Game path ends in Playing + NewGame + scenario loaded, and
      (b) Sandbox path ends in Playing + editor state entered.
- [ ] Run check/fmt and the newly written tests (skip full local
      suite/clippy per repo policy; CI covers it).
- [ ] Docs: update docs/architecture.md (crate map row for nova_menu, the
      States section) and CHANGELOG.md; append a Fix record line to
      docs/spikes/20260711-180500-main-menu.md.

## Notes

- Spike: docs/spikes/20260711-180500-main-menu.md
- Parent task: 20260711-174915
- Only two OnEnter(GameStates::Playing) hooks exist today: nova_core
  AppBuilder (status UI, actually registered on Loaded) and nova_editor
  (enters ExampleStates::Editor).
- "asteroid_field" is registered in crates/nova_assets/src/scenario.rs and
  already spawns a player ship; New Game needs no new content.
- The "sandbox_field without hostiles" idea from the spike: the editor's
  Play button uses its own test_scenario() in nova_editor; check during
  work whether it even contains hostiles before adding a variant - if the
  only other ship is SpaceshipController::None (as in asteroid_field), no
  variant is needed and the step is a no-op.
- bevy_ui only; no egui. DespawnOnExit is the established cleanup pattern.
