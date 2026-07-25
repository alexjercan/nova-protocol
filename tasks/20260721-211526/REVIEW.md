# Review

## Round 1

- VERDICT: APPROVE

Reviewer: out-of-context subagent `019f9905-3f84-7b12-8b45-1828911404b9`

Findings:

- R1.1 MINOR: `crates/nova_gameplay/src/input/reference.rs` did not list the
  new comms dismiss/skip controls, while `web/src/wiki/keybinds.md` says
  bindings are viewable in-game.

Resolution:

- Added a COMMS section to the in-game keybind reference with `V` for dismiss
  oldest visible card and `B` for skip queued backlog into view. Both are
  marked `Unbound` for gamepad.

Verification:

- `nix develop --command cargo test -p nova_gameplay hud::comms_panel`
- `nix develop --command cargo test -p nova_gameplay input::reference`
- `nix develop --command cargo fmt --check`
