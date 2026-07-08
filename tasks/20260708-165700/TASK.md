# Screen-projected-indicator widget (HUD substrate)

- STATUS: OPEN
- PRIORITY: 78
- TAGS: v0.4.0,hud,spike

Spike: docs/spikes/20260708-165647-weapons-hud.md

Generalize the `torpedo_target` world-to-viewport trick into a reusable widget so
every weapons-HUD indicator (lead pip, target info, edge arrows) is a thin consumer
instead of a fresh copy of projection + visibility + off-screen handling.

Direction: a component holding a world anchor (entity or fixed point) plus styling,
a system that positions/hides its UI node each frame via
`Camera::world_to_viewport`, and optional edge-clamping with a direction arrow when
the anchor is off-screen. Keep it UI-pass (not a second Camera2d, not gizmos - see
`hud/torpedo_target.rs`). Clean candidate to promote to `bevy_common_systems` once a
second nova consumer exists. This is the substrate the rest of the weapons-HUD arc
builds on; land it before the third bespoke overlay.
</content>
