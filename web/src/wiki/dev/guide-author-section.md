# Author a section (RON)

A how-to for content authors. A `Section` is the other kind of `*.content.ron`
item alongside `Scenario` (see [Author a scenario](../guide-author-scenario/)):
a ship part - hull, thruster, controller, turret, or torpedo bay - authored as
RON data, no Rust. A mod ships sections to add new parts to the editor palette
or to re-balance a base part by reusing its id (overlay by id; see
[Make and publish a mod](../guide-make-a-mod/)).

Every field below is copied from the shipped catalog
(`assets/base/sections/base.content.ron`) or the config struct that parses it
(`crates/nova_gameplay/src/sections/`). The loader uses strict RON, so a
misspelled or unknown field is a hard parse error, not a silent default. For the
RON gotchas (newtype double-parens, `Some(...)`, tagged enums) shared with
scenarios, see [RON scenario/mod format](../modding-ron/).

## The Section item

A section is a `Section((base: ..., kind: ...))` content item (a `SectionConfig`):

```ron
Section((
    base: (
        id: "basic_thruster_section",
        name: "Basic Thruster Section",
        description: "A basic thruster section for spaceships.",
        mass: 1.0,
        health: 70.0,
    ),
    kind: Thruster((
        magnitude: 1.0,
    )),
)),
```

- `base` is a `BaseSectionConfig`, shared by every section kind:
  - `id` - unique section id; the key a ship's `source: Prototype("<id>")`
    references, and the key a mod overlays to replace a base part.
  - `name`, `description` - display strings (editor palette, tooltips).
  - `mass` - contributes to the ship's total mass and center of mass.
  - `health` - the section's hit points before it is disabled/destroyed.
- `kind` selects the behavior and carries that kind's own config. It is one of
  `Hull` / `Thruster` / `Controller` / `Turret` / `Torpedo`, each documented
  below.

The render meshes and effect fields are all `Option`s that default to `None`
(the built-in prototype mesh), so omit them for the default look or write
`Some("<scheme>://path")` for a custom asset. An asset ref is a SCHEMED path
string - `self://models/my.glb#Scene0` for your own model, or
`dep://base/gltf/hull-01.glb#Scene0` to reuse a base-game mesh; never bare.

## Hull

`HullSectionConfig` - passive armor. One optional field:

```ron
Section((
    base: (
        id: "reinforced_hull_section",
        name: "Reinforced Hull Section",
        description: "A reinforced hull section for spaceships.",
        mass: 1.0,
        health: 200.0,
    ),
    kind: Hull((
        render_mesh: Some("dep://base/gltf/hull-01.glb#Scene0"),
    )),
)),
```

- `render_mesh` (optional) - the hull mesh; omit for a default 1x1x1 cuboid.
- every section's `base` block also takes `impact_sound` + `destroy_sound`
  (optional) - the sounds a hit on / the destruction of THIS section plays,
  asset refs like the meshes (`dep://base/sounds/impact.wav` /
  `dep://base/sounds/explosion.wav` are the base voices); an omitted sound is
  silent. Per-target = per-material: your reinforced hull can clang.

## Thruster

`ThrusterSectionConfig` - forward thrust.

```ron
kind: Thruster((
    magnitude: 1.0,
)),
```

- `magnitude` - the thrust force this section produces at full throttle.
- `render_mesh` (optional) - custom mesh; omit for the default thruster body.
- `loop_sound` (optional) - the engine hum this thruster contributes to
  (`dep://base/sounds/thruster_loop.wav` is the base drone); thrusters sharing
  a sound share one loop whose volume tracks the loudest ship burning it. An
  omitted sound hums nothing.

## Controller

`ControllerSectionConfig` - the steering PD controller; a ship needs a live one
to fly.

```ron
kind: Controller((
    frequency: 4.0,
    damping_ratio: 4.0,
    max_torque: 40.0,
)),
```

- `frequency` - the PD controller frequency in Hz (how stiffly it chases the
  commanded heading).
- `damping_ratio` - the PD damping ratio (overshoot vs settle).
- `max_torque` - the maximum torque the controller may apply.
- `render_mesh` (optional) - custom mesh; omit for the default body.
- `lock_on_sound`, `lock_off_sound`, `radar_deny_sound`,
  `radar_retarget_sound`, `safety_on_sound` (all optional) - the computer's
  radar/lock and weapons-safety feedback ticks, asset refs like the meshes
  (`dep://base/sounds/lock_on.wav` etc. for the base cues); an omitted cue is
  silent. Your ship's computer can have its own voice.

## Turret

`TurretSectionConfig` - an articulated gun that aims with intercept lead and
fires bullets. The most field-heavy kind; the shipped `better_turret_section`
is the reference:

```ron
kind: Turret((
    yaw_speed: 3.1415927,
    pitch_speed: 3.1415927,
    min_pitch: Some(-0.5235988),
    max_pitch: Some(1.5707964),
    base_offset: (0.0, -0.5, 0.0),
    render_mesh_yaw: Some("dep://base/gltf/turret-yaw-01.glb#Scene0"),
    yaw_offset: (0.0, 0.1, 0.0),
    render_mesh_pitch: Some("dep://base/gltf/turret-pitch-01.glb#Scene0"),
    pitch_offset: (0.0, 0.332706, 0.303954),
    render_mesh_barrel: Some("dep://base/gltf/turret-barrel-01.glb#Scene0"),
    barrel_offset: (0.0, 0.128437, -0.110729),
    muzzle_offset: (0.0, 0.0, -1.2),
    fire_rate: 100.0,
    muzzle_speed: 100.0,
    projectile_lifetime: 5.0,
    bullet_damage: 4.0,
    bullet_kind: Kinetic,
    fire_sound: Some("dep://base/sounds/turret_fire.wav"),
    ammo_capacity: Some(500),
)),
```

