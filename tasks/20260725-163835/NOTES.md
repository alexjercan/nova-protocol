# Notes: drawer tabs scroll instead of overflowing

- Task: 20260725-163835
- Branch: fix/drawer-scroll-tabs

## What changed

Both drawer side panels now wrap their rebuilt row lists in persistent scroll
viewports. The existing `DrawerFlightLogListMarker` and
`DrawerObjectivesListMarker` stay on the inner row containers, so appending
comms/objective events still rebuilds rows without replacing the viewport or
resetting its `ScrollPosition`.

The scroll implementation follows the existing editor/menu pattern:
`Overflow::scroll_y()` plus `ScrollPosition`, with a drawer-local mouse-wheel
system moving the scroll offset. Bevy does not scroll overflow nodes on its own.

## Input behavior

The drawer has two visible scroll panels, so the wheel system prefers the
viewport with `Hovered(true)` when Bevy picking has marked one. If no drawer
viewport is hovered, it scrolls all drawer viewports as a fallback, matching the
older single-panel editor/menu behavior and avoiding a dead wheel if hover state
is unavailable.

## Verification notes

- Fail-first: the first `nix develop --command cargo test -p nova_gameplay drawer`
  failed before implementation because the new test referenced the missing
  drawer scroll system.
- After implementation: `nix develop --command cargo test -p nova_gameplay drawer`
  passed, including the left viewport, right viewport, scroll clamp, and
  hovered-panel targeting tests.
- Manual visual acceptance remains pending for the user/reviewer: open an
  overlong Flight Log and Objectives list and confirm both stay inside their
  panels while scrolling.

## Reflection

The useful part of the implementation was checking the existing editor/menu
scroll code first; that caught the need for a wheel system before the layout
patch was written. The first test run also exposed the Bevy 0.19 `MouseWheel`
`phase` field and the correct `RunSystemOnce` import path, so the final tests
mirror the current engine API instead of an older remembered shape.
