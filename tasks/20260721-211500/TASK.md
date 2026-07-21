# Bug: mouse cursor visible during gameplay - hide it while flying

- STATUS: OPEN
- PRIORITY: 90
- TAGS: v0.8.0,bug,hud,input

## Story

Playtest verdict (owner, 2026-07-21): the mouse cursor is visible during
gameplay. It should be hidden while flying; the drawer/menu surfaces that
need a cursor re-enable it themselves. Filed at playtest-bug priority per
the v0.8.0 policy.

## Steps

- [ ] Verify-first: find the current cursor handling (grep window/cursor
      options in nova_core/nova_gameplay/nova_menu; check what Bevy 0.19
      exposes: CursorOptions visible/grab_mode) and record where the gap is.
- [ ] Hide AND lock/capture the cursor while in GameStates::Playing with
      no menu/overlay up (owner decision below).
- [ ] Release + show it in MainMenu, pause, the outcome overlays, editor,
      and any surface that needs pointing; make the transition state-driven
      so a future drawer (Tab) gets it for free.
- [ ] Harness coverage where the project can (state-driven assertions on
      the cursor options component/resource); manual check in the real app.
- [ ] Docs sweep: controls/HUD wiki mentions of the cursor, CHANGELOG Fixes.

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
