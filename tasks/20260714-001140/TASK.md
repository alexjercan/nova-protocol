# v0.6.0: gamepad navigation for menus/editor + mobile virtual pad

- STATUS: OPEN
- PRIORITY: 40
- TAGS: v0.6.0,input,gamepad,mobile

## Goal

Broaden input beyond keyboard + mouse in two workstreams for the v0.6.0 cycle.
Gameplay already has gamepad bindings (flight verbs in player.rs; pause / HUD
cycle / back-to-editor added in the v0.5.x cycle), but the menus and the editor
cannot be driven by a gamepad at all, and there is no touch support for mobile.

### Part A - Gamepad navigation for menus and the editor

Make the whole out-of-cockpit UI operable with a gamepad (no mouse required):

- **Menus** (main menu + ESC pause menu, `nova_menu`): directional focus
  movement (D-Pad / left stick), confirm (South), back/cancel (East), with a
  visible focus ring, so New Game / Sandbox / Settings / Resume / Exit are all
  reachable on a pad.
- **Editor** (`nova_editor`): navigate the section palette / build panel, place
  and rebind sections, and trigger the play-test transition from the pad.
- Prefer Bevy's UI focus/navigation primitives if they fit; otherwise a small
  focus-ring + gamepad-driven focus system. Wire it through the existing
  `ButtonInput<GamepadButton>` reads / input rig.

### Part B - Mobile virtual pad (touch)

Make the web build playable on a phone with on-screen touch controls, built on
the bevy-common-systems touch primitives as the reference implementation:

- `bevy_common_systems::ui::touchpad` - `TouchpadPlugin` + `TouchSeen`
  (reveal-on-first-touch gating via `RevealOnTouch` / `HideOnTouch`, so a
  desktop session never sees the pad, with no wasm/`maxTouchPoints` sniffing),
  plus the pure, unit-testable hit-tests `stick_deflection` (finger offset from
  a floating origin -> stick vector) and `button_grid_at` (touch pos -> zone).
- `bevy_common_systems::input::pointer::UnifiedPointer` - collapses
  mouse/touch/cursor into one per-frame pointer (touch wins over cursor) for
  aim/look on a touch device.
- Reference the crate's shipped touch-control work:
  `~/personal/bevy-common-systems/docs/2026-07-04-{dropzone,reactor,overload}-touch-controls.md`
  and `examples/08_dropzone.rs`.
- Lay out an on-screen left stick (thrust / nav intent), a right-side aim area,
  and buttons for the core verbs (GOTO / ORBIT / STOP, radar lock, fire),
  revealed only once the screen is first touched.

Done when: the menus and the editor are fully operable with a gamepad, and the
web build is playable on a touchscreen via a virtual pad built on the
bevy-common-systems primitives.

## Notes

- Forward-looking backlog for the v0.6.0 cycle; depends on nothing shipped.
- Fuzzy enough that it likely wants a `/spike` first (menu-nav approach: Bevy UI
  focus vs a custom focus ring; the virtual-pad layout and which verbs get
  on-screen buttons), then `/plan` per part - Part A and Part B are independently
  shippable and could split into two tasks at plan time.
- `bevy-common-systems` is a git dependency with a local checkout at
  `~/personal/bevy-common-systems`; if the touch primitives need extending, make
  the change there (same task flow) and bump the pinned `rev` here.
- Touch points in this repo: menu widgets are bevy `ui_widgets`
  (`Button`/`Activate`) in `nova_menu`; the editor build UI is in `nova_editor`;
  existing gamepad reads are `Option<Res<ButtonInput<GamepadButton>>>` (see
  `nova_editor`, `nova_menu`, `nova_gameplay/hud`) and the `bindings!` flight rig
  in `nova_gameplay/src/input/player.rs`.
