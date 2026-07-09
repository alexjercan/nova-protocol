# Hit feedback / game juice (camera shake, hit flash, impact FX)

- STATUS: CLOSED
- PRIORITY: 88
- TAGS: v0.4.0,polish,destruction

Spike: docs/spikes/20260708-161726-modding-language-and-scripting.md (roadmap)

The destruction pipeline already spawns mesh fragments, but there is little
moment-to-moment feedback when a shot lands or the player takes damage. Add
"juice": camera shake on impact/detonation, a brief hit flash on damaged sections,
and small impact FX at collision points. Drive it off existing signals
(`HealthApplyDamage`, collision events, `IntegrityDestroyMarker`). Keep it
wasm-safe (prefer shader/gizmo/transform effects over particles where the particle
system is still wasm-blocked, 162908).

## Design

New module `crates/nova_gameplay/src/juice.rs` (sibling to `audio.rs`), plugin
`NovaJuicePlugin`, wired in `plugin.rs`. It mirrors `audio.rs`: fire off the same
event seams (`On<HealthApplyDamage>`, `On<Add, IntegrityDestroyMarker>`),
distance-attenuate from the gameplay camera, and per-area-cell throttle so a blast
hitting many colliders in one frame collapses to one cue. All logic that a headless
run cannot exercise (rendering) is pushed into pure helpers so it is unit-testable,
exactly as the audio retro recommended.

Effects:

1. **Camera shake** - reuse the bcs `CameraShakePlugin` (trauma model, drift-free,
   ordered around `ChaseCameraSystems::Sync`). Add the plugin; ensure the gameplay
   `Camera3d` carries a `CameraShake` configured from settings; observers add trauma
   on damage (small) and destruction (large), scaled by distance to the camera and
   throttled per cell.
2. **Impact / hit-flash FX (gizmos)** - a bounded `ActiveJuiceFx` resource of
   `{ pos, start_secs, kind }`; the same observers push a flash (throttled per cell).
   A draw system renders each as a camera-facing expanding, fading ring (impact =
   small/fast, destroy = large/slow) and prunes expired ones. Wasm-safe, zero asset
   churn. Section materials live in shared/buried gltf children, so an overlay flash
   is chosen over recoloring them (per-section emissive flash is a possible follow-up).

3. **Tweakable `JuiceSettings` resource** (`Resource + Reflect + Default`) holds every
   tunable and per-effect `enabled` toggles plus a `master_enabled`, so a settings menu
   can bind to it later.

## Steps

- [x] Add `JuiceSettings` resource with shake + flash sub-configs, `Default`, `Reflect`.
- [x] Add `NovaJuicePlugin`: add bcs `CameraShakePlugin`, init resources, register
      observers + systems; wire it into `NovaGameplayPlugin`.
- [x] Camera shake: `ensure_camera_shake` attaches `CameraShake` from settings to the
      gameplay camera; observers feed `CameraShakeInput.add_trauma` (damage/destroy),
      distance-attenuated + per-cell throttled.
- [x] Flash FX: `ActiveJuiceFx` resource, observers push flashes (throttled), draw
      system renders camera-facing expanding/fading rings and prunes expired ones.
- [x] Pure helpers (`distance_falloff`, throttle, `flash_radius`, `flash_alpha`) with
      unit tests, observer-level integration tests, and a "settings defaults" guard.
- [x] `cargo fmt`, `cargo clippy --all-targets`, `cargo test --workspace` all green.
- [x] Design note in `docs/2026-07-09-hit-feedback-juice.md`.
</content>
