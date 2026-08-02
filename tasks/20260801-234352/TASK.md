# Fix v0.9.0 web build: gate pause exit import

- PRIORITY: 100
- TAGS: v0.9.1, bug, web
- KIND: TASK
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

## Story

The v0.9.0 release web pipeline cannot compile `nova_menu`: `pause.rs`
imports the native-only `menu_ui::on_exit` on wasm32. Restore the release web
build without changing native pause-menu behavior.

## Steps

- [x] Gate the pause-menu exit handler import with the same target cfg as its
  definition and button.
- [x] Verify `nova_menu` for native and wasm32 targets.
- [x] Record the patch in the changelog.

## Definition of Done

- `nova_menu` compiles for wasm32. (cmd: nix develop --command cargo check -p nova_menu --target wasm32-unknown-unknown)
- `nova_menu` native library tests pass. (cmd: nix develop --command cargo test --lib -p nova_menu)
- Rust formatting passes. (cmd: nix develop --command cargo fmt --check)

## Notes

- Regression introduced by the v0.9.0 `nova_menu` split: the button use stayed
  target-gated, but its newly cross-module import did not.
- Fail-first proof: CI `cargo build --target=wasm32-unknown-unknown --release`
  failed with E0432 because `crate::menu_ui::on_exit` is configured out.
- Verification: wasm32 check passed; native `nova_menu` library tests passed
  (76 tests); formatting passed. Existing `nova_gameplay` ambiguous-import
  warnings remain unrelated.
