# Wait on the editor's state, not on frame counts

- STATUS: IN_PROGRESS
- PRIORITY: 95
- TAGS: v0.12.0, editor, probe, bug

Phase 0 of the v0.12.0 editor release (`20260812-131912`). It is
simultaneously the CI fix, and the first proof of the node model's premise:
editor state is inspectable data. Do this before any new editor work.
Research: `tasks/20260815-231945/EDITOR-STATE.md` section 2,
`SCENARIO-PIPELINE.md` section 4.

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

## The audit (round 4) - exactly what is hidden and where the sleeps are

Everything the harness needs is `pub(crate)`; the crate exports only
`NovaEditorPlugin` and `EditorSandboxSystems`:

- `SectionChoice` (nova_editor/src/config.rs:38), `PlacementPreview` (:77),
  `Placement` (:84), `SectionGhost` (:96), `PlacementStatus` (:105).
- `snap::Placement` (snap.rs:69-78) and `Refusal` (snap.rs:36).
- `GalleryState` (gallery/mod.rs:37).

The status line only speaks on refusal or a solved mate; with nothing under
the pointer it is hidden (placement.rs:600-602) - so from outside the crate
"ready" and "nothing under the pointer" are indistinguishable, which is
exactly the distinction this needs.

The sleeps to delete, all of the same shape:

- `SETTLE = 10` and `SHIP_SETTLE = 40`
  (examples/systems/system_ship_editor.rs:170, :175; the doc comment at
  :162-168 already names this task).
- `STEP_DEADLINE_SECS = 30` and `GESTURE_FRAMES = 12`
  (examples/screenshots/shared/ui_walk.rs:32, :58), used throughout
  screenshot_editor.
- `SETTLE_FRAMES = 30` (crates/nova_debug/src/harness.rs:127).
- Same pattern in bug_sandbox_soak.rs:96-100 and system_menu_boot.rs:81.

Proxies the ranges use today because state is private: section-count deltas,
status-line TEXT scraping via `subtree_text(world, "Placement Status")`,
"arming proven only by the next click", gallery-open proven by a named UI
node's rect. All of these become direct reads.

## Direction

- Add one public read-only probe resource instead of exposing solver
  internals: e.g. `EditorPlacementProbe { None | Solved { prototype, target }
  | Refused(reason) }`, written at the end of `update_placement_preview`
  (placement.rs:484-548). Export via the prelude.
- Expose "a part is armed" (`SectionChoice` or a bool) and "gallery open"
  (`GalleryState.open` or a run condition) the same way.
- Add the missing `or()` combinator to nova_autopilot predicates
  (crates/nova_autopilot/src/predicate.rs has `and`/`not` only).
- Wait on "placement solved and not refused" before pressing, and on the
  section count after. Both then fail by DEADLINE, naming the beat, instead of
  by a snapshot assertion that raced.
- Expect the wait to expose the real bug rather than paper over it: if the
  editor never solves a placement at that socket under software rendering, the
  timeout says so, which is more than the current failure manages.
- Then delete `SETTLE` and the `frames(..)` gesture waits with it, in all the
  sites listed above.
- Keep the predicate vocabulary consciously parallel with the scenario
  `Sequence` gates (`20260820-223059`): until/deadline are the same idea for
  two consumers.

## Done when

- `nova_editor` exposes placement/armed/gallery state a harness can wait on.
- The editor ranges wait on conditions; `SETTLE` no longer exists.
- `probe run system_ship_editor --render sw` passes locally, and the
  `probe / systems` shard is green on CI.

## Notes

`--render sw` reproduces the CI failure locally in about 70 seconds, which is
the fast loop for this. It is the same lavapipe path CI runs on, and it is how
the torpedo fuze bug next door was found and confirmed.

`NOTES.md` carries what was built and the evidence. Read its first section
first: the "Why" above is STALE. The `raise a tower` failure was the
archetype-ordered face pick, already fixed at the source in `placed_sections`,
and the unchanged range passed `--render sw` here before a line was touched.
What was left - and what was done - is the structural half this task also asks
for.
