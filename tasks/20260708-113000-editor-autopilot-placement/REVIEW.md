# Review: Drive section placement in the editor autopilot

- TASK: 20260708-113000
- BRANCH: feat/editor-autopilot-placement

## Round 1

- VERDICT: APPROVE

Diff extends only `examples/09_editor.rs`: the one-shot autopilot becomes a frame-paced state
machine that creates a ship, selects a hull, and places it via a simulated mouse click. No
production or editor code changed.

Verified:

- It actually places a section, through the real pipeline. The headless run logs
  `placed a section (1 -> 2 sections)` - the synthetic `PointerInput` drives avian's physics
  picking, which raycasts the section collider and fires the editor's own
  `on_click_spaceship_section`. This is genuine end-to-end coverage (camera projection + picking +
  placement observer), not a faked shortcut.
- Deterministic. Ran 3x, all placed 1->2 with a clean exit. The editor camera is static during
  the autopilot (WASD controller, no keys pressed), so `world_to_viewport` of a fixed section
  position is stable, and the 30-frame wait after ship creation lets avian prepare the section
  colliders before the click.
- Driven entirely through public seams - the reason no editor code needed changing. Selection via
  `Add<Pressed>` on the section button (matching the editor's `button_on_setting` contract),
  aiming via `Camera`/`RenderTarget`/`GlobalTransform` reads, clicking via `PointerInput`
  messages. The `RenderTarget` is correctly read as a separate component (bevy 0.19 moved it off
  `Camera`).
- The borrow handling in `aim_at_a_section` is sound: section pos/count are collected first, then
  `Camera` + `GlobalTransform` + `RenderTarget` coexist as shared borrows to compute the viewport
  point and normalized target.
- State machine is guarded: `Aim` retries until a section/camera/window exist; `Verify` warns
  (does not panic) if the click added nothing, so a hypothetical picking miss degrades to a
  logged warning rather than a failed/hung run.
- Green: `cargo clippy --workspace --all-targets` clean (only the pre-existing `hull_section.rs`
  warning), `cargo test --workspace` with `examples_smoke` running `09_editor` (49s), 58
  nova_gameplay tests, etc.

Honest scope (TASK.md): `examples_smoke` asserts clean-exit, not the "placed a section" line, so a
picking miss would not fail CI - acceptable given the determinism, and noted. Only the hull kind
is placed; other kinds are a possible extension.

No BLOCKER/MAJOR/MINOR findings.
