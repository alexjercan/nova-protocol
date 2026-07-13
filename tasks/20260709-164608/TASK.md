# Promote screen-indicator widget to bevy_common_systems

- STATUS: OPEN
- PRIORITY: 0
- TAGS: v0.6.0,hud,refactor,spike

Spike: tasks/20260709-164502/SPIKE.md

Once the weapons-HUD arc lands, `hud/screen_indicator.rs` has ~4 nova consumers
(torpedo reticle, autopilot destination marker, turret lead pip, locked-target
readout) and its API is exercised. Move it to bevy-common-systems (it matches
the house widget style: bundle constructor, marker, plugin, system set,
prelude; avian3d is already a dep there for the ApparentSize AABB union), bump
the pinned rev in nova, and re-export via the prelude so nova consumers only
change imports. Keep `ScreenIndicatorCamera` as the generic camera marker.

