# Audio/SFX system (thrust, weapons, explosions, impacts)

- STATUS: OPEN
- PRIORITY: 55
- TAGS: v0.5.0, audio

Spike: docs/spikes/20260708-161726-modding-language-and-scripting.md (roadmap)

There is no audio in the game today. Add a sound layer for the core feedback
moments: thruster engine loop, turret fire, torpedo launch, explosions/detonation,
and collision/impact hits. Hook it off the existing gameplay events where possible
(the integrity `IntegrityDestroyMarker` seam already fires on destruction; weapons
already have fire events). Keep it wasm-safe. Consider whether the trigger points
should be a promotable `bevy_common_systems` concern (generic "play sound on
event") vs game-specific asset wiring.
</content>
