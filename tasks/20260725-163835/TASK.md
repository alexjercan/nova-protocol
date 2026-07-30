# Bug: drawer tabs scroll instead of overflowing

- STATUS: CLOSED
- PRIORITY: 57
- TAGS: v0.9.0,bug,ui,hud
- KIND: TASK
- FLOW STEP: DONE
- PLAN STATUS: APPROVED

## Story

As a player opening the Tab drawer, I need long drawer content to stay inside
the drawer panels and scroll, so the combined Flight Log does not spill out of
the left panel and the right Objectives list has the same bounded behavior when
it grows.

## Decision

Treat both side drawer panels as scrollable tabs: the left `COMMS / LOG` panel
and the right `SHIP COMPUTER` panel keep their list rows in bounded Bevy UI
scroll viewports. The left panel remains the chronological "past things plus
journal" stream, while the right panel remains current active objectives only.

Use the codebase's existing Bevy UI scroll pattern from the editor and menu:
`Overflow::scroll_y()` plus `ScrollPosition`, driven by a drawer-scoped mouse
wheel system. Bevy does not auto-scroll overflow nodes, so a component-only
layout fix would still leave the player unable to move the list.

The rebuild systems should continue to own only the inner row lists. Scroll
viewport nodes must persist across list rebuilds so appending comms/objective
events does not constantly replace the scroll component.

## Steps

- [x] Reproduce the failure with drawer widget-tree tests that fail on the current
   plain column lists:
   - left Flight Log list has a bounded scroll viewport using
     `Overflow::scroll_y()` and `ScrollPosition`;
   - right Objectives list has the same bounded scroll viewport;
   - wheel input changes the drawer scroll offset and clamps at the top.
- [x] Add drawer scroll viewport marker components around the existing
   `DrawerFlightLogListMarker` and `DrawerObjectivesListMarker` inner lists.
   Give the section/viewports stable flex sizing so the viewport consumes the
   remaining panel height instead of expanding past it.
- [x] Add a drawer-scoped wheel scroll system using the existing editor/menu
   scroll semantics. Register it with the drawer HUD systems only while the
   drawer is available/open enough to receive input.
- [x] Keep `rebuild_drawer_flight_log` and `rebuild_drawer_objectives` targeting
   the inner list markers, preserving the viewport and its `ScrollPosition`
   across row rebuilds.
- [x] Run the focused drawer tests, format/check the Rust workspace, and web CI
   for the HUD wiki update.
- [x] Record the manual acceptance check for an overlong log/objective list as
   a `manual:` Definition of Done item for review/user acceptance.
- [x] Record implementation notes in `tasks/20260725-163835/NOTES.md`, including
   the scroll API choice, any input conflict considered, and how the bug was
   verified.

## Definition of Done

- test: a new drawer test proves the left Flight Log is hosted in a scrollable,
  bounded viewport.
- test: a new drawer test proves the right Objectives list is hosted in a
  scrollable, bounded viewport.
- test: a drawer wheel-scroll test proves `ScrollPosition` changes and clamps
  at the top.
- test: existing drawer log/objective rebuild tests still pass, proving rows
  still render in order and completed objectives remain only in the left Flight
  Log.
- cmd: `nix develop --command cargo test -p nova_gameplay drawer`
- cmd: `nix develop --command cargo fmt --check`
- cmd: `nix develop --command cargo check`
- manual: open a scenario with an overlong Flight Log and verify the left panel
  contents stay inside the panel and scroll instead of covering the lower-left
  key hints; verify a long Objectives list scrolls inside the right panel.

## Notes

- Lessons consulted: `log-ui-shape-before-plan`,
  `widget-tree-eyeball-for-logical-layout`, `render-output-eyeball`, and
  `advertised-but-unwired`.
- The user asked for "Tabs" plural. This task intentionally fixes both drawer
  sides because the current right Objectives panel has the same overflow shape
  as the reported left Logs panel.
- Fail-first evidence: `nix develop --command cargo test -p nova_gameplay drawer`
  failed before implementation because `scroll_drawer_panels` was missing; after
  implementation the same command passed with 25 drawer/HUD tests.

## Outcome

The drawer now uses persistent scroll viewports around both side-panel row
lists. The fix keeps the existing inner list markers as the rebuild targets,
which preserves row rebuild behavior while bounding long Flight Log and
Objectives content inside the side panels.

The chosen mechanism is the existing editor/menu Bevy UI pattern:
`Overflow::scroll_y()` plus `ScrollPosition`, driven by a drawer-scoped wheel
system. A layout-only alternative was rejected because Bevy does not move
overflow nodes without a system writing `ScrollPosition`. The two-panel input
case is handled by preferring the hovered drawer viewport when one exists, with
an all-viewports fallback if no hover state is available.

Player-facing docs were updated in `web/src/wiki/hud.md`, with a short
Unreleased changelog entry.

Verification:

- `nix develop --command cargo fmt --check`
- `nix develop --command cargo test -p nova_gameplay drawer`
- `nix develop --command cargo check`
- `npm run ci` from `web/` after `npm ci` installed this worktree's web deps
- `tatr check --ledger LESSONS.md`

The first test run caught two Bevy 0.19 API details in the new harness:
`MouseWheel` requires `phase`, and the `RunSystemOnce` trait is exported through
`bevy::ecs::system`. Fixing the harness kept the regression tests aligned with
the engine version in use.

Manual acceptance remains pending: open a scenario with an overlong Flight Log
and Objectives list and confirm both stay inside their panels while scrolling.

Self-reflection: checking the editor/menu scroll implementations before coding
kept the fix aligned with local precedent. The task plan should have separated
automated verification from human visual acceptance up front, instead of
requiring a small checklist correction during close-out.
