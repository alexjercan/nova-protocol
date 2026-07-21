# Bug: F11 debug toggles inverted after cursor fix - boot all debug off, toggle in phase

- STATUS: OPEN
- PRIORITY: 88
- TAGS: v0.8.0, bug, hud, input

## Story

Playtest (owner, 2026-07-21), regression from 20260721-211500 (hide cursor in
flight): the F11 debug toggles are now INVERTED. At boot the nova gizmo
overlays show but the egui inspector + avian physics debug are hidden; pressing
F11 (expecting to hide everything) instead HIDES the gizmos and SHOWS the
inspector UI + avian physics. F11 should move the whole debug layer together.

Cause: the cursor fix flipped ONLY the bcs egui-inspector `DebugEnabled` to
false (so flight boots cursor-free) but left the three sibling F11-toggled
debug states at their `true` default, desyncing them. There are FOUR
F11-toggled states, all previously in phase:
1. nova_debug `DebugEnabled` -> gravity/sections gizmos
2. bcs inspector `DebugEnabled` -> inspector UI + avian PhysicsGizmos + physics UI
3. bcs wireframe `DebugEnabled` -> wireframe pass
4. nova_gameplay `AmmoReadoutDebug` -> debug ammo number

## Steps

- [ ] Flip all four F11-toggled debug defaults to false so a dev build boots
      clean (no overlays, no inspector, no avian, cursor hidden in flight) and
      F11 raises the whole debug layer together:
      - nova_debug: nova `DebugEnabled(false)`, override bcs wireframe
        `DebugEnabled(false)` (inspector already false from 20260721-211500).
      - nova_gameplay: `AmmoReadoutDebug` default false + fix its doc comment.
- [ ] Verify nothing relies on the old default-on (screenshots/reels call
      hide_dev_overlays, idempotent; no test asserts default true).
- [ ] Test: a headless assertion that the four defaults agree (all off), so a
      future single-flip desync is caught.
- [ ] Docs: fold into the same CHANGELOG Fixes entry / note the dev-build
      debug-layer default flip.

## Definition of Done

- All four F11 debug states share one default (off); F11 toggles them in phase
  (test: default-agreement pin; manual: owner presses F11 once - everything
  debug appears together - and again - everything hides, cursor re-locks).
- Cursor stays hidden in flight at boot (the 20260721-211500 behavior holds).

## Notes

- Regression of 20260721-211500; same subsystem. The coherent model: in a dev
  build ALL debug tooling is off at boot; F11 toggles the whole layer; flight
  hides the cursor whenever the inspector panel is down.
