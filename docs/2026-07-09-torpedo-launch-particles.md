# Torpedo bay launch particles

Task: `20260525-133024` (v0.4.0, torpedo). Legacy #135.

## What changed

The torpedo bay now emits a particle burst at its spawner every time a torpedo
leaves the tube, giving the same kind of firing feedback the turret already has.

- `TorpedoSectionConfig` gained a `launch_effect: Option<Handle<EffectAsset>>`
  field (default `None`), mirroring the existing `blast_effect`.
- The bay spawner carries a `TorpedoSectionSpawnerEffect` holder component; on
  spawner add, `insert_torpedo_spawner_effect` parents a `ParticleEffect` child
  to it (marked `TorpedoSectionSpawnerEffectMarker`). When `launch_effect` is
  `None` it builds a default `torpedo_launch_burst`: a spawn-on-command effect
  (`with_emit_on_start(false)`) so it only emits when explicitly triggered.
- When a torpedo projectile is spawned, `on_torpedo_launch_effect` (an
  `On<Add, TorpedoProjectileMarker>` observer) looks up the spawner via the
  projectile's `TorpedoSectionSpawnerEntity`, points the burst along the
  spawner's launch axis (its `up`), and calls `EffectSpawner::reset()` to emit
  one puff.

Both new observers are registered in the plugin's `render` block and gated
`#[cfg(not(target_family = "wasm"))]`, exactly like the existing hanabi blast
burst (`insert_particle_effect`), because bevy_hanabi does not run on wasm
(FIXME 20260706-162908).

## Why this design

This is a direct mirror of the reviewed turret muzzle-effect pattern
(`insert_turret_barrel_muzzle_effect` + `on_projectile_marker_effect` in
`turret_section.rs`): a spawn-on-command effect parented to the firing point,
triggered by the projectile's Add observer through a runtime property + a
`reset()` call. Reusing the established pattern keeps the two weapon sections
consistent and stays inside shapes that have already passed review.

The effect asset itself is authored inline rather than loaded from disk, again
matching both the turret muzzle flash and the torpedo blast burst. The default
burst is a cold white-blue propellant flash (baked color/size gradients) to read
as a launch, distinct from the turret's hot-orange muzzle flash. `normal` and
`base_velocity` are the only runtime properties; color and size are baked, since
they do not vary per shot.

## Alternatives considered

- **A wasm-safe mesh fallback** (like `BlastRadiusVisual` for the blast). The
  blast has one because a detonation is a gameplay-significant event that must be
  legible on every platform; a muzzle/launch flash is pure juice, and the sibling
  turret muzzle effect ships hanabi-only with no wasm fallback. Matching the
  turret keeps scope focused; a wasm launch flash can be a separate task if
  wanted.
- **Triggering from `shoot_spawn_projectile` directly** instead of an Add
  observer. The turret drives its muzzle flash from the projectile Add observer,
  not the shoot system, so the effect logic stays in the render module and out of
  the physics/spawn path. Followed the same split here.

## Verification

`cargo fmt` + `cargo check -p nova_gameplay` clean (only an unrelated upstream
`proc-macro-error2` future-incompat warning).

Smoke test via the `06_torpedo_range` autopilot harness (built once, run
directly with the asset root per the range runbook):

```text
cargo build --example 06_torpedo_range --features debug
BEVY_ASSET_ROOT=$PWD DISPLAY=:99 BCS_AUTOPILOT=1 ./target/debug/examples/06_torpedo_range
```

Output showed `reached Playing`, repeated `range: torpedo fired` (each firing
builds/triggers the launch burst and calls `reset()`), and
`autopilot: cycle complete, no panic (t=6.0s)` with exit 0. No
`effect for spawner ... not found` errors, which confirms the spawner's effect
child exists and is found on every shot (observer ordering is correct because the
projectile is spawned already carrying `TorpedoSectionSpawnerEntity`).

## Not verified here (playtest item)

The on-screen look of the burst (color, size, spread, whether it reads as a
launch at gameplay distance) is a GPU-rendered, sub-0.35s transient that a
headless no-panic run cannot assert. The functional path is verified; the visual
tuning is left for playtest, consistent with how prior visual tasks flagged
detent/feel properties.
