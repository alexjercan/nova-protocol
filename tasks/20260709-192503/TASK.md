# Hybrid lock acquisition: aim cone + signature-range proximity

- STATUS: OPEN
- PRIORITY: 58
- TAGS: v0.4.0,targeting,gameplay,spike


Spike: docs/spikes/20260709-192358-component-lock-vats-lite.md

Extend `update_spaceship_target_input` (input/player.rs): keep the instant
aim-cone pick; when the cone finds nothing, auto-acquire the nearest
`AISpaceshipMarker` ship root within a shorter SIGNATURE_RANGE (heat-signature
close-range lock; start ~500-600 m vs the 2000 m cone range). Minimal hostile
definition = AI ships until the faction model (20260708-203708) lands.
Consider the mechanical rename of `SpaceshipPlayerTorpedoTargetEntity` to a
general target-lock name here (three systems consume it after this arc).
