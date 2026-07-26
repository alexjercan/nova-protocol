# NOTES - NOVA OS topbar FPS; hide flight status bar in drawer

## What changed

Three files, additive and small:

- `crates/nova_core/src/lib.rs` (`setup_status_ui`): removed `HudDrawerExempt` from
  the flight status bar root. The bar is still `HudTier::Status` so it survives the
  `Minimal` HUD level and hides only at cinematic `None`; dropping the exempt tag lets
  `apply_hud_visibility` hide it in `PauseStates::Drawer`. Its pause-change restore
  branch un-hides it on close in the same frame. Kept `GlobalZIndex::default()` for a
  stable HUD-layer z.

- `crates/nova_gameplay/src/hud/drawer.rs`:
  - `nova_os_status_text(ship_name, fps)` now emits
    `SHIP: <name>     LINK: LOCAL     FPS: <n|-->`.
  - Added `drive_nova_os_topbar_fps` (runs on the real-time drawer group beside
    `blink_nova_os_caret`, gated `in_state(PauseStates::Drawer)`): reads the smoothed
    `FrameTimeDiagnosticsPlugin::FPS` from `DiagnosticsStore`, rounds to a whole number,
    and rewrites only the `FPS: <n>` tail of the `NovaOsStatusMarker` text via
    `topbar_line_with_fps` (preserving the ship/link head).
  - The topbar spawns with `FPS: --` until the diagnostic reads.

- `crates/nova_gameplay/src/hud/mod.rs`: added a test that a `HudTier::Status` widget
  WITHOUT `HudDrawerExempt` hides in `Drawer` and returns on close.

## Why this shape

- Reuses the EXACT FPS source the hidden status bar used (bcs `status_fps_value_fn`:
  smoothed FPS rounded to u32), so the topbar number matches what the flight bar showed.
- Hides the WHOLE flight status bar via the existing tier/visibility machinery rather
  than adding a new hide path; only the FPS item is rehomed, as the goal asked.
- Real-time system (not virtual) because virtual time is frozen while the drawer is open,
  same reasoning as the caret blink and CRT grain.

## Tests

`nix develop --command cargo test -p nova_gameplay drawer` -> 59 passed, 0 failed.
New/updated:
- `topbar_status_line_carries_a_live_fps_segment` (pure helpers).
- `drive_topbar_fps_writes_the_smoothed_reading_onto_the_status_line` (seeds a
  DiagnosticsStore, runs the system, asserts `FPS: 60` on the topbar).
- `flight_status_bar_hides_while_the_drawer_is_open_and_returns_on_close` (hud/mod.rs).
- Updated the existing PoC-structure assertion to expect the `FPS: --` suffix.

`cargo fmt` clean; full workspace `cargo check` green (my crates + nova_core).

## Merge-risk note

Another branch is converting the NOVA OS screen to render-to-texture; it keeps
`spawn_nova_os_terminal_content` and the topbar structure, so this additive edit should
still apply. Overlap is confined to the `NovaOsStatusMarker` spawn line and the caret/CRT
system-registration block in `drawer.rs`. Master also advanced to `4302e41a fix: remove
status bar`, but that commit only touched the HTML PoC, not the real status bar code.
