# Mobile virtual pad: the web build playable by touch

- STATUS: OPEN
- PRIORITY: 45
- TAGS: v0.15.0,input,mobile,touch

Split 2026-08-31 out of `20260714-001140`, which v0.13.0 rescopes to real
gamepad hardware. The touch half stays backlog so its layout targets
interactions proven with a pad first. Current-state audit:
`tasks/20260815-231945/INPUT-AND-PROCESS.md`.

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

Done when: the web build is playable on a touchscreen via a virtual pad
built on the bevy-common-systems primitives.

## Scheduled for v0.15.0

Pulled onto the board at priority 45 on 2026-09-21 by owner decision, in epic
`20260921-231507`. The work is mechanical: the touch primitives it builds
on already ship in `bevy-common-systems`.

It keeps its dependency. This task FOLLOWS `20260714-001140` (gamepad) and
starts from that task's interaction findings - the pad layout, the chord
decisions, and the dead-zone and focus behaviour proven on hardware. Do not
commit a touch layout for a verb whose pad interaction is still open.
