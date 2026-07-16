# Review: Debug F12 screenshot to Downloads

- TASK: 20260716-114125
- BRANCH: debug-screenshot

## Round 1

- VERDICT: APPROVE

Independently verified the load-bearing claims:

- **F12 is unbound.** Swept `crates` + `src` for `KeyCode::F12` / `F12`: the
  only hits are the new `SCREENSHOT_KEYCODE` and a test string. F1 (editor) and
  F11 (`DEBUG_TOGGLE_KEYCODE`) are the only other F-keys. No collision.
- **Delivers the Goal.** `capture_screenshot_on_key` spawns
  `Screenshot::primary_window()` + `save_to_disk(<downloads>/<millis>.png)` on
  `just_pressed(F12)`, in plain `Update` (not the `DebugSystems` gated set), so
  it fires regardless of the F11 overlay toggle - matching the design intent.
- **Feature isolation holds.** `cargo check` (default) and
  `cargo check --features debug` both compile; `nova_debug` is an `optional`
  dep added only under `#[cfg(feature = "debug")]`, so the non-debug build does
  not compile any of this.
- **Tests are meaningful.** `f12_spawns_a_screenshot_capture` would fail with
  the handler deleted (asserts 1 capture entity), and the no-press arm asserts 0
  in the same rig - a real stimulus-fired guard, not a copy that cannot fail.
  Full `nova_debug` suite (6 tests + 2 doctests) is green.

Findings:

- [ ] R1.1 (NIT) crates/nova_debug/Cargo.toml:12 - `dirs = "6"` is added
  unconditionally, whereas `nova_assets` gates it behind
  `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]`. That gate exists
  because `nova_assets` is compiled for wasm; `nova_debug` is not (the web build
  does not enable `debug`), so `dirs`/`std::fs` here are never compiled for the
  wasm target and an unconditional dep is fine and simpler. Left as-is
  deliberately; revisit only if a wasm build ever enables `debug`, at which
  point `dirs::download_dir()` would need a wasm branch anyway.

No BLOCKER/MAJOR/MINOR findings. Approving; the single NIT is left to
discretion (recommend leaving it - a `cfg` branch for a build that does not
exist would be needless complexity).
