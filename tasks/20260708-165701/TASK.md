# Turret lead/intercept pip (HUD)

- STATUS: OPEN
- PRIORITY: 72
- TAGS: v0.4.0,hud,turret,spike

Spike: docs/spikes/20260708-165647-weapons-hud.md

Phase 1 cheap win: draw the turret's already-computed intercept point. The turret
plugin computes `TurretSectionAimPoint(Option<Vec3>)` via `lead_intercept_point`
(crates/nova_gameplay/src/sections/turret_section.rs) - the point to aim at to hit a
moving target - but nothing renders it. Add a HUD pip at that projected point so the
player can see the lead the turret is taking.

Pure rendering off existing data, wasm-safe. Should be a first consumer of the
screen-projected-indicator widget (20260708-165700) rather than another one-off
system.
</content>
