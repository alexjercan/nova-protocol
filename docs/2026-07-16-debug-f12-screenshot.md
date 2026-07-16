# F12 debug screenshot to Downloads

## What changed

Added an F12 screenshot hotkey to the `debug` feature. In a build compiled with
`--features debug` (or `dev`), pressing F12 captures the primary window and
writes it to the user's Downloads directory as `<unix-millis>.png`.

- New module `crates/nova_debug/src/screenshot.rs`: `SCREENSHOT_KEYCODE`
  (`KeyCode::F12`), a `ScreenshotHotkeyPlugin`, and the capture system.
- `DebugPlugin::build` now `add_plugins(screenshot::ScreenshotHotkeyPlugin)`,
  and the prelude re-exports `ScreenshotHotkeyPlugin` + `SCREENSHOT_KEYCODE`.
- `crates/nova_debug/Cargo.toml` gained `dirs = "6"` (already a workspace dep,
  used by `nova_assets`) to resolve the Downloads directory.
- Docs: `web/src/wiki/dev/development.md` (dev keybind reference) and the
  CHANGELOG `[Unreleased]` section.

## Why these decisions

- **Home in `nova_debug`.** The crate is an `optional` dep only pulled in under
  the `debug` feature, and `nova_core` only `add_plugins(DebugPlugin)` behind
  `#[cfg(feature = "debug")]`. So a new module here is debug-only for free - no
  extra `cfg` attributes, and a non-debug build does not compile any of it.
- **F12.** F1 is the editor toggle and F11 is `DEBUG_TOGGLE_KEYCODE`; F12 was
  free. Exposed as a `pub const` mirroring `DEBUG_TOGGLE_KEYCODE`.
- **Not gated by `DebugEnabled`.** The F11 toggle hides/shows the dev overlays.
  A screenshot should work regardless of overlay state, so the capture system
  runs in plain `Update`, outside the `DebugSystems` run-condition set. If it
  were in that set, F12 would silently do nothing whenever overlays were toggled
  off - the opposite of what you want when grabbing a clean shot.
- **`Commands`, not exclusive `World`.** `EntityCommands::observe` lets a normal
  system spawn the capture and attach `save_to_disk` in one go, so unlike the
  harness `capture_window` (called from an autopilot closure with `&mut World`)
  no exclusive system is needed. Same Bevy 0.19 primitive
  (`Screenshot::primary_window()` + `save_to_disk`) the reel harness and the
  `Screenshot` scenario action already use.
- **Downloads via `dirs::download_dir()`,** falling back to `.` when the
  platform has no Downloads dir, and `create_dir_all` on the parent defensively
  (mirrors `capture_window`).
- **Timestamp = Unix epoch millis.** Dependency-free, unique across rapid
  presses, and sorts chronologically. A readable `YYYY-MM-DD_HH-MM-SS` name
  would pull in `chrono` (not a current dep) plus local-timezone handling; out
  of scope for a "for now" debug convenience. The millis->name mapping is a pure
  `screenshot_filename(Duration)` so it unit-tests without a clock.

## Alternatives considered

- **A player-facing keybind / entry in `web/src/wiki/keybinds.md`.** Rejected:
  this is a dev/debug-only tool (compiled out of release builds), so it belongs
  in the dev wiki, not the player control reference.
- **Writing next to the binary / under `NOVA_SHOT_DIR`** (as the harness reel
  does). Rejected: the ask is specifically the user's Downloads folder, which is
  where a person expects an ad-hoc screenshot to land.
- **A human-readable timestamp.** Deferred to keep the change dependency-free;
  the helper is a one-line swap if we later add `chrono`.

## Testing

- `screenshot_filename` maps a known `Duration` to `<millis>.png` (including
  sub-millisecond truncation).
- A behavior test builds an `App`, presses F12, and asserts exactly one
  `Screenshot` capture entity is spawned - and that the same rig with no press
  spawns none, so the press (not the rig) is what fires.

Caveat: `save_to_disk` only runs on the render side (`ScreenshotCaptured`),
which never fires under a headless `MinimalPlugins` app, so the test verifies the
input handler's observable behavior (the capture-entity spawn), not an on-disk
PNG. On-disk verification needs a GPU and is a manual/e2e check: run
`cargo run --features debug`, press F12, and confirm the file appears in
Downloads.

## Reflection

Straightforward: the codebase already had the exact screenshot primitive and an
input-handler pattern (`toggle_debug_mode`) to copy, so the work was mostly
picking the right seam (plain `Update`, not `DebugSystems`) and the path/naming
policy. The one non-obvious call was keeping the capture out of the
`DebugEnabled` gate - easy to get wrong by reflexively dropping it in the debug
set. Next time, for a feature this small in a well-mapped crate, a single
targeted read of the host plugin plus a grep for the primitive would have been
enough without a full-repo exploration pass.
