# Drawer right panel: objectives as a styled list (not plain text)

- PRIORITY: 54
- TAGS: v0.9.0, feature, ui, hud
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

## Goal

Playtest rework (owner, 2026-07-24; amended by owner, 2026-07-25): the
drawer's RIGHT panel shows objectives, but they are "just text right now - we
need to make it prettier". Rework the objectives section from the plain text
lines the shell spawned into a proper styled LIST, and keep completed objectives
in that list as a mission-log style history with strike-through treatment.

Scope (direction-level; /plan breaks into steps at pickup):

- Rework the drawer's objectives section (`hud/drawer.rs`, the
  DrawerObjectivesListMarker container spawned by the shell 20260724-102304) from
  plain `Text` lines into a styled list: per-item rows with a bullet/status glyph,
  consistent spacing, done/active states, panel styling that matches the drawer
  chrome (nova_ui::theme). Right panel, slides in from the right.
- Keep active objectives data-driven from bevy_common_systems GameObjectives
  (already synced), and derive completed objective history from that resource's
  removals so the drawer reads like a compact objective log.

## Story

As a player reading the paused ship-computer drawer, I want the objectives in
the right panel to read as a deliberate cockpit list and log, so I can see both
what is active now and what I already completed instead of losing completed
steps as soon as the scenario removes them.

## Steps

- [x] Replace `rebuild_drawer_objectives` in `crates/nova_gameplay/src/hud/drawer.rs`
  with a small local row builder that spawns one objective row per
  drawer objective-log entry under `DrawerObjectivesListMarker`.
- [x] Add drawer-local objective-log state in `hud/drawer.rs` that mirrors active
  `GameObjectives` entries and marks removed objectives as completed while
  preserving their messages and order for the current scenario.
- [x] Clear the drawer objective log on scenario teardown, using the same empty
  `GameObjectives` transition discipline as `objective_feedback` so failed or
  abandoned objectives do not appear as completed.
- [x] Style each non-empty objective row as a stable Bevy UI node: raised panel
  fill, hard 1px border, compact padding, gap, objective-colored bullet/glyph,
  and message text using `nova_ui::theme`.
- [x] Style completed rows as muted log entries with a done glyph and a
  line-through visual. Because Bevy UI text has no native text-decoration
  component in this tree, implement the line-through as a thin themed overlay
  node in the row's text area.
- [x] Keep the empty state styled as drawer chrome too, not a bare text line.
- [x] Preserve data flow and rebuild semantics: list contents still come only
  from `GameObjectives` plus the drawer-local derived log, rebuild on resource
  change or first list spawn, and old rows are despawned before replacement.
- [x] Update or add headless tests in the drawer module that assert the list is
  row-structured, has a marker/id/status per objective row, carries the objective
  text, retains completed objectives with line-through styling, clears on
  teardown, and renders a styled empty state.
- [x] Add `tasks/20260724-134350/NOTES.md` with the change record, tradeoffs,
  difficulties, and self-reflection required by the repo instructions.

## Definition of Done

- The drawer objectives section no longer spawns direct bare objective `Text`
  children; each active objective is represented by a styled row node with
  bullet/glyph and message descendants. (test:
  `drawer_objectives_section_uses_styled_rows`)
- A completed objective remains in the drawer list as a completed log row after
  it is removed from `GameObjectives`; the row is muted, carries a done glyph,
  and has a line-through overlay. (test:
  `drawer_objectives_keep_completed_rows_with_strike`)
- Scenario teardown clears the derived objective log instead of marking every
  previously active objective completed. (test:
  `drawer_objective_log_clears_on_teardown`)
- Empty objectives render as a styled empty-state row using drawer/theme chrome,
  not a lone muted line. (test: `drawer_objectives_empty_state_is_styled`)
- Rebuild behavior remains data-driven from `GameObjectives` and replaces stale
  rows when objectives change. (test:
  `drawer_objectives_rebuild_replaces_stale_rows`)
- The changed gameplay crate compiles and the touched drawer tests pass. (cmd:
  `nix develop --command cargo test -p nova_gameplay drawer`)
- The workspace is formatted after the change. (cmd:
  `nix develop --command cargo fmt --check`)
- manual: open the drawer in a real or screenshot-capable run and confirm the
  right panel objectives read as a styled list that matches the drawer chrome.

## Notes

- From the 2026-07-24 playtest. Extends the shell's objectives section
  (20260724-102304, LANDED) which rendered plain text as a placeholder. Pairs with
  the drawer-open rework (20260724-134335) and the flight objective surface
  (20260724-134312).
- Files: hud/drawer.rs (rebuild_drawer_objectives + the section container).
- Current code: `DrawerObjectivesListMarker` is already the right panel list
  container; `rebuild_drawer_objectives` currently despawns children and spawns
  direct `Text` lines.
- Artifact choice: keep this as local Nova drawer UI, not the generic
  `bevy_common_systems` objectives panel, because the generic panel renders text
  lines while this task needs drawer-specific row chrome and glyphs. See
  `DECISION.md`.
- User amendment 2026-07-25: keep completed objectives in the drawer like a log
  and show them with line-through treatment.
- Current data fact: `GameObjectives` only stores active objectives. Completion
  is observable as removal from the active list; `objective_feedback` already
  diffs the same resource for transient completion ghosts and treats an empty
  list as teardown.
- Bevy UI fact checked locally: no native Bevy text line-through decoration was
  found in the installed source, so the visual strike should be a small overlay
  node rather than a `Text` property.
- Implementation: added a private `DrawerObjectiveLog` resource in
  `hud/drawer.rs`, synced it from `GameObjectives` diffs, and rebuilt the drawer
  list from row nodes instead of direct text children.
- Docs: updated `web/src/wiki/hud.md` so the ship-computer drawer describes the
  active-plus-completed objective log.
- Verification: `nix develop --command cargo test -p nova_gameplay drawer`,
  `nix develop --command cargo fmt --check`, `npm run ci` in `web/`, and
  `nix develop --command cargo check` passed. `npm run ci` first failed because
  `web/node_modules` was absent; `npm ci` installed the locked dependencies and
  reported existing audit warnings, then the site CI passed.
