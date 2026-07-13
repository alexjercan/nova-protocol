# Torpedo bay shooting particles

- STATUS: CLOSED
- PRIORITY: 48
- TAGS: v0.4.0, torpedo

Visual feedback when firing. Legacy #135.

A launch particle burst at the torpedo bay spawner, fired each time a torpedo
leaves the tube. Mirrors the reviewed turret muzzle-effect pattern
(`insert_turret_barrel_muzzle_effect` + `on_projectile_marker_effect` in
`turret_section.rs`): a spawn-on-command hanabi effect parented to the spawner,
triggered by the torpedo-projectile Add observer via `EffectSpawner::reset()`.
Hanabi-only and wasm-gated, like the existing torpedo blast burst.

## Steps

- [x] Add `launch_effect: Option<Handle<EffectAsset>>` to `TorpedoSectionConfig`
      (default `None`), mirroring the existing `blast_effect` field.
- [x] Add the effect holder + marker components
      (`TorpedoSectionSpawnerEffect(Option<Handle<EffectAsset>>)`,
      `TorpedoSectionSpawnerEffectMarker`) and carry the holder on the spawner
      entity in `insert_torpedo_section`.
- [x] Add `insert_torpedo_spawner_effect` observer (on `Add
      TorpedoSectionSpawnerMarker`) in `render.rs`: use the configured effect or
      build a default `spawn_on_command` launch burst (emit-on-start false) with
      `normal` / `base_velocity` properties, parented to the spawner. (Color and
      size are baked gradients since they do not vary per shot, so no
      `spawn_color` property was needed.)
- [x] Add `on_torpedo_launch_effect` observer (on `Add TorpedoProjectileMarker`)
      in `render.rs`: find the spawner via the projectile's
      `TorpedoSectionSpawnerEntity`, set normal (spawner `up`)/base-velocity
      properties, and call `effect_spawner.reset()`.
- [x] Register both observers in the plugin's `render` block, gated
      `#[cfg(not(target_family = "wasm"))]`, like `insert_particle_effect`.
- [x] Verify: build the `06_torpedo_range` example binary, run the autopilot
      smoke test under Xvfb, confirm firing spawns the burst with no panic.
      `cargo fmt` + `cargo check`.
- [x] Document the change and any diagnosis notes under `docs/`.

## Resolution

Implemented a launch particle burst at the torpedo bay spawner, fired on every
shot, mirroring the turret muzzle-effect pattern. Also updated the explicit
`TorpedoSectionConfig` construction in `nova_assets/src/sections.rs` for the new
`launch_effect` field. Default burst is a cold white-blue propellant flash; a
config-supplied `launch_effect` overrides it. Hanabi-only, wasm-gated.

Verified with `cargo check -p nova_gameplay` and the `06_torpedo_range`
autopilot smoke test (reached Playing, repeated fired, cycle complete no panic,
exit 0). Visual look is a playtest item. See
`tasks/20260525-133024/NOTES.md`.
