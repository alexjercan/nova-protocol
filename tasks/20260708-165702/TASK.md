# Locked-target info readout: distance, closing speed, health (HUD)

- STATUS: OPEN
- PRIORITY: 70
- TAGS: v0.4.0,hud,torpedo,spike

Spike: docs/spikes/20260708-165647-weapons-hud.md

Phase 1 cheap win: alongside the target reticle, show the locked target's range
(|target - ship|), closing speed (relative velocity along the line of sight, from
`LinearVelocity`), and a small health bar (`Health` is on targets). All data is
already queryable; this is rendering only, wasm-safe.

The locked target comes from `SpaceshipPlayerTorpedoTargetEntity`. Prefer building on
the screen-projected-indicator widget (20260708-165700) so the readout tracks the
target anchor the same way the reticle does.
</content>
