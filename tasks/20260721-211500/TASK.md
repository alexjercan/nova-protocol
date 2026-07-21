# Bug: mouse cursor visible during gameplay - hide it while flying

- STATUS: CLOSED
- PRIORITY: 90
- TAGS: v0.8.0, bug, hud, input

## Story

Playtest verdict (owner, 2026-07-21): the mouse cursor is visible during
gameplay. It should be hidden while flying; the drawer/menu surfaces that
need a cursor re-enable it themselves. Filed at playtest-bug priority per
the v0.8.0 policy.

## Steps

- [x] Verify-first: find the current cursor handling (grep window/cursor
      options in nova_core/nova_gameplay/nova_menu; check what Bevy 0.19
      exposes: CursorOptions visible/grab_mode) and record where the gap is.
- [x] Hide AND lock/capture the cursor while in GameStates::Playing with
      no menu/overlay up (owner decision below).
- [x] Release + show it in MainMenu, pause, the outcome overlays, editor,
      and any surface that needs pointing; make the transition state-driven
      so a future drawer (Tab) gets it for free.
- [x] Harness coverage where the project can (state-driven assertions on
      the cursor options component/resource); manual check in the real app.
- [x] Docs sweep: controls/HUD wiki mentions of the cursor, CHANGELOG Fixes.

## Definition of Done

- Cursor hidden in flight, visible in menus/pause/outcome
  (test: state assertion where riggable; manual: owner sees no cursor
  while flying).
- CHANGELOG entry under Fixes (cmd: `grep -ni "cursor" CHANGELOG.md`).

## Notes

- Owner decision (questionnaire, 2026-07-21): HIDDEN + LOCKED while
  flying (captured, so it cannot drift off-window); released and shown in
  menus, pause, outcome overlays. Make it state-driven so the future Tab
  drawer (20260721-211512, pause + free cursor) gets it for free.

## Fix (2026-07-21)

Verify-first diagnosis: the whole flight grab already existed and was
state-driven (nova_editor's `setup_grab_cursor_scenario` OnEnter Scenario,
nova_menu's `restore_cursor` on resume and `regrab_cursor_on_player_spawn` on
ship spawn), so a plain `cargo run` already hid the cursor. The gap: every one
of those grabs was wrapped in `cfg!(not(feature = "debug"))`, a compile-time
carve-out so the F11 egui inspector (which needs a pointer) stayed clickable.
So a `--features dev` build - the standard playtest build - had ALL grabs
compiled to no-ops and the cursor floated over the whole flight. This is the
`verify-stale-brief-against-current-tree` lesson: the report read as "cursor
never hidden" but the mechanism was there, just disabled for dev.

Fix (owner chose "inspector reclaims cursor when open"):
- Dropped the `cfg!(not(feature = "debug"))` guard from all three grab paths so
  flight hides+locks the cursor unconditionally, debug builds included.
- nova_debug: the bcs egui inspector's `DebugEnabled` now defaults OFF (it
  ships ON), so a dev build flies cursor-free out of the box; a new
  `sync_inspector_cursor` (Update, gated on Playing) frees the cursor while the
  inspector panel is up and grabs it back when the panel drops - yielding to
  pause / a live outcome / the pre-spawn no-player gap. nova's OWN overlay
  `DebugEnabled` (gizmos, ammo readout mirror) stays ON: gizmos need no pointer,
  and touching it would desync the ammo-readout mirror in nova_gameplay.
- F11 still toggles both `DebugEnabled`s in lockstep; with the inspector now
  defaulting off and the overlays on they sit inverse, so F11 alternates
  "gizmos" and "inspector+cursor" instead of raising both together. Acceptable
  dev-UX trade for a cursor-free default flight.

Tests (fail-first reasoning: under `--features debug` the old grabs were
no-ops, so these would have failed before the un-gating):
- nova_editor `scenario_grab_hides_and_locks_the_cursor` - the cfg-free grab
  hides+locks.
- nova_debug `inspector_off_while_flying_hides_the_cursor`,
  `inspector_on_frees_the_cursor`, `inspector_off_yields_to_pause`,
  `inspector_off_yields_when_no_player`.
All pass (nova_debug 10, nova_editor 13). Probe N/A: the headless harness has no
real window/cursor to observe, so cursor visibility is a manual owner check.
