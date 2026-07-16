# Notes: Pause menu Retry button

## What changed

The pause overlay (Esc / gamepad Start during Playing) gained a Retry button
between Resume and Back to Main Menu (`crates/nova_menu/src/lib.rs`):

- `setup_pause_ui` now takes `Option<Res<CurrentScenario>>` and spawns the
  button only when a scenario is live. The Option keeps headless menu rigs
  (no scenario loader) working, same pattern as the outcome overlay's
  resources.
- New `on_retry` observer: clones the live `ScenarioConfig` out of
  `CurrentScenario`, re-triggers `LoadScenario` with it, and sets
  `PauseStates::Unpaused` in the same activation.

Docs in the same task: `web/src/tutorial.html` (the Esc line now lists
Retry), CHANGELOG.md under Unreleased > Interface & HUD. The keybinds wiki
page needed nothing (no new key; its "Pause menu: Esc" row stays true).

## Why this design

- **Reload via `LoadScenario(current)`, not a state round-trip.** The loader
  already owns restart semantics: `on_load_scenario` runs
  `teardown_scenario_entities` first, which clears the event world (variables,
  queued `NextScenario`), the declared outcome, HUD emphasis, and despawns
  every `ScenarioScopedMarker` entity. This is exactly the path the outcome
  overlay's Defeat-Retry takes (via a lingering NextScenario), so pause-Retry
  and defeat-Retry behave identically. Alternative considered: bouncing
  through GameStates (Playing -> MainMenu -> Playing) - rejected, it drags the
  menu ambience load/unload and HUD visibility churn along for no benefit.
- **Button hidden without a live scenario.** The editor's build mode pauses
  through the same overlay but never has a scenario loaded (`CurrentScenario`
  is None there - that liveness gate is what keeps its preview inert), so a
  Retry would be a dead button. Same reasoning covers the FAILED TO START
  case after a refused first load.
- **Cursor handling is free.** `regrab_cursor_on_player_spawn` was built for
  exactly this shape ("a Retry reloads the scenario WITHOUT a state
  transition") - the new ship's spawn re-grabs the pointer once unpaused.
  `restore_cursor` on OnExit(Paused) sees no player ship mid-teardown and
  correctly does nothing.
- **Works for both modes.** NewGame (menu-loaded scenario) and the editor's
  Sandbox play-test both go through `LoadScenario`, so `CurrentScenario`
  holds the right config either way; a sandbox retry rebuilds the same
  asteroid field with the player's built ship (the config snapshots the ship).

## Verification

- `cargo check` + `cargo fmt` clean.
- Two new tests in nova_menu (run, both pass):
  `pause_overlay_offers_retry_only_over_a_live_scenario`,
  `pause_retry_reloads_the_current_scenario_and_unpauses`.
- Full `cargo test` / clippy deliberately skipped per repo rule; CI is the
  source of truth.
- `cd web && npm run lint && npm run build` pass. `npm run format:check`
  flags four files (`src/wiki-pages.ts`, `src/index.html`, `src/news.html`,
  `markdown.js`) that are already unformatted on master HEAD (627f5e43) -
  pre-existing drift, not touched by this task, left for its owner.

## Difficulties

- One test-only compile error: writing the two-rig test as two successive
  `let mut app = app();` bindings shadowed the `app()` helper with the first
  `App` value. Restructured into a `paused_app(CurrentScenario)` closure with
  distinctly named rigs.

## Self-reflection

- The Explore-agent-first approach paid off: the outcome overlay's Retry and
  the cursor-regrab observer were the load-bearing precedents, and finding
  them before designing avoided inventing a second restart mechanism.
- Could have gone better: I only thought to check prettier on the web tree
  because the repo AGENTS.md demands `npm run ci` - good rule, but the
  pre-existing drift cost a diagnosis round; checking `git status` of the
  flagged files against HEAD settled it quickly.
