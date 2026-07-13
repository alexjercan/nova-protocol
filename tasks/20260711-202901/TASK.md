# Diegetic HP: move the health readout onto the ship

- STATUS: OPEN
- PRIORITY: 0
- TAGS: v0.6.0,hud,ui,spike

Goal: improve the HP UI by making it diegetic - the health readout should
live on/in the ship somehow instead of the current generic screen-space
health bar (bevy_common_systems HealthDisplay, spawned in
crates/nova_gameplay/src/hud/mod.rs setup_hud_health). Requested by the
user 2026-07-11 during the menu/HUD flow.

Needs a spike to pick the shape. Candidate directions to weigh (not
decided):
- Per-section damage tinting/glow on the ship's own meshes (integrity data
  already lives per section; sections disable/explode on death today).
- A ship-anchored chip in the flight-status family (speed/mode chips
  already ride the ship via the screen_indicator widget, nav-cyan
  palette) - a hull integrity chip beside them.
- A schematic mini-ship (top-down section outline) that colors per-section
  health, either as a HUD widget or projected near the ship.

Context for the spike:
- Health lives per-section (integrity/) with a ship aggregate; the
  current bar reads the generic Health component on the root.
- The HUD tier system (HudVisibility/HudTier, task 20260711-180501) tags
  the current bar Instrument; a diegetic replacement should inherit that.
- Related promotion note in docs/architecture.md: hud/health.rs is listed
  as a bevy_common_systems promotion candidate - a diegetic replacement
  changes that calculus; retire or keep the generic bar for other games.

Notes:
- Spike first (SPIKE.md in the task folder), then plan.
- Related: tasks/20260710-234019/SPIKE.md (the
  chips this could join), crates/nova_gameplay/src/integrity/.