- `yaw_speed`, `pitch_speed` - traverse speeds in radians per second.
- `min_pitch`, `max_pitch` (optional) - pitch limits in radians; `None` for no
  limit.
- `base_offset`, `yaw_offset`, `pitch_offset`, `barrel_offset`,
  `muzzle_offset` - `Vec3` mount offsets stacking base -> yaw -> pitch -> barrel
  -> muzzle (bare 3-tuples).
- `render_mesh_base`, `render_mesh_yaw`, `render_mesh_pitch`,
  `render_mesh_barrel` (all optional) - the per-part meshes; omit for defaults.
- `fire_sound` (optional) - the sound each fired round plays, an asset ref like
  the meshes (`self://` a wav your mod ships, or `dep://base/sounds/
  turret_fire.wav` for the base cue); omit and the turret fires SILENTLY (the
  base turrets author it, so copy their line if you want the stock sound). Your
  turret can sound like its own gun.
- `dry_fire_sound` (optional) - the click when the trigger is pulled on an
  empty magazine; same asset-ref rules (`dep://base/sounds/dry_fire.wav` is the
  base click), omit for a silent dry pull.
- `fire_rate` - rounds per second.
- `muzzle_speed` - projectile launch speed in units per second.
- `projectile_lifetime` - projectile lifetime in seconds.
- `bullet_damage` - authored per-hit damage (pre-resistance).
- `bullet_kind` - the damage type of the loaded round (`Kinetic`, and the other
  `DamageType` variants).
- `ammo_capacity` (optional) - magazine size; `None` fires without a limit,
  `Some(n)` gives an ammo slot of `n` rounds.
- `reload` (optional) - auto-reload for the magazine (needs `ammo_capacity`).
  `Some((reload_time, rounds_per_cycle, only_when_empty))`: a completed
  `reload_time` cycle restores `rounds_per_cycle` rounds (clamped to capacity).
  `only_when_empty: true` with `rounds_per_cycle` = capacity is discrete
  reload-on-empty; `only_when_empty: false` with `rounds_per_cycle: 1` is
  continuous per-round regen. `None` = a spent magazine stays empty.

## Torpedo

`TorpedoSectionConfig` - a bay that launches guided, proportional-navigation
torpedoes dealing blast damage. The shipped `torpedo_section`:

```ron
kind: Torpedo((
    render_mesh: Some("dep://base/gltf/torpedo-bay-01.glb#Scene0"),
    spawn_offset: (0.0, 0.0, -2.0),
    spawn_rotation: (0.0, 0.0, 0.0, 1.0),
    fire_rate: 1.0,
    spawner_speed: 1.0,
    projectile_lifetime: 100.0,
    arm_time: 0.5,
    arm_distance: 5.0,
    nav_constant: 3.0,
    max_speed: 35.0,
    linear_damping: 0.8,
    blast_radius: 30.0,
    blast_damage: 100.0,
    ammo_capacity: Some(6),
)),
```

- `launch_sound` (optional) - the sound a departing torpedo plays
  (`dep://base/sounds/torpedo_launch.wav` is the base whoosh); omit for a
  silent launch.
- `render_mesh`, `projectile_render_mesh` (both optional) - the bay mesh and the
  torpedo mesh; omit for defaults.
- `spawn_offset` (`Vec3`), `spawn_rotation` (`Quat`, a bare 4-tuple) - where the
  torpedo leaves the bay, relative to the section.
- `fire_rate` - launches per second.
- `spawner_speed` - launch speed in units per second.
- `projectile_lifetime` - torpedo lifetime in seconds.
- `arm_time`, `arm_distance` - the torpedo may detonate only after this many
  seconds OR this distance from the muzzle (arms on whichever comes first), so
  it clears the firing ship.
- `nav_constant` - the proportional-navigation constant `N` (typically 3-5;
  higher leads a moving target harder).
- `max_speed` - cruise speed cap in units per second.
- `linear_damping` - drag on the torpedo body (gives a real terminal velocity so
  the flight path follows guidance).
- `blast_radius`, `blast_damage` - detonation radius and peak centre damage
  (falls off to zero at the radius).
- `blast_effect`, `launch_effect` (both optional) - custom particle effects;
  omit for the built-in bursts.
- `ammo_capacity` (optional) - magazine size in torpedoes; `None` for unlimited.
- `reload` (optional) - auto-reload for the bay (needs `ammo_capacity`); same
  `Some((reload_time, rounds_per_cycle, only_when_empty))` shape as the turret.
  The shipped bay uses continuous regen (one torpedo every few seconds).

## A section in a mod

The example mod (`assets/mods/example/example.content.ron`) overlays the base
`reinforced_hull_section` - same id, so it REPLACES the base part everywhere
(editor palette, ships) with more health and a renamed label:

```ron
Section((
    base: (
        id: "reinforced_hull_section",
        name: "Reinforced Hull Section (Example Mod)",
        description: "Base hull, up-armored by the example mod to show section overlay by id.",
        mass: 1.0,
        health: 400.0,
    ),
    kind: Hull((
        render_mesh: Some("dep://base/gltf/hull-01.glb#Scene0"),
    )),
)),
```

Reuse a base id to REBALANCE or re-skin that part; give a NEW id to ADD a part
alongside the base catalog. Either way, a ship references the section by id via
`source: Prototype("<id>")` in its `sections` list. To ship the section, package
the file as a mod - [Make and publish a mod](../guide-make-a-mod/) is the full
flow.
