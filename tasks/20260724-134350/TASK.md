# Drawer right panel: objectives as a styled list (not plain text)

- STATUS: OPEN
- PRIORITY: 54
- TAGS: v0.9.0,feature,ui,hud

## Goal

Playtest rework (owner, 2026-07-24): the drawer's RIGHT panel shows objectives,
but they are "just text right now - we need to make it prettier". Rework the
objectives section from the plain text lines the shell spawned into a proper
styled LIST.

Scope (direction-level; /plan breaks into steps at pickup):

- Rework the drawer's objectives section (`hud/drawer.rs`, the
  DrawerObjectivesListMarker container spawned by the shell 20260724-102304) from
  plain `Text` lines into a styled list: per-item rows with a bullet/status glyph,
  consistent spacing, done/active states, panel styling that matches the drawer
  chrome (nova_ui::theme). Right panel, slides in from the right.
- Keep it data-driven from bevy_common_systems GameObjectives (already synced).

## Notes

- From the 2026-07-24 playtest. Extends the shell's objectives section
  (20260724-102304, LANDED) which rendered plain text as a placeholder. Pairs with
  the drawer-open rework (20260724-134335) and the flight objective surface
  (20260724-134312).
- Files: hud/drawer.rs (rebuild_drawer_objectives + the section container).
