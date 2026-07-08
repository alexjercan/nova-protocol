# Hit feedback / game juice (camera shake, hit flash, impact FX)

- STATUS: OPEN
- PRIORITY: 50
- TAGS: v0.5.0, polish

Spike: docs/spikes/20260708-161726-modding-language-and-scripting.md (roadmap)

The destruction pipeline already spawns mesh fragments, but there is little
moment-to-moment feedback when a shot lands or the player takes damage. Add
"juice": camera shake on impact/detonation, a brief hit flash on damaged sections,
and small impact FX at collision points. Drive it off existing signals
(`HealthApplyDamage`, collision events, `IntegrityDestroyMarker`). Keep it
wasm-safe (prefer shader/gizmo/transform effects over particles where the particle
system is still wasm-blocked, 162908).
</content>
