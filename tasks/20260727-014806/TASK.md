# NOVA OS topbar FPS; hide flight status bar in drawer

- PRIORITY: 5
- TAGS: v0.9.0, feature, ui, hud
- KIND: TASK
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

While the NOVA OS ship-computer drawer is open (game paused in `PauseStates::Drawer`,
terminal/computer mode), hide the normal flight status bar entirely, and instead show a
live "FPS: X" segment appended to the NOVA OS terminal TOPBAR line, so it reads like:

  SHIP: CERES QUEEN    LINK: LOCAL    FPS: 60

Only the FPS item moves onto the screen; the other flight status-bar items are simply
hidden while the computer is open (not relocated). When the drawer closes, the status
bar returns to normal.

## Context

- The flight status bar is the bcs `StatusBarRootMarker`, spawned in
  `crates/nova_core/src/lib.rs::setup_status_ui` with `HudTier::Status + HudDrawerExempt`.
  `HudDrawerExempt` is currently what KEEPS it visible while the drawer is open; dropping
  that tag makes `apply_hud_visibility` hide it in `PauseStates::Drawer` and restore it on
  close (the pause-change restore branch already fires on close).
- The FPS source is Bevy `FrameTimeDiagnosticsPlugin::FPS` via `DiagnosticsStore` (already
  added in `nova_gameplay::plugin`). bcs `status_fps_value_fn` reads the smoothed value and
  rounds to u32; reuse that same reading (smoothed FPS rounded to a whole number).
- The topbar is `NovaOsTopbarMarker` -> `NovaOsStatusMarker` text built by
  `spawn_nova_os_terminal_content` in `crates/nova_gameplay/src/hud/drawer.rs`; the status
  text comes from `nova_os_status_text(ship_name)`. Append the live FPS to this line and
  update it each frame while the drawer is open (real-time system alongside
  `blink_nova_os_caret`, gated on `in_state(PauseStates::Drawer)`).

## Steps

1. In `nova_core/src/lib.rs::setup_status_ui`, remove `HudDrawerExempt` from the status bar
   root so the flight status bar hides in `PauseStates::Drawer` and returns on close.
2. In `drawer.rs`, add a pure helper `nova_os_topbar_line(ship_name, fps)` that produces
   "SHIP: <name>     LINK: LOCAL     FPS: <n>" (or a "--" placeholder when fps is None).
3. Add a `drive_nova_os_topbar_fps` system that reads the smoothed FPS from `DiagnosticsStore`
   and writes the `NovaOsStatusMarker` text each frame; gate it on
   `in_state(PauseStates::Drawer)` in the caret/CRT real-time group.
4. Point the initial `NovaOsStatusMarker` text at the new helper (fps None at spawn).

## Definition of Done

- `nix develop --command cargo test -p nova_gameplay drawer` passes, including a new test
  asserting the topbar carries an FPS segment while the drawer is open.
- A test (in `hud/mod.rs` or `flight_status`) asserts the flight status bar is hidden in
  `PauseStates::Drawer` and visible otherwise.
- `nix develop --command cargo fmt` clean; pre-commit rustfmt hook passes.
- Manual/eyeball (optional): `screenshot_nova_os` example shows the topbar FPS and no flight
  status bar while the drawer is open.

## Flow State

- BRANCH: feature/nova-os-topbar-fps
- TEST: cargo test -p nova_gameplay drawer -> 59 passed, 0 failed
- HOLD: not landed (coordinated landing; another branch landing to master)
