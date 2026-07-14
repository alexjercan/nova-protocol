# Spike+impl: ScatterObjects scenario action (seeded procedural object scatter)

- STATUS: CLOSED
- PRIORITY: 68
- TAGS: v0.6.0, modding, scenario, spike

Surfaced porting the built-ins to RON (133028): `menu_ambience` (14 rocks) and
`asteroid_field` (20 rocks) scatter asteroids with runtime RNG at build time.
Freezing that into static RON loses the procedural intent and bloats the file.
A declarative modding format should express "scatter N objects in a volume", so
add it as a new action rather than freeze.

## Design (decided)

New `EventActionConfig::ScatterObjects(ScatterObjectsConfig)`:

```
struct ScatterObjectsConfig {
    id_prefix: String,       // spawned ids: "{prefix}{i}"
    count: u32,
    seed: u64,               // deterministic: seeded StdRng (data files must be reproducible)
    region: ScatterRegion,
    template: ScenarioObjectConfig,   // cloned per object; base.id/position overwritten
    asteroid_radius: Option<(f32, f32)>, // if Some and template kind is Asteroid, randomize radius
}
enum ScatterRegion {
    Box { min: Vec3, max: Vec3 },
    Ring { inner: f32, outer: f32, y_min: f32, y_max: f32 }, // horizontal annulus about origin
}
```

At action time (typically OnStart), seed `StdRng::seed_from_u64(seed)` and push
`count` spawn commands: clone `template`, set `base.id = format!("{id_prefix}{i}")`,
`base.position` = uniform sample in `region` (Box: per-axis uniform; Ring: uniform
angle + radius in [inner,outer] + y in [y_min,y_max]), and randomize the asteroid
radius when configured. Reuse the existing spawn path (the same code
`SpawnScenarioObject` uses). Determinism via the seed means the same file yields the
same layout every load - a property `SpawnScenarioObject` scatter never had.

serde: all fields are pure data (Vec3 via bevy/serialize), so cfg_attr serde like
the rest of the tree. Unit test: fixed seed -> fixed positions (regression pin) +
a RON round-trip.

Used by the ported `menu_ambience` (Ring) and `asteroid_field` (Box) scenarios.
