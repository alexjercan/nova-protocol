# Wait on the editor's state, not on frame counts

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog

## Goal

Delete `SETTLE` from the editor ranges. Gesture beats should wait on the
EDITOR's state - a placement is solved, a section landed - instead of counting
frames and hoping.

## Why

A frame is not a unit of work. `frames(10)` is about 20 ms on this machine and
about 600 ms on lavapipe, and neither number says anything about whether the
editor has finished reacting. Every gesture in `system_ship_editor` is built out
of that guess:

```
move cursor -> frames(SETTLE) -> press -> frames(SETTLE) -> release -> frames(SETTLE) -> check
```

It fails on CI. `system_ship_editor` dies at `editor: raise a tower, first
course: it built` under `probe run --render sw`, the beat having clicked and
built nothing.

**Raising the number is NOT the fix - that was tried.** `SETTLE` 10 -> 24 reached
the same beat and built nothing there too, just slower; the run then blew the
harness completion backstop instead. So the click is not arriving early, it is
arriving somewhere that does not place, and no frame count fixes that. An
instrumented run showed `aim_at_world` resolving the socket to a plausible
screen point, so the projection is not obviously wrong either. That is where the
investigation stopped.

## Direction

- Give the editor a public way to answer "is there a solved placement under the
  pointer, and what is it". `SectionGhost`, `Placement` and `PlacementStatus`
  are all `pub(crate)` in `nova_editor` today, and the status line only speaks
  when it REFUSES - so from outside the crate "ready" and "nothing under the
  pointer" are indistinguishable, which is exactly the distinction this needs.
- Wait on that before pressing, and on the section count after. Both then fail
  by DEADLINE, naming the beat, instead of by a snapshot assertion that raced.
- Expect the wait to expose the real bug rather than paper over it: if the
  editor never solves a placement at that socket under software rendering, the
  timeout says so, which is more than the current failure manages.
- Then delete `SETTLE` and the `frames(..)` gesture waits with it. The same
  shape lives in `examples/screenshots/shared/ui_walk.rs` (`STEP_DEADLINE_SECS`
  and its own settles) and in `screenshot_editor`, so this is worth doing once
  and applying to all of them.

## Done when

- `nova_editor` exposes enough placement state for a harness to wait on it.
- The editor ranges wait on conditions; `SETTLE` no longer exists.
- `probe run system_ship_editor --render sw` passes locally, and the
  `probe / systems` shard is green on CI.

## Notes

`--render sw` reproduces the CI failure locally in about 70 seconds, which is
the fast loop for this. It is the same lavapipe path CI runs on, and it is how
the torpedo fuze bug next door was found and confirmed.
