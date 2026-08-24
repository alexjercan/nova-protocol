# Gamepad navigation for menus/editor and mobile virtual pad

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog,input,gamepad,mobile,spike

Rewritten 2026-08-24. The settings-menu half (tabs + rebinding, the absorbed
`20260818-182012` Part C) moved into v0.12.0 as `20260824-120527`; this task
keeps the two parts that should target a SETTLED editor and menu surface,
which v0.12.0's node-editor rework is actively changing. Stays backlog;
schedule after the editor interaction model lands. Current-state audit:
`tasks/20260815-231945/INPUT-AND-PROCESS.md` (it corrects this task's stale
paths: the flight rig is nova_ship/src/input/player/flight_rig.rs, the HUD
crate is nova_hud).

## Part A - Gamepad navigation for menus and the editor

Make the whole out-of-cockpit UI operable with a gamepad (no mouse):

- Menus (main menu + ESC pause menu, `nova_menu`): directional focus
  movement (D-Pad / left stick), confirm (South), back/cancel (East), with a
  visible focus ring.
- Editor (`nova_editor`): navigate the gallery and rail, place and rebind
  sections, trigger play-test from the pad.
- Prefer Bevy's UI focus/navigation primitives if they fit; otherwise a
  small focus-ring + gamepad-driven focus system.
- The existing raw pad reads are inventoried in INPUT-AND-PROCESS.md
  section 5 (pause Start, HUD Select, editor L3, placement capture, NOVA OS
  RightThumb/Start). By then they should be registry actions
  (`20260820-174148`) or documented fixed chords.

## Part B - Mobile virtual pad (touch)

Make the web build playable on a phone, built on the bevy-common-systems
touch primitives as the reference implementation:

- `bevy_common_systems::ui::touchpad` - `TouchpadPlugin` + `TouchSeen`
  (reveal-on-first-touch via `RevealOnTouch`/`HideOnTouch`; no
  wasm/maxTouchPoints sniffing), pure hit-tests `stick_deflection` and
  `button_grid_at`.
- `bevy_common_systems::input::pointer::UnifiedPointer` for aim/look.
- Reference the crate's shipped touch work:
  ~/personal/bevy-common-systems/docs/2026-07-04-{dropzone,reactor,overload}-touch-controls.md
  and examples/08_dropzone.rs.
- On-screen left stick (thrust/nav), right-side aim area, buttons for the
  core verbs (GOTO / ORBIT / STOP, radar lock, fire), revealed on first
  touch.
- `bevy-common-systems` is a git dependency with a local checkout at
  ~/personal/bevy-common-systems; extend the primitives there and bump the
  pinned rev if needed.

## Notes

- Requires a spike first (menu-nav approach: Bevy UI focus vs a custom focus
  ring; virtual-pad layout), then split Part A and Part B into independent
  child tasks. Gamepad navigation first; mobile last, so its layout targets
  stable interactions.

Done when: the menus and the editor are fully operable with a gamepad, and
the web build is playable on a touchscreen via a virtual pad built on the
bevy-common-systems primitives.
