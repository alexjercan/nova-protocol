# Render-scale Low breaks UI clicks: HUD/menu unclickable because UI is on the image-targeted camera

- STATUS: CLOSED
- PRIORITY: 60
- TAGS: v0.7.0, performance, web, settings, bug

## Goal

Switching graphics to Low (task 20260718-004723's render-scale lever) makes the
resolution drop correctly, but all UI stops responding to clicks - including the
settings menu button that would switch back off Low, so the player is stuck.

Root cause: the reconcile marked the scenario `Camera3d` `IsDefaultUiCamera` and
redirected it to render into the offscreen image, so the HUD/menus rendered into
that image. But bevy_ui's `ui_focus_system` (bevy_ui `focus.rs`) only resolves a
cursor for cameras whose render target is a `Window`; an image-targeted UI camera
never receives the cursor, so its nodes are never hit. Rendering worked; picking
did not.

"Done" = on Low the resolution still drops AND every UI element (menus + HUD) is
clickable, with world-space HUD markers still correctly positioned.

## Steps

- Move the default UI camera off the scenario `Camera3d` onto the blit
  `Camera2d`, which targets the window - so UI is clickable and renders
  full-resolution (crisp) over the upscaled world.
- Keep the world->screen HUD projection aligned by setting the scenario camera's
  image-target `scale_factor` so it reports the window's logical viewport
  (`logical = physical / scale_factor`), instead of sharing a coordinate space
  with the UI.
- Drop the RenderLayers isolation (the blit is the only Camera2d and Camera3d
  never draws 2D sprites, so it is unnecessary and would block UI from rendering
  on the blit camera).
- Regression test: assert the scenario camera is NOT `IsDefaultUiCamera` and the
  blit (window) camera IS - a UI camera on a window target is the invariant that
  fixes clicks.
- Verify in the real app: Low renders the world upscaled with a CRISP, clickable
  HUD, and edge indicators land at the same positions as High.

## Notes

- Fix lives in `crates/nova_scenario/src/render_scale.rs`. `screen_indicator` is
  the only world->screen HUD projector (small blast radius); the `scale_factor`
  trick keeps it unchanged.
- Follows task 20260718-004723 (the lever itself). Found by the user immediately
  on switching to Low.
