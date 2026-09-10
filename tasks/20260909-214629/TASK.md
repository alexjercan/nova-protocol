# Lifecycle defects: quit to menu, refused loads, failed assets, overlays

- STATUS: OPEN
- PRIORITY: 80
- TAGS: v0.14.0, bug, menu, scenario, review

## Goal

Fix the lifecycle and state-transition defects found by the 2026-09-09
read of `nova_core`, `nova_menu`, `nova_scenario::loader`, `nova_assets`
and `nova_os_ui`. Each one is reached by a player action a store build
sees on day one: quit to menu, a failed asset, a refused scenario, ESC on
the wrong frame. Line numbers are from the read and will drift.

Every item gets a `bug_` range (or an invariant on the session range in
`20260909-213441`) that fails before the fix, then the fix, one commit.
Owner (2026-09-09): "create systems examples for all edge cases and bugs
we find."

## Confirmed by reading

- [ ] `crates/nova_core/src/lib.rs:473` `setup_status_ui` runs on every
      `OnEnter(GameAssetsStates::Loaded)` with no teardown: each quit to
      menu adds a status-bar root, and `insert_status_bar_item`'s
      `Single<StatusBarRootMarker>` (`nova_ui/src/status_bar.rs:204`) is
      unsatisfied from the second load on. Leak plus missing FPS/version
      items. Despawn on exit or spawn once.
- [ ] `crates/nova_menu/src/lib.rs:257` `OnExit(GameStates::Playing)`
      writes `ReloadContent` unconditionally, so "Back to Main Menu" loads
      the backdrop, tears it down, shows the boot screen, and builds the
      menu again with a different backdrop. One wasted scenario load and a
      visible flash per quit. Reload only when content changed.
- [ ] `crates/nova_core/src/lib.rs:466,422` the `boot_to_menu` closure is
      computed once and re-fires on every restart: launched with
      `--scenario <id>`, "Back to Main Menu" bounces straight back into the
      scenario, forever. Gate the hook on the first boot.
- [ ] `crates/nova_scenario/src/loader/lifecycle.rs:245,254` the content
      gate refuses a load BEFORE teardown, so a campaign's refused next
      scenario leaves the previous one simulating under a modal
      `should_block_lower` overlay (`nova_menu/src/outcome.rs:272`) with the
      cursor still locked. Only ESC -> pause -> Back escapes. Tear down
      first, and free the cursor on the failure overlay as the outcome
      overlay does (`outcome.rs:366`).
- [ ] `crates/nova_assets/src/plugin.rs:141,235`
      `OnEnter(GameAssetsStates::Failed)` only logs, and the loading screen
      despawns only on `Loaded`: a truncated patch or a quarantined file
      leaves the boot screen animating forever with no message and no
      exit. Show the failure and offer quit.
- [ ] `crates/nova_menu/src/outcome.rs:114` vs `pause.rs:277` the outcome
      overlay is z 9 under the pause overlay's z 10 on the assumption they
      never coexist, but ESC on the frame the `Outcome` action is still in
      `queued_commands` (drained on a 3 ms budget, `world.rs:489`) opens
      the pause over the banner, and `toggle_pause` then returns early on a
      live outcome (`pause.rs:63`). Order the two and make `sync_outcome_pause`
      re-run when the pause already holds.
- [ ] `crates/nova_menu/src/outcome.rs:209` `auto_advance_outcome` ticks
      `Time<Real>` and cannot be paused, since the pause refuses to open on
      a live outcome: alt-tab on the banner and the chain advances without
      the player. Hold the timer on focus loss, or let the pause open.
- [ ] `crates/nova_menu/src/ambience.rs:112` the "no clean backdrop" branch
      returns without `UnloadScenario`, and `load_menu_ambience` is the only
      teardown on `Playing -> MainMenu`: a mod that overlays
      `menu_backdrop` with a lint error leaves the gameplay scenario
      simulating behind the menu, then a black fallback camera for the
      session. Unload first, then fall back.
- [ ] `crates/nova_menu/src/pause.rs:108` + `lib.rs:218` `open_command_shell`
      has no `GameStates` gate: `:` on the main menu freezes the backdrop
      and blocks the buttons until ESC. Gate on Playing.

## Plausible, verify by running

- [ ] `crates/nova_os_ui/src/terminal/style.rs:190` `DRAWER_PANEL_Z` 11
      ties the pause Settings panel's `GlobalZIndex(11)` (`pause.rs:360`);
      both are full-screen blockers, so ESC -> Settings -> `:` leaves the
      top modal to traversal order. Give them distinct z.
- [ ] `crates/nova_menu/src/pause.rs:373` the Settings panel is a hard
      620 px with no width cap and `window_plugin` sets no
      `resize_constraints` (`nova_core/src/lib.rs:583`): a 500 px window or
      a narrow wasm iframe overflows the keybind rows off both edges. Cap
      the width and add a minimum window size.

## Judged solid, leave alone

Rebinding and conflict resolution, settings persistence and back-compat,
scenario teardown (`try_despawn`, `NovaEventWorld::clear`), deferred timer
clamps, mod merge, New Game scenario resolution, the NOVA OS render target
clamp, wasm `Instant` imports.

## Proof

`system_session_loop` (`20260909-213441`) carries the status bar count,
the single backdrop load, and the `--scenario` return; the refused-next-
scenario and failed-asset cases are `bug_` ranges of their own; the
outcome-under-pause case is a `bug_` range that queues an outcome behind a
heavy spawn and presses ESC.
