# Blast radius visual

- STATUS: CLOSED
- PRIORITY: 50
- TAGS: v0.4.0,torpedo

Shader or particle effect on detonation. Legacy #147.

Pulled into v0.4.0 (roadmap spike 20260708-161726): completes torpedo detonation
feedback. Prefer a shader/gizmo expanding-sphere over particles so this is NOT
blocked by the wasm particle issue (162908) - unlike the bay-particles task
(133024), which stays in v0.5.0 for that reason.

## Resolution (CLOSED - 2026-07-08)

Added a wasm-safe expanding-sphere blast visual in
`crates/nova_gameplay/src/sections/torpedo_section/render.rs`:

- `insert_blast_radius_visual` (observer on `Add<BlastDamageMarker>`, render-gated but
  NOT wasm-gated, unlike the hanabi `insert_particle_effect`) spawns a unit-sphere
  mesh + per-instance translucent emissive `StandardMaterial` at the detonation, read
  from the blast's `BlastDamageConfig.radius`. The sphere mesh is shared via a `Local`
  cache; only the material is per-instance so each blast fades independently.
- `animate_blast_radius_visual` grows the sphere (ease-out cubic) to exactly the blast
  radius while fading it out over 0.4s, then despawns it and frees its material (no
  asset leak). The growth/fade curve is a pure `blast_visual_step` helper, unit-tested
  (4 tests).

Chose a mesh sphere over gizmos (styleable, matches the UI/mesh HUD convention) and
over particles (wasm-blocked, 162908). It shows the blast's actual area of effect, so
it complements the fire/smoke particle burst rather than replacing it.

Verified: 52 lib tests pass, clippy/fmt clean, `cargo check --workspace` clean, and
the `06_torpedo_range` example under the autopilot harness fires + detonates torpedoes
repeatedly and reaches "cycle complete, no panic" - exercising the new observer and
animation system through real detonations in a render app.
