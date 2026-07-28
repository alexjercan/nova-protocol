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

## Steps (refined from SPIKE.md, 2026-07-28)

Accepted direction (SPIKE.md D1): phosphor terminal is the PRIMARY skin with
CLI-rendered widgets; hardware casing is a SECONDARY alternative.

- [ ] Swap `nova_ui::theme` to the NOVA OS phosphor tokens mirrored from
      `nova_os_terminal_poc.html` (`--case-*`, `--phosphor(-dim/-muted)`,
      `--amber`, `--orange`, `--screen-*`, `--mono`); keep the current
      navy/cyan values only as the hardware-skin token set. Re-check semantic
      HUD accents against the phosphor base.
- [ ] Introduce a UI SKIN concept in nova_ui (Phosphor primary | Hardware): a
      resource/component the widgets read, defaulting to Phosphor. (The web
      easter egg 20260728-185730 exposes it as a Settings option; the game may
      too - a Graphics/Interface setting.)
- [ ] Rework ThemedButton to render BOTH skins: phosphor = flat 1px border +
      `>` cursor marker + INVERTED selection (phosphor fill, dark glyphs);
      hardware = light-3D bevel (gradient face, lit top edge, deep bottom,
      pressed inset). States idle/hover/pressed/selected/disabled per demo 1's
      widget zoo.
- [ ] Add the shared widget set consumed by every screen: segmented control,
      slider (ASCII-block meter in phosphor / bevelled track in hardware),
      toggle, panel header + separator (dashed rule in phosphor), badges
      (bracketed `[TAG]` in phosphor), list row, checkbox.
- [ ] Typography: route the mono UI font (Iosevka Term, already shipped) as the
      primary UI typeface per the demo; size scale from the demo.
- [ ] Fold nova_menu's `MenuButton` + `update_button_colors` into the shared
      nova_ui observer path so there is ONE observer/reconciler for buttons.
- [ ] Ship a widget-zoo screenshot example that renders the full widget set in
      both skins (the eyeball rig for this task).

## Definition of Done (refined 2026-07-28)

1. example + render eyeball: a widget-zoo example renders button / segmented /
   slider / toggle / badge / row / header in both skins; screenshot reviewed.
2. test: live-tree tests assert ThemedButton state reconciliation
   (idle/hover/pressed/selected/disabled) drives the right markers in each skin.
3. cmd: `grep -rn 'update_button_colors\|MenuButton' crates/nova_menu` shows the
   duplicate colour system is gone (one shared observer path); recorded here.
4. manual: owner eyeballs the widget zoo in-engine in both skins; phosphor is the
   default and its widgets read as CLI elements, not bevelled buttons on glass.
