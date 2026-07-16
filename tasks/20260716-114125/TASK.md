# Debug F12 screenshot to Downloads

- STATUS: CLOSED
- PRIORITY: 90
- TAGS: debug, feature, screenshot

## Goal

When the game is built with `--features debug`, pressing F12 captures the
primary window and saves it as a PNG in the user's Downloads directory, named
with a timestamp. "Done" = in a `--features debug` build, F12 spawns a
`Screenshot::primary_window()` capture that lands at
`<downloads>/<timestamp>.png`, and this does nothing (does not compile in) a
non-debug build.

## Design decisions

- **Home: `crates/nova_debug`.** The whole crate is gated behind the `debug`
  feature (`nova_core` only `add_plugins(DebugPlugin)` under
  `#[cfg(feature = "debug")]`, and `nova_debug` is an `optional` dep), so a new
  module here is automatically debug-only with no extra `cfg`. New file
  `screenshot.rs` + a `ScreenshotHotkeyPlugin` added in `DebugPlugin::build`.
- **Keybind F12.** F1 = editor toggle, F11 = `DEBUG_TOGGLE_KEYCODE`; F12 is
  free. Expose as `pub const SCREENSHOT_KEYCODE: KeyCode = KeyCode::F12`,
  mirroring `DEBUG_TOGGLE_KEYCODE`.
- **Not gated by `DebugEnabled`.** The F11 toggle hides/shows dev overlays; a
  screenshot must work regardless of overlay state, so the capture system is a
  plain `Update` system, NOT in the `DebugSystems` run-condition set.
- **Input via `Res<ButtonInput<KeyCode>>` + `just_pressed`,** matching
  `toggle_debug_mode`.
- **Capture via `commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path))`.**
  `EntityCommands::observe` works from a normal system, so no exclusive-`World`
  system is needed (unlike the harness `capture_window`, which is called from an
  autopilot closure). Same Bevy 0.19 primitive the harness/scenario action use.
- **Downloads path via `dirs::download_dir()`** (add `dirs = "6"` to
  `nova_debug`, already a workspace dep used by `nova_assets`), falling back to
  the current dir if the platform has no Downloads dir. `create_dir_all` the
  parent defensively, matching `capture_window`.
- **Timestamp = Unix epoch millis** (`SystemTime::now().duration_since(UNIX_EPOCH)`),
  filename `<millis>.png`. Dependency-free, unique across rapid presses, and
  sortable. A human-readable `YYYY-MM-DD_HH-MM-SS` name would need `chrono`
  (not a current dep) and local-timezone handling; out of scope for "for now".
  Factor the millis->name mapping into a pure `screenshot_filename(Duration)`
  so it is unit-testable without a clock.

## Steps

- [x] Add `dirs = "6"` to `crates/nova_debug/Cargo.toml` `[dependencies]`.
- [x] Create `crates/nova_debug/src/screenshot.rs`:
  - `pub const SCREENSHOT_KEYCODE: KeyCode = KeyCode::F12;`
  - pure `fn screenshot_filename(since_epoch: Duration) -> String` returning
    `format!("{}.png", since_epoch.as_millis())`.
  - `fn downloads_screenshot_path() -> PathBuf`: `dirs::download_dir()` (fallback
    `PathBuf::from(".")`) joined with the filename derived from
    `SystemTime::now()`.
  - system `fn capture_screenshot_on_key(commands, keyboard)`: on
    `just_pressed(SCREENSHOT_KEYCODE)`, resolve the path, `create_dir_all` the
    parent (warn on failure), `commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path))`,
    and `info!` the destination.
  - `pub struct ScreenshotHotkeyPlugin;` adding the system to `Update`.
  - module + item docs in the crate's voice.
- [x] Wire into `crates/nova_debug/src/lib.rs`: `pub mod screenshot;`,
  `app.add_plugins(screenshot::ScreenshotHotkeyPlugin);` in `DebugPlugin::build`,
  and re-export `ScreenshotHotkeyPlugin` + `SCREENSHOT_KEYCODE` from the
  `prelude`.
- [x] Tests in `screenshot.rs`:
  - `screenshot_filename` maps a known `Duration` to `"<millis>.png"`.
  - behavior test: build an `App`, `init_resource::<ButtonInput<KeyCode>>()`,
    add the system; press F12 + `update()` and assert exactly one entity carries
    a `Screenshot` component; assert that WITHOUT the press the same rig spawns
    none (stimulus-fired guard in the same test).
- [x] Docs: add `docs/2026-07-16-debug-f12-screenshot.md` (decision, keybind,
  path, timestamp format, alternatives). Grep for where the F11/`DEBUG_TOGGLE`
  keybind is documented (README / dev wiki / CHANGELOG unreleased) and add the
  F12 binding alongside it. Add a CHANGELOG entry if there is an unreleased
  section.
- [x] Verify: `cargo fmt`, `cargo check --features debug` (and the default,
  non-debug build still compiles), and run the two new tests. Skip the full
  local clippy/test suite per repo convention (CI runs it).

## Notes

- Behavior test caveat: `save_to_disk` only fires on `ScreenshotCaptured`, which
  never happens under a headless `MinimalPlugins` app, so the observer is inert
  and the test asserts the capture-entity spawn, not an on-disk file. That is the
  observable behavior of the input handler; on-disk verification needs a GPU and
  belongs to manual/e2e checking.
