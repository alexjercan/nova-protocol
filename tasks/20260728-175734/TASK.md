# nova_ui theme + widgets: NOVA OS palette + light-3D treatment

- STATUS: OPEN
- PRIORITY: 40
- TAGS: v0.9.0,ui,refactor

## Story

`nova_ui::theme` still carries the flat navy/cyan/amber language the owner
called washed out; the NOVA OS monitor proved the replacement. Rework the
shared theme + widget vocabulary to the accepted spike direction: NOVA
OS-derived palette, mono-first typography (Iosevka Term is already shipped),
and a light-3D "physical control" widget treatment (BackgroundGradient faces,
BoxShadow bevels, lit top edge / deep bottom edge, pressed inset) as the
shared building blocks every screen consumes. Unify nova_menu's duplicate
MenuButton color system onto the shared observers while touching the widget
layer.

## Steps (direction-level - refined at spike close)

- [ ] DIRECTION: palette swap in `nova_ui::theme` per the accepted demo
      tokens; semantic HUD accents re-checked against the new base.
- [ ] DIRECTION: ThemedButton + segmented control + slider + panel header /
      separator / badges get the light-3D treatment; idle/hover/pressed/
      selected/disabled states per the demo widget zoo.
- [ ] DIRECTION: typography pass (font routing + sizes) per demo.
- [ ] DIRECTION: nova_menu's MenuButton + update_button_colors fold into the
      shared nova_ui widget system (one observer path).
- [ ] Refine these into real Steps/DoD from the accepted SPIKE.md before any
      implementation.

## Definition of Done (direction-level - refined at spike close)

1. Refined at spike close. Must include at minimum: a widget-state screenshot
   example (eyeball-the-rendered-output), live-tree tests for reworked
   observers/reconcilers, and a manual owner eyeball per screen family.
