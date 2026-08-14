# Ship sections for mods

A `Section` is a reusable ship part defined in a mod's `*.content.ron` file.
Create a new id to add a part to the editor palette, or reuse a base id to
replace that part everywhere. The five available kinds are `Hull`, `Thruster`,
`Controller`, `Turret`, and `Torpedo`.

Start with the two working section items in
`assets/mods/example/example.content.ron`: one replaces
`reinforced_hull_section`, and one adds `example_plated_hull_section`. Use the
[base content catalog](../base-content/#section-prototypes) to find prototype
ids and reusable `dep://base/` assets. See [Mod files](../mod-files/) for the
bundle structure and [Publish a mod](../publish-a-mod/) when the part is ready
for release.

This page is the field-by-field section reference. For general RON spelling
rules such as double parentheses, `Some(...)`, and asset schemes, see
[RON spelling rules](../reference/#how-ron-content-is-written).

## The Section item

A content file is a list. Each section is one `Section((...))` item with a
shared `base` block and one kind-specific block:

```ron
[
    Section((
        base: (
            id: "my_mod_thruster",
            name: "Compact Thruster",
            description: "A light engine for small ships.",
            mass: 0.8,
            health: 70.0,
        ),
        kind: Thruster((
            magnitude: 1.0,
        )),
    )),
]
```

### Common fields

| field | type | default | meaning |
|---|---|---|---|
| `base.id` | string | required | Prototype key used by `source: Prototype("<id>")`. A new id adds a part; a matching id replaces the earlier part. Prefix new ids with your mod id. |
| `base.name` | string | required | Display name in the editor palette and ship UI. |
| `base.description` | string | required | Editor and tooltip description. |
| `base.mass` | number | required | Density, not absolute mass. Real mass is density multiplied by collider volume. |
| `base.health` | number | required | Hit points before the section is destroyed. |
| `base.impact_sound` | `Option` asset ref | `None` | Sound played when this section is hit. Omitted means silent. |
| `base.destroy_sound` | `Option` asset ref | `None` | Sound played when this section is destroyed. Omitted means silent. |
| `base.collider` | `Option` collider | `None` | Physics shape. Omitted means a 1 x 1 x 1 cube. |
| `base.link_points` | link-point list | `[]` | Structural sockets. Multi-section ships must derive one connected graph from their mates. |
| `base.hide_in_editor` | bool | `false` | `true` hides the prototype from the editor palette. Ships can still reference it. |
| `kind` | section kind | required | `Hull((...))`, `Thruster((...))`, `Controller((...))`, `Turret((...))`, or `Torpedo((...))`. |

Collider forms:

```ron
collider: Some(Cuboid(size: (1.0, 0.5, 1.5))),
collider: Some(Sphere(radius: 0.5)),
collider: Some(Capsule(radius: 0.4, length: 1.0)),
collider: Some(Cylinder(radius: 0.5, height: 1.0)),
```

`Cuboid.size` is the full size on each axis. Capsules and cylinders extend
along local Y. A larger collider also increases real mass because `base.mass`
is density.

### Structural link points

Structural attachment is explicit and independent of the collider. Two points
mate when their transformed positions coincide and their outward unit normals
oppose. The point `id` is unique within its section and is used for diagnostics
and UI; IDs do not have to match.

Author normals on an AXIS, and positions at face centres, unless the part really
needs otherwise. A socket's roll zero is derived from its normal, so two parts
that were never drawn together still mate square; an off-axis normal instead
tilts everything mated onto it by exactly its own angle. Size is free: a part
authored at its own size mates one of any other size - a 0.3 mount sits on a 1.0
hull face - so a small mount does not have to pretend to be a cube.

A one-unit cube normally authors six face-center points. Example pair:

```ron
link_points: [
    (
        id: "positive_x",
        position: (0.5, 0.0, 0.0),
        normal: (1.0, 0.0, 0.0),
    ),
    (
        id: "negative_x",
        position: (-0.5, 0.0, 0.0),
        normal: (-1.0, 0.0, 0.0),
    ),
    // Add positive_y, negative_y, positive_z, and negative_z as needed.
],
```

Rules:

- Positions and normals are section-local.
- Normals must be finite, nonzero, and unit length.
- One point cannot mate with multiple points.
- Unused exterior points are valid.
- A multi-section ship must form one connected mate graph.
- Omitted `link_points` means no sockets. Collider contact and one-unit spacing
  do not provide compatibility behavior.
- A section override replaces the complete prototype. Repeat its link points or
  author new ones; it does not inherit sockets from the replaced section.

Error-level graph findings prevent the scenario from starting.

Link points are also how a part is PLACED. The editor mates the socket nearest
the pointer with one of the placed part's own, so a prototype with no sockets
can be spawned by a ship but never built by hand, and a part with a single
socket only attaches that way round. The builder chooses which of the part's
sockets does the mating and how far it is rolled about the mating axis;
everything else follows from the pair. A placement is refused - and says why -
when the target socket is already mated, when it would leave a socket with two
suitors, or when the part would sit inside a section it does not mate with.

### Assets and visual transforms

Asset fields use namespaced strings:

- `self://models/my-part.glb#Scene0` - art listed in your own bundle.
- `dep://base/gltf/hull-01.glb#Scene0` - art from the base content catalog.

Never use a bare path. Most mesh, sound, and effect fields are optional. Omit
them for the built-in visual or silent audio behavior described below.

Hull, thruster, controller, and torpedo sections can move their render mesh
without moving the collider:

```ron
render_mesh_transform: Some((
    position: (0.0, 0.1, 0.0),
    rotation: (0.0, 0.0, 0.0, 1.0),
)),
```

Both transform fields default independently: omit `position` for zero and
`rotation` for identity. Turrets place this transform on each joint instead.

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
- `render_mesh_transform` (optional) - visual-only position and rotation; does
  not move the collider.
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

- `magnitude` - impulse applied at full throttle on each fixed simulation tick.
  Larger values accelerate the same ship faster; compare against the base
  prototypes when balancing.
- `render_mesh` (optional) - custom mesh; omit for the default thruster body.
- `render_mesh_transform` (optional) - visual-only position and rotation.
- `loop_sound` (optional) - the engine hum this thruster contributes to
  (`dep://base/sounds/thruster_loop.wav` is the base drone); thrusters sharing
  a sound share one loop whose volume tracks the loudest ship burning it. An
  omitted sound hums nothing.
- `exhaust` (optional) - custom flame placement and shape. `None` uses the
  standard cone. Write `Some((...))` when fitting exhaust to custom art.

`exhaust` fields:

| field | type | default | meaning |
|---|---|---|---|
| `offset` | 3-tuple | `(0.0, 0.0, 0.3)` | Flame origin relative to the section. |
| `rotation` | 4-tuple | standard rear-facing rotation | Rotates the flame, which is built along local +Y. |
| `shape.geometry` | `Cone` or `Rect` | `Cone` | Round or rectangular flame cross-section. |
| `shape.width`, `shape.height` | number | `0.8`, `0.8` | Full rectangular nozzle size; ignored by `Cone`. |
| `shape.exhaust_height`, `shape.exhaust_radius` | number | `0.1`, `0.4` | Outer flame length and base radius. |
| `shape.exhaust_max` | number | `1.0` | Outer flame peak intensity. |
| `shape.exhaust_inner_height`, `shape.exhaust_inner_radius` | number | `0.05`, `0.1` | Inner core length and radius. |
| `shape.exhaust_inner_max` | number | `0.5` | Inner core peak intensity. |
| `shape.emissive_color`, `shape.emissive_inner_color` | linear color | cyan, blue | Outer and inner glow colors. |

Every nested exhaust field has a default, so a mod can override only the values
it needs. Copy a semantic engine part from
`assets/base/sections/base.content.ron` for a complete rectangular example.

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
- `render_mesh_transform` (optional) - visual-only position and rotation.
- `lock_on_sound`, `lock_off_sound`, `radar_deny_sound`,
  `radar_retarget_sound`, `safety_on_sound`, `rcs_loop_sound` (all optional) - the computer's
  radar/lock and weapons-safety feedback ticks, asset refs like the meshes
  (`dep://base/sounds/lock_on.wav` etc. for the base cues); an omitted cue is
  silent. Your ship's computer can have its own voice.

## Turret

`TurretSectionConfig` - an articulated gun that aims with intercept lead and
fires bullets. The mount is an arbitrary tree of joints (`root`): each joint
sits at an `offset` from its parent, optionally rotates about an `axis` (a hinge
the aim solver drives), optionally carries a `render_mesh`, optionally is a
`muzzle` (a fire point), and hangs `children` joints off itself. Today's turret
is one specific tree - base(fixed) -> yaw(axis Y) -> pitch(axis X) ->
barrel(fixed) -> muzzle - but you can build twin barrels, extra hinges, or a
turret whose elevation lives two joints down. The shipped `better_turret_section`
is the reference:

```ron
kind: Turret((
    root: (
        offset: (0.0, -0.5, 0.0),                                     // base (fixed)
        children: [(
            offset: (0.0, 0.1, 0.0),
            axis: Some((0.0, 1.0, 0.0)),                              // yaw hinge (Y)
            render_mesh: Some("dep://base/gltf/turret-yaw-01.glb#Scene0"),
            children: [(
                offset: (0.0, 0.332706, 0.303954),
                axis: Some((1.0, 0.0, 0.0)),                         // pitch hinge (X)
                min: Some(-0.5235988), max: Some(1.5707964),          // pitch limits
                render_mesh: Some("dep://base/gltf/turret-pitch-01.glb#Scene0"),
                children: [(
                    offset: (0.0, 0.128437, -0.110729),               // barrel (fixed)
                    render_mesh: Some("dep://base/gltf/turret-barrel-01.glb#Scene0"),
                    children: [(
                        offset: (0.0, 0.0, -1.2),                     // muzzle (fixed)
                        muzzle: Some((fire_rate: 100.0)),
                    )],
                )],
            )],
        )],
    ),
    muzzle_speed: 100.0,
    projectile_lifetime: 5.0,
    bullet_damage: 4.0,
    bullet_kind: Kinetic,
    fire_sound: Some("dep://base/sounds/turret_fire.wav"),
    ammo_capacity: Some(500),
)),
```

Per-joint fields (on every `root`/`children` node):

- `offset` - `Vec3` local translation from the parent joint (the section origin
  for `root`), a bare 3-tuple. A joint's `children` are placed in its ROTATED
  frame, so they swing with it.
- `axis` (optional) - the local hinge axis (a bare 3-tuple like `(0.0, 1.0,
  0.0)`). Omit for a FIXED node (offsets and can still carry a mesh/muzzle, never
  rotates); set it to make the joint a hinge the aim solver steers. A muzzle's
  forward is its local `-Z`; the solver distributes the aim across every hinge
  above it.
- `speed` (optional) - traverse speed in radians per second; omit for the
  default 180 deg/s (PI). Only meaningful on a hinge (`axis` set).
- `min`, `max` (optional) - rotation limits in radians for this hinge; `None`
  for no limit.
- `render_mesh` (optional) - this joint's mesh; omit for a plain default
  primitive. Shipped turrets author a GLB per visible joint.
- `render_mesh_transform` (optional) - re-seats this joint's render mesh
  visually without moving the hinge or the collider.
- `muzzle` (optional) - marks this joint a fire point: `Some((fire_rate: N))`
  (rounds per second), plus an optional `muzzle_effect` flash asset ref. A turret
  aims and fires ALL of its muzzles: hang two off one barrel for a twin PDC, or
  give each its own arm. Every muzzle fires at its own `fire_rate` but draws from
  the ONE shared section magazine (`ammo_capacity`), so a twin barrel empties the
  same mag twice as fast rather than carrying a pool per gun.
- `children` (optional) - joints hanging off this one; omit for a leaf.

Section-wide fields (once, alongside `root`):

- `fire_sound` (optional) - the sound each fired round plays, an asset ref like
  the meshes (`self://` a wav your mod ships, or `dep://base/sounds/
  turret_fire.wav` for the base cue); omit and the turret fires SILENTLY (the
  base turrets author it, so copy their line if you want the stock sound). Your
  turret can sound like its own gun.
- `dry_fire_sound` (optional) - the click when the trigger is pulled on an
  empty magazine; same asset-ref rules (`dep://base/sounds/dry_fire.wav` is the
  base click), omit for a silent dry pull.
- `muzzle_speed` - projectile launch speed in units per second (shared by all
  muzzles; `fire_rate` is per-muzzle, see the joint fields above).
- `projectile_lifetime` - projectile lifetime in seconds.
- `bullet_damage` - authored per-hit damage (pre-resistance).
- `bullet_kind` - the damage type of the loaded round (`Kinetic`, `ArmorPiercing`,
  `Emp`, or `Explosive`).
- `projectile_render_mesh` (optional) - custom bullet mesh; omit for the
  built-in projectile.
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
- `detonation_sound` (optional) - the sound the warhead plays when it blasts
  (proximity or on impact); rides the torpedo's own destroy event, so it fires
  even when a torpedo is shot down. Omit for a silent detonation.
- `render_mesh`, `projectile_render_mesh` (both optional) - the bay mesh and the
  torpedo mesh; omit for defaults.
- `render_mesh_transform` (optional) - visual-only bay mesh position and
  rotation. It does not move the launch point.
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
- `projectile_health` (optional, default `1.0`) - hit points on each of the
  torpedo's two collider sections; either reaching zero shoots it down
  (silently, no blast). The default keeps ordnance one-bullet fragile; author
  more for armored ordnance point defense has to chew through.
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
the file as a mod - [Publish a mod](../publish-a-mod/) is the release flow.

Base ships a whole catalog of semantic ship-part prototypes. The mainline
corvette, gunship, and civilian hulls use `cargoa_*`, `cargob_*`, and
`racer_*` parts. A mod
does not have to inline a big ship or
carry any mesh paths: build one as a compact list of
`(id, position, rotation, source: Prototype("<base-part-id>"))` entries, and each
prototype resolves the base's meshes and sounds for you. Vary a ship by grade by
swapping which prototypes it references (for example a weaker `_light` turret variant
for an enemy) rather than re-authoring the parts. `SectionSource` is `Inline`
(the full config, for a one-off part) or `Prototype` (a catalog reference, the
compact reusable form). Every prototype id base ships - with its kind, so you
can tell structure from a gun - is tabled in the
[base content catalog](../base-content/).
