# Hit feedback / game juice

Task: `tasks/20260708-162013/TASK.md`. Adds moment-to-moment combat feedback
("juice") on top of the existing destruction pipeline: camera shake on
impact/detonation, and gizmo impact/hit-flash rings at the event location. All of
it is tunable through a single reflected `JuiceSettings` resource so a settings
menu can bind to it later.

## Where it lives

New module `crates/nova_gameplay/src/juice.rs` (sibling to `audio.rs`), plugin
`NovaJuicePlugin`, wired into `NovaGameplayPlugin` right after the audio plugin.
It is deliberately modelled on `audio.rs`, because juice and sound are the same
shape of problem: react to a handful of gameplay events, attenuate by distance to
the player, and throttle co-located bursts.

## Signals it hooks

The same two seams the audio layer uses, so no gameplay system needs to know juice
exists:

- `On<HealthApplyDamage>` -> an **impact** kick + flash (a shot landed on a living
  target);
- `On<Add, IntegrityDestroyMarker>` -> a **destruction** kick + flash (a section /
  asteroid died or a torpedo detonated; everything funnels through this marker).

Both observers resolve the event's world position from the target's
`GlobalTransform` and hand off to one shared `emit_juice` path, so impact and
destruction differ only in their tunables.

## Effects

### Camera shake (reused from bevy-common-systems)

bcs already ships a `CameraShakePlugin`: the classic **trauma** model (game code
adds `0..1` trauma on impact, it decays over time, and while positive the camera
gets a random offset of magnitude `trauma^exponent`). Crucially it is **drift-free**
- it un-applies the previous frame's offset before the base-writing driver runs and
re-applies a fresh one after, so it composes with the chase camera automatically and
never accumulates. Nova had not added it yet.

So this module does not implement shake at all; it only *feeds* it:

- `ensure_camera_shake` attaches a `CameraShake` (configured from `JuiceSettings`)
  to any gameplay `Camera3d` that lacks one. It runs every frame but no-ops once the
  component exists, which transparently handles Nova swapping the camera's controller
  between WASD and chase (the entity persists).
- `sync_camera_shake_config` pushes live `JuiceSettings` changes onto the existing
  `CameraShake` when the resource changes, so a settings-menu edit takes effect
  without respawning the camera.
- The two observers write `CameraShakeInput.add_trauma` (attenuated + throttled).

### Impact / hit-flash FX (gizmos)

An `ActiveJuiceFx` resource holds a bounded `Vec<Flash>` (`pos`, `start_secs`,
`kind`, `strength`). The observers push a flash with the distance falloff captured
as its `strength`; `draw_juice_flashes` (in `PostUpdate`, after transform
propagation) renders each as one or more **camera-facing rings** that expand
(ease-out radius) and fade (quadratic alpha scaled by `strength`, so a far event's
ring is faint from its first frame - radius stays world-scale since perspective
already shrinks distant rings) over the flash lifetime, then prunes the ones that
have elapsed. Impact rings are small and quick; destruction rings are large and
slower.

Why gizmos rather than spawned meshes or particles:

- **wasm-safe** - the Hanabi particle system is still wasm-blocked (162908); gizmos
  are immediate-mode and always available.
- **zero churn** - a blast that damages a dozen colliders of one ship in a single
  frame would otherwise spawn a dozen mesh+material entities. Gizmos allocate
  nothing per hit.
- Section render meshes are gltf-instanced children with **shared** materials, so
  recoloring "the damaged section" would risk flashing every instance that shares
  the material. An overlay ring sidesteps that entirely. A true per-section emissive
  flash is left as a possible follow-up.

## Distance attenuation + throttling

Both effects are attenuated by distance from the gameplay camera (the same listener
the audio layer uses) and throttled per quantized world cell, so:

- a distant skirmish shakes/flashes weakly (or not at all past `far_distance`),
- a co-located burst - a blast hitting many colliders, or a multi-section ship dying
  in one frame - collapses to a single kick and a single ring.

Fully-attenuated events return before even stamping the throttle, so a far event
stays silent without consuming throttle state.

### Tuning note (subtlety + distance feel)

Initial playtest feedback was that the shake was too strong. The point-blank
impulses were dialled down (`hit_trauma` 0.18 -> 0.08, `destroy_trauma` 0.5 ->
0.24, smaller `max_offset`/`max_kick`, faster `decay`), and the shake's distance
falloff was pulled *tighter* than the sound's (`near_distance` 20 -> 8,
`far_distance` 320 -> 200). The result: only a hit on your own hull produces a real
bump, while a detonation across the arena is a faint tremor - the shake falls off
with distance noticeably faster than the audio does, which is the intended feel.

## Tweakability (the settings-menu hook)

`JuiceSettings` is a `Resource + Reflect` (`#[reflect(Resource)]`) with:

- `master_enabled` - one switch to kill all juice (a "reduce motion" toggle);
- `shake: ShakeSettings` - per-effect `enabled`, the two trauma impulses, and the
  `CameraShake` config (decay / max_offset / max_kick / exponent);
- `flash: FlashSettings` - per-effect `enabled`, per-kind colors / radii / durations,
  and ring count;
- `near_distance` / `far_distance` - the attenuation ramp.

Because it is reflected and inserted with `Default`, a future settings menu can edit
it live; `sync_camera_shake_config` already propagates shake edits to the running
camera.

## Testing

The rendering can't be exercised headlessly, so all the math lives in pure helpers
that are unit-tested: `distance_falloff` (full/zero endpoints, monotonic ramp,
degenerate-range guard), the throttle (per-key independence, pruning), `area_cell`
grouping, and the flash `progress`/`radius`/`alpha` curves. A `default_settings_are_sane`
test guards the invariants (destruction out-shakes/out-flashes impact, ranges
well-formed, master switch disables both). `cargo test --workspace` is green and the
crate `cargo check`s for `wasm32-unknown-unknown`.
