# Main menu: MainMenu state, bottom-right panel UI, mode wiring

- STATUS: CLOSED
- PRIORITY: 45
- TAGS: v0.5.0,ui,menu,spike

## Goal

Give the game a real front door. Add a `MainMenu` variant to `GameStates`
(Loading -> MainMenu -> Playing), implement the menu in a new `nova_menu`
crate with bevy_ui: a small panel anchored bottom-right with the title
"Nova Protocol" and buttons New Game / Sandbox / Settings / Exit. Existing
examples keep working.

## Steps

- [x] Add a `MainMenu` variant to `GameStates` between `Loading` and
      `Playing` in `crates/nova_gameplay/src/lib.rs:46`, with a doc comment
      saying the menu crate owns it. Audit the only two
      `OnEnter(GameStates::Playing)` hooks (nova_core `AppBuilder::build`,
      nova_editor `editor_plugin`) for assumptions that Playing follows
      Loading directly.
- [x] Add a `GameMode` resource in `crates/nova_gameplay/src/lib.rs` next to
      `GameStates` (same shared-vocabulary rationale):
      `enum GameMode { #[default] Sandbox, NewGame }`, exported via the
      prelude.
- [x] Create the `crates/nova_menu` crate: Cargo.toml (deps: bevy,
      nova_assets, nova_gameplay, nova_scenario; workspace lints/edition),
      `src/lib.rs` with `NovaMenuPlugin` and a `pub mod prelude`. Add it to
      the workspace `members` in the root Cargo.toml and as a dependency +
      prelude re-export in `crates/nova_core`.
- [x] In `NovaMenuPlugin`, on `OnEnter(GameStates::MainMenu)` spawn a menu
      camera with skybox + post-processing (mirror the scenario camera at
      `crates/nova_scenario/src/loader.rs:162`, minus SfxListenerMarker) and
      the UI root, both tagged `DespawnOnExit(GameStates::MainMenu)`.
      (Task 20260711-180455 later replaces this camera with the ambient
      scenario; keep the spawn isolated so it is easy to swap.)
- [x] Build the panel with bevy_ui in the editor sidebar style
      (`crates/nova_editor/src/lib.rs:363`): absolute-positioned node with
      `right`/`bottom` insets, "Nova Protocol" title, buttons New Game,
      Sandbox, Settings, Exit (Exit gated `#[cfg(not(target_arch =
      "wasm32"))]`), using `Button` + `observe` click observers.
- [x] Wire the buttons: New Game sets `GameMode::NewGame` and
      `NextState<GameStates>::Playing`; Sandbox sets `GameMode::Sandbox` and
      Playing; Settings toggles a placeholder sub-panel (title + Back, same
      DespawnOnExit root); Exit sends `AppExit`.
- [x] New Game scenario load: in nova_menu, `OnEnter(GameStates::Playing)`
      gated on `GameMode::NewGame`, trigger
      `LoadScenario(GameScenarios["asteroid_field"])` (it already contains
      the canned player ship; pattern in `examples/03_scenario.rs:33`).
- [x] Gate the editor: in `crates/nova_editor/src/lib.rs:46`, enter
      `ExampleStates::Editor` on `OnEnter(GameStates::Playing)` only when
      `GameMode::Sandbox`. Default `GameMode` is Sandbox, so editor apps
      that skip the menu behave exactly as today.
- [x] `AppBuilder` wiring in `crates/nova_core/src/lib.rs`: add a
      `with_main_menu(bool)` builder flag defaulting to on only for the
      default (editor) app; when on, add `NovaMenuPlugin` and make the
      `OnEnter(GameAssetsStates::Loaded)` hook set `MainMenu` instead of
      `Playing`. Examples using `with_game_plugins` therefore skip the menu
      with zero changes.
- [x] Keep `examples/09_editor.rs` smoke harness green: it shares
      `editor_app` with the binary, so under the harness env (BCS_AUTOPILOT /
      BCS_SHOT) add a debug-feature system that auto-advances MainMenu ->
      Playing with `GameMode::Sandbox`. Verify how the harness drives states
      before coding (plan-from-the-system rule).
- [x] Tests: add an integration test (or extend the smoke harness) that
      builds the app with the menu enabled, steps to MainMenu, and asserts
      (a) New Game path ends in Playing + NewGame + scenario loaded, and
      (b) Sandbox path ends in Playing + editor state entered.
- [x] Run check/fmt and the newly written tests (skip full local
      suite/clippy per repo policy; CI covers it).
- [x] Docs: update docs/architecture.md (crate map row for nova_menu, the
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

## Close record (2026-07-11)

What changed:
- GameStates gained MainMenu (Loading -> MainMenu -> Playing for the default
  app); GameMode { Sandbox (default), NewGame } lives beside it in
  nova_gameplay and is init'd by NovaGameplayPlugin.
- New crates/nova_menu crate (NovaMenuPlugin): skybox menu camera + bottom
  -right panel (title, New Game/Sandbox/Settings/Exit) in bevy_ui, editor
  palette; Settings toggles a centered placeholder panel; Exit is compiled
  out on wasm. Buttons write GameMode and set Playing; OnEnter(Playing)
  gated on NewGame triggers LoadScenario(asteroid_field).
- nova_editor enters ExampleStates::Editor only in GameMode::Sandbox; its
  play-test scenario's other ship became a passive target (was AI) per the
  spike's sandbox scope - the Notes' "check first" resolved to: yes, it had
  a hostile.
- AppBuilder: with_main_menu(bool) override; default on only for the
  default (editor) app, so all examples with custom plugins keep
  Loading -> Playing untouched. 09_editor's autopilot clicks the Sandbox
  button when it sees MainMenu, which doubles as menu smoke coverage.

Verification:
- cargo check --workspace (+ --all-targets --features debug) clean;
  cargo fmt applied.
- cargo test -p nova_menu: 2/2 pass (New Game handoff loads the scenario -
  delivery guard; Sandbox handoff loads nothing).
- BCS_AUTOPILOT 09_editor run: clicked Sandbox -> reached Playing ->
  created ship -> placed section -> clean exit.
- Xvfb screenshot confirms the rendered panel bottom-right with all four
  buttons.
- Skipped per repo policy: full local test suite and clippy (CI runs them).

Reflection: the harness docs (nova_debug/src/harness.rs) answered the
"how do examples survive a new state" question before any code was
written - reading them first avoided a wrong builder-flag detour for
09_editor. The tatr same-second collision cost a few minutes at task
creation (known gotcha, now hit twice).

## Review round 1 addendum (2026-07-11)

Review found one BLOCKER the original verification missed (it only
exercised the Sandbox path): in NewGame the editor's ExampleStates stayed
Loading, and the spaceship input/section system sets are gated on
ExampleStates::Scenario, so the New Game scenario was unflyable. Fixed by
routing NewGame -> ExampleStates::Scenario with the editor's own
setup_scenario gated to Sandbox. Evidence (throwaway harness: boot default
app, click the real New Game button, hold thrust 120 frames, record max
player speed): pre-fix 2.88 (residual physics drift only), post-fix 51.40.
Durable regressions: nova_editor tests pin the NewGame -> Scenario routing
+ zero editor scenario loads, and Sandbox -> Editor routing; nova_menu
gained real_new_game_button_is_wired (clicks the button setup_menu_ui
actually spawns). Also from review: the Loaded hook only advances from
Loading (protects BCS_SHOT force-advance), GameMode registered for
reflection, menu panel now Absolute-positioned with right/bottom insets.
