# Bug: drawer scroll clamps at content end

- STATUS: CLOSED
- PRIORITY: 57
- TAGS: v0.9.0,bug,ui,hud
- KIND: TASK
- FLOW STEP: DONE
- PLAN STATUS: APPROVED

## Story

As a player scrolling a long drawer list, I need the stored scroll offset to
stop at the actual bottom of the content, so scrolling back up responds
immediately instead of first unwinding invisible overscroll.

## Steps

- [x] Add a regression test that starts a drawer viewport near its maximum
  scroll offset, scrolls down past the bottom, and proves `ScrollPosition.y`
  clamps to the computed max.
- [x] Clamp drawer wheel updates against Bevy UI's same scroll bound:
  `content_size - size + scrollbar_size`, never below zero.
- [x] Verify the existing top clamp, hover targeting, and drawer row rebuild
  tests still pass.

## Definition of Done

- test: drawer wheel down at the bottom clamps `ScrollPosition.y` to the
  viewport's computed maximum.
- cmd: `nix develop --command cargo test -p nova_gameplay drawer`
- cmd: `nix develop --command cargo fmt --check`
- cmd: `nix develop --command cargo check`

## Notes

- Diagnosis: Bevy's layout system clamps the rendered scroll position, but the
  drawer input system only clamped the stored `ScrollPosition` at the top. That
  allowed invisible overscroll to accumulate past the bottom.

## Outcome

Fixed the drawer wheel system to clamp the stored `ScrollPosition.y` against
the same maximum Bevy UI uses during layout:
`content_size.y - size.y + scrollbar_size.y`, clamped to zero. This prevents
invisible bottom overscroll from accumulating after the visible list is already
at the end.

Fail-first evidence: `nix develop --command cargo test -p nova_gameplay drawer`
failed with `drawer_wheel_scroll_clamps_at_content_bottom`, reporting stored
scroll `115.0` when the computed max was `100.0`. After the fix, the focused
drawer suite passed with 26 tests.

Verification:

- `nix develop --command cargo fmt --check`
- `nix develop --command cargo test -p nova_gameplay drawer`
- `nix develop --command cargo check`
- `tatr check --ledger LESSONS.md`

Self-reflection: the first scroll fix copied the repo's top-clamp pattern but
did not inspect Bevy's layout clamp closely enough. For future UI scroll input,
read the engine's own clamp formula before writing the wheel handler.
