# Ship sections for mods

A `Section` is a reusable ship part defined in a mod's `*.content.ron` file.
Create a new id to add a part to the editor palette, or reuse a base id to
replace that part everywhere. The six available kinds are `Hull`, `Thruster`,
`Controller`, `Turret`, `Torpedo`, and `Railgun`.

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

**Units.** Every reach, blast and speed on this page is authored in meters and
meters per second - `blast_radius: 300.0` is 300 m, and there is no conversion
to do. A section's own GEOMETRY is the exception, because a section is built on
a grid: its `collider`, its `link_points`, a turret joint's `offset`, a bay's
`spawn_offset` and `spawn_recess`, a railgun's `muzzle_offset` and a thruster's
exhaust cone are all counted in BUILD CELLS, and one cell is 10 m on a side.
Each field below says which it is.

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
| `base.health` | number | required | Hit points before the section is destroyed. |
| `base.material` | `Option` string | `None` | What the section is MADE of, looked up in the [impact table](../impacts/) against the round that hit it. Omitted means `"hull"`. |
| `base.destroy_sound` | `Option` asset ref | `None` | Sound played when this section is destroyed. Omitted means silent. |
| `base.collider` | `Option` collider | `None` | Physics shape. Omitted means a 1 x 1 x 1 cube. |
| `base.link_points` | link-point list | `[]` | Structural sockets. Multi-section ships must derive one connected graph from their mates. |
| `base.hide_in_editor` | bool | `false` | `true` hides the prototype from the editor palette. Ships can still reference it. |
| `base.damage_effects` | effect list | `([Cracks])` | The damage looks this section wears. Omitted means its surface cracks, which is what every section does unless it says otherwise. |
| `base.animations` | track list | `[]` | Art the section moves when a gameplay cue calls for it - a bay's iris, a turret's stow. Omitted means the section has no moving parts. |
| `kind` | section kind | required | `Hull((...))`, `Thruster((...))`, `Controller((...))`, `Turret((...))`, or `Torpedo((...))`. |

Collider forms:

```ron
collider: Some(Cuboid(size: (1.0, 0.5, 1.5))),
collider: Some(Sphere(radius: 0.5)),
collider: Some(Capsule(radius: 0.4, length: 1.0)),
collider: Some(Cylinder(radius: 0.5, height: 1.0)),
```

`Cuboid.size` is the full size on each axis. Capsules and cylinders extend
along local Y. The collider is also what the section WEIGHS: every section runs
at a density of 1, so its mass is the volume of this shape and a bigger box is
a heavier part. There is no density knob - a section is solid ship, and the
render mesh never counts.

An integral cuboid of at least one cell on every axis is also the section's cell
footprint. For example, `Cuboid(size: (3.0, 3.0, 2.0))` occupies 18 cells as one
machine. Skin and clearance read every occupied cell. Put mounting sockets at
the centres of the cells on each face that can attach; collider contact alone
never creates structure. Non-integral cuboids and non-cuboid shapes remain
one-cell footprints.

### Damage effects

What a section LOOKS like as it takes damage, authored as a list:

```ron
damage_effects: ([Cracks, Sparks, Plume]),
```

| effect | what it does |
|---|---|
| `Cracks` | The section's surface fractures where it is failing, glows through the cracks when it is critical, and burns out cold when it dies. |
| `Sparks` | The section throws sparks, faster the worse it is. |
| `Plume` | The section's exhaust guts and flickers. Thrusters only - it grades the exhaust cone, so a section with none shows nothing. |

`Cracks` grades in EIGHT steps from painted to burnt, so the first fractures
show once about a fourteenth of the section's own health is gone and every step
after that is a seventh. Stepping it is what keeps a fleet's damage off the
frame rate - a continuous value per section put every section mesh in a draw bin
of its own. `Sparks` and `Plume` both start once the section is properly hurt -
past 35 percent of its own health gone - so a scratch does not make a part read
as broken.

<!-- Numbers verified against crates/nova_ship/src/sections/damage_cracks.rs (SECTION_CRACK_BUCKETS 8 :90, crack_bucket rounds to nearest :98-102). -->


Omitting the field means `([Cracks])`. Author `([])` for a section that should
never show damage at all - saying "none" and saying nothing are different.

The rule the list is kept honest by: NO SECTION LOSES GEOMETRY. Every effect is
a material or a particle, and the only thing that changes a ship's shape is a
whole piece leaving - a cladding plate shot off, a section destroyed. Where a
section carries expendable material it wears cladding, and the cladding comes
off first (see [ships](../ships/)).

There was a `Carve` effect that cut a real crater out of a section's drawn mesh.
It is GONE, and mods authoring it will fail to load. It read well in a gallery
and badly in a fight: reading a solid out of authored art costs 6-15 ms per
mesh, a ship's marks belong to its root, and so a single round anywhere on a
hull turned every drawn mesh under it into a solid in one frame - measured at
325 meshes and 2.0 seconds. Asteroids still carve, because a rock's solid comes
from the same noise its mesh does and never has to be read out of art.

Effects compose freely and none of them changes what the section DOES: a
damaged thruster delivers exactly the thrust it authored.

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
    scale: (0.5, 0.5, 0.5),
)),
```

All three fields default independently: omit `position` for zero, `rotation`
for identity and `scale` for the size the art was modelled at. Turrets place
this transform on each joint instead.

`scale` resizes the ART, not the section. The collider, the link points and the
mass stay exactly as authored, so shrinking a mesh does not shrink what the part
occupies - set the collider and its sockets to match if you want them to agree.
Resizing a whole ASSEMBLY (a turret's joint tree) takes both halves: scale every
joint's mesh AND every joint offset by the same factor, or the parts stay spaced
for the size they used to be.

### Animation tracks

`base.animations` is what makes a section's art MOVE. It is a list of TRACKS,
and one track binds a gameplay CUE to a set of named nodes and says what their
travelled pose is:

```ron
animations: [
    (
        cue: MuzzleDoor,
        node_prefix: "door_petal_",
        motion: RotateX(degrees: 105.0),
        open_seconds: 0.25,
        close_seconds: 0.7,
    ),
],
```

That is the shipped torpedo bay, whole: the six iris petals modelled into
`bay_tube.glb` as `door_petal_0..5`, folding 105 degrees out of the muzzle in a
quarter second and closing again over 0.7.

The runtime holds one PROGRESS number per track, 0 at rest and 1 fully
travelled, and every frame it composes `motion` at that progress onto the pose
the node was modelled at. Content owns WHAT moves; the section kind's own
systems own WHEN, by steering the cue.

- `cue` - the gameplay moment that drives this track. The set is closed
  (`MuzzleDoor`, `StowLift`, `StowDoors`, `Charge` - see the table below); a
  mod picks from it rather than inventing one. Several tracks may share a cue, and a cue
  no system on this section steers simply rests at 0.
- `node_prefix` - which nodes move, by NAME PREFIX: `"door_petal_"` takes
  `door_petal_0` through `door_petal_5`. A prefix and not a list, so art can
  change the part count without the track being re-authored. The match runs
  over every named node under the section, which is why a turret joint given a
  `name` is steered by exactly the same track machinery as a node modelled
  inside a glb.
- `motion` - the pose at full progress, one of two:
  - `RotateX(degrees: N)` turns each node about its own LOCAL X axis. Local X
    is the hinge convention: the part is modelled with its origin ON the hinge
    line and X along it, and its placement transform aims the hinge. That is
    how ONE track swings the bay's six petals on six different hinges.
  - `Translate(offset: (x, y, z))` slides each node by `offset`, measured in
    that node's own rest frame. Same one-track-many-nodes rule: the PDC's two
    housing lids are modelled mirror-rotated, so one signed travel closes both
    toward each other.
- `open_seconds` - seconds progress takes to run 0 -> 1.
- `close_seconds` - seconds progress takes to run 1 -> 0. Either value at zero
  or below SNAPS that direction instead of travelling it.

Which direction reads as "opening" is yours to choose, because progress 1 is
whatever the track says it is. The bay's 1 is an open iris, so it opens in
`open_seconds`. The PDC's 1 is a STOWED gun, so `close_seconds` is the number
that matters in a fight: it sinks lazily over 0.9 s and comes back up in 0.35.

Who raises each cue:

| cue | steered by | progress 1 is |
|---|---|---|
| `MuzzleDoor` | The torpedo bay's fire path, on the HELD trigger - and only while the bay could genuinely fire, so weapons safety or an empty magazine keeps the iris shut. A launched round holds it open across the cold coast, so a tapped trigger closes the doors behind the torpedo rather than on it. | open |
| `StowLift` | The turret's stow machine. | sunk into the housing |
| `StowDoors` | The turret's stow machine, sequenced against the lift: it shuts the lids only once the gun is fully down, and parts them before raising it. | shut over the sunk gun |
| `Charge` | The railgun's charge system, from the committed trigger to the shot. It writes the charge fraction straight in, so `open_seconds` and `close_seconds` are unread on this cue: the travel is the authored `charge_seconds`, and the snap back to 0 is the shot leaving. | fully charged, the instant before firing |

Authoring a `StowLift` track is what MAKES a turret retractable - the stow
machine is armed on turrets that have one and on no others. Such a turret
spawns stowed, deploys when its ship goes weapons hot, tracks a body, or is
assigned to point defense, and folds away again after four quiet seconds. It
cannot track or fire until it is fully up, which is the cost the mount pays for
being a smaller target. A ship that manages no weapons safety of its own reads
as hot, so a bare test rig deploys at spawn and stays up; an editor preview
carries no stow machine at all and shows the deployed gun, which is the pose
the art is modelled at.

Tracks move ART. A section's collider, its link points and its mass stay where
they were authored whatever a track is doing, so a bay with a shut iris weighs
the same and occupies the same cells as an open one, and its launch point does
not budge. A track that moves a turret JOINT is the one case that carries
gameplay geometry with it - the joints above it swing down too, muzzles
included - which is exactly why a stowed mount is held off tracking and firing
until it is back up.

<!-- Grammar verified against crates/nova_ship/src/sections/section_animation.rs (cues :35-49, motions :55-97, fields :105-118, prefix match :267, travel + snap :303-321) and crates/nova_ship/src/sections/turret_section/stow.rs (armed on StowLift :101, live turrets only :82-92, spawns stowed :104-105, deploy gate :72-74, demand and unmanaged fail-open :169-172, settle 4.0 s :21, sequencing :190-231) and crates/nova_ship/src/sections/torpedo_section/bay.rs (held trigger :551-568). Values from assets/base/sections/base.content.ron (bay :2008-2018, PDC :1156-1183). -->

## Hull

`HullSectionConfig` - passive armor. One optional field:

```ron
Section((
    base: (
        id: "reinforced_hull_section",
        name: "Reinforced Hull Section",
        description: "A reinforced hull section for spaceships.",
        health: 200.0,
    ),
    kind: Hull((
        render_mesh: Some("dep://base/gltf/hull-01.glb#Scene0"),
    )),
)),
```

- `render_mesh` (optional) - the hull mesh; omit for a default 1x1x1 cuboid.
- `render_mesh_transform` (optional) - visual-only position, rotation and
  scale; does not move or resize the collider.
- every section's `base` block also takes `destroy_sound` (optional) - the
  sound THIS section's destruction plays, an asset ref like the meshes
  (`dep://base/sounds/explosion.wav` is the base voice); an omitted sound is
  silent.
- what a HIT sounds like is not authored here. A hit has two halves - the round
  and what it struck - so the section names only its `material` and the
  [impact table](../impacts/) pairs that with the damage type.

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
- `render_mesh_transform` (optional) - visual-only position, rotation and scale.
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

The cone is MESH geometry, sized in cells inside the section that carries it,
not a distance out in the world: `offset`, the nozzle size and both flame
lengths are all fractions of a 10 m cell. Every nested exhaust field has a
default, so a mod can override only the values it needs. Copy a semantic engine
part from `assets/base/sections/base.content.ron` for a complete rectangular
example.

## Controller

`ControllerSectionConfig` - the steering PD controller; a ship needs a live one
to fly.

```ron
kind: Controller((
    steering_lag: 0.5,
    max_torque: 1501.0,
)),
```

- `steering_lag` - approximate time in seconds that the hull trails a
  continuously moving steering command. Larger values make the computer feel
  laggier and brake earlier; smaller values track more tightly. The value must
  be positive and finite. Very small values are allowed but can make the
  fixed-step controller unstable. This is not a startup delay or total turn
  time: the computer reacts immediately, and the hull's turn ceiling still
  limits large turns.
- `max_torque` - how hard this computer's reaction wheels twist the hull, in
  torque units. Every shipped controller carries `1501.0`. Controllers on one
  hull ADD: two are twice the torque, with no cap and no stacking curve.
- `render_mesh` (optional) - custom mesh; omit for the default body.
- `render_mesh_transform` (optional) - visual-only position, rotation and scale.
- `lock_on_sound`, `lock_off_sound`, `radar_deny_sound`,
  `radar_retarget_sound`, `safety_on_sound`, `rcs_loop_sound` (all optional) - the computer's
  radar/lock and weapons-safety feedback ticks, asset refs like the meshes
  (`dep://base/sounds/lock_on.wav` etc. for the base cues); an omitted cue is
  silent. Your ship's computer can have its own voice.
- `warn_lock_sound` (optional) - the threat alarm, on the rising edge of a
  hostile taking a combat lock on you (`dep://base/sounds/warn_lock.wav` is the
  base tone). The COMPUTER owns it because knowing you are locked is a sensor
  capability: a hull flown without a controller gets no warning at all.
- `ammo_dry_sound` (optional) - a magazine running dry, heard at the GAUGE
  (`dep://base/sounds/ammo_dry.wav`). It fires on the same edge as the gun's own
  `dry_fire_sound` out on the mount and the two are meant to read as one event
  from two places - the gun's is per-turret, this one is per-SHIP.
- `warn_hull_sound` (optional) - the hull-critical alarm
  (`dep://base/sounds/warn_hull.wav`). ONE alarm, on the falling edge through
  `warn_hull_fraction`; the gravest thing the computer says.
- `warn_hull_fraction` (default `0.3`) - the fraction of the health the ship
  was BUILT with below which `warn_hull_sound` goes off. The same quantity
  `collapse_threshold` is priced in (default `0.05`), so the alarm sits well
  clear of the wreckage floor - which is the point of having it. A cheap
  civilian computer may warn late, and `0.0` never warns at all. Clamped to
  `0..=1`.

### What a hull does with the torque

A hull may mount several controllers, but they do NOT each steer it: the ship
derives ONE attitude loop and shares it out. The turn ceiling it derives is
never authored:

<!-- Numbers verified against crates/nova_ship/src/physics/attitude.rs (envelope :75-90, arm in meters :75, structural_arm measured in world units :149, sustained rate :111, vector load :124-130), crates/nova_events/src/scale.rs (LOAD_LIMIT 8 * 9.81 :17), crates/nova_ship/src/sections/controller_section.rs (the one arm conversion, Meters::from_engine :490, linear torque sum :385-388, STACK_PRECISION_LIMIT 1.5 :259, stack_curve :267-269, smallest steering_lag :379-383) and crates/nova_authoring/src/base_content/sections/standard.rs (steering_lag 0.5 :376, max_torque 1501.0 :384). -->

```text
ceiling = min( sum(max_torque) / I , 78.48 / r )   rad/s2
```

- `I` - the hull's largest principal moment of angular inertia, measured by the
  physics engine from the live section colliders and their densities. An engine
  number, and `max_torque` is one too: torque over inertia is a rate whatever
  scale the two are measured in, as long as it is the same scale.
- `r` - the structural arm in METERS: the distance from the hull's centre of
  mass to the outer FACE of its furthest live section. It is the one figure here
  the engine measures for itself - off the live colliders, in world units - and
  it crosses into meters once, where the envelope is built.
- `78.48` m/s2 is 8 G, the load hull metal takes. One global constant, the same
  for every ship and every mod.

So `max_torque` is the only handling number you author, and it binds only on a
hull heavy enough that its computers give up before its metal does. Everything
that ships is on the second term: the hull would tear first, so fitting more
computers buys it no turn rate at all. Size and shape set the rest. A long hull
has a long arm and a low ceiling; a short one is sharp. Author a bigger
`max_torque` for a capital-scale hull that reads sluggish, not for a small one -
a small one is already at its limit.

Three consequences to author around:

- **Damage sharpens a hull.** Losing sections shortens `r`, which raises the
  ceiling. A wreck turns harder than it did intact.
- **A hard turn spends the margin.** The turning load `alpha * r` and the
  centripetal load `omega^2 * r` add as a vector, and that sum is what must stay
  under 8 G. So a hull holds `sqrt(78.48 / r)` rad/s indefinitely and has no
  authority left to tighten past it.
- **Stacking buys precision, not authority.** A stack starts arresting a turn
  earlier and lands on the commanded attitude instead of swinging past. That
  gain approaches x1.5 from below: x1.25 at two computers, x1.375 at four,
  x1.45 at ten. A mixed stack takes its response from the smallest live
  `steering_lag`.

## Turret

`TurretSectionConfig` - an articulated gun that aims with intercept lead and
fires bullets. The mount is an arbitrary tree of joints (`root`): each joint
sits at an `offset` from its parent, optionally rotates about an `axis` (a hinge
the aim solver drives), optionally carries a `render_mesh`, optionally is a
`muzzle` (a fire point), and hangs `children` joints off itself. Today's turret
is one specific tree - housing(fixed) -> stow lift(fixed, named) -> yaw(axis Y)
-> pitch(axis X) -> barrel(fixed) -> muzzle - but you can build twin barrels,
extra hinges, or a turret whose elevation lives two joints down. The shipped
`pdc_kinetic_turret_section` is the reference. Its `base` block is half the
part - a 0.5 mount box with ONE socket on its underside is why the same gun
bolts to any hull face, and the two stow tracks are why it sinks out of sight
between fights:

```ron
base: (
    id: "pdc_kinetic_turret_section",
    health: 130.0,
    collider: Some(Cuboid(size: (0.5, 0.5, 0.5))),
    link_points: [(id: "base", position: (0.0, -0.25, 0.0), normal: (0.0, -1.0, 0.0))],
    damage_effects: ([Cracks, Sparks]),
    animations: [
        (
            cue: StowLift,
            node_prefix: "stow_lift",
            motion: Translate(offset: (0.0, -0.8, 0.0)),              // sink the column
            open_seconds: 0.9,
            close_seconds: 0.35,
        ),
        (
            cue: StowDoors,
            node_prefix: "stow_lid_",
            motion: Translate(offset: (-0.24, 0.0, 0.0)),             // slide both lids shut
            open_seconds: 0.5,
            close_seconds: 0.25,
        ),
    ],
    // name, description and sounds omitted
),
kind: Turret((
    root: (
        offset: (0.0, -0.25, 0.0),                                    // housing (fixed)
        render_mesh: Some("dep://base/gltf/pdc_housing.glb#Scene0"),
        children: [(
            offset: (0.0, 0.0, 0.0),                                  // stow elevator (fixed)
            name: Some("stow_lift"),
            render_mesh_transform: Some((position: (0.0, 0.33, 0.0), scale: (0.44, 0.44, 0.44))),
            children: [(
                offset: (0.0, 0.4, 0.0),
                axis: Some((0.0, 1.0, 0.0)),                          // yaw hinge (Y)
                render_mesh: Some("dep://base/gltf/pdc_gatling_yaw.glb#Scene0"),
                render_mesh_transform: Some((scale: (0.5, 0.5, 0.5))),
                children: [(
                    offset: (0.0, 0.2, 0.0),
                    axis: Some((1.0, 0.0, 0.0)),                      // pitch hinge (X)
                    min: Some(-0.17453294), max: Some(1.5707964),     // -10 deg to +90 deg
                    render_mesh: Some("dep://base/gltf/pdc_gatling_pitch.glb#Scene0"),
                    render_mesh_transform: Some((scale: (0.5, 0.5, 0.5))),
                    children: [(
                        offset: (0.0, 0.01, -0.05),                   // barrel (fixed)
                        render_mesh: Some("dep://base/gltf/pdc_gatling_barrel.glb#Scene0"),
                        render_mesh_transform: Some((scale: (0.5, 0.5, 0.5))),
                        children: [(
                            offset: (0.0, 0.0, -0.475),               // muzzle (fixed)
                            muzzle: Some((fire_rate: 100.0)),
                        )],
                    )],
                )],
            )],
        )],
    ),
    muzzle_speed: 1000.0,
    projectile_lifetime: 2.0,
    bullet_damage: 4.0,
    bullet_kind: Kinetic,
    fire_sound: Some("dep://base/sounds/turret_fire.wav"),
    dry_fire_sound: Some("dep://base/sounds/dry_fire.wav"),
    ammo_capacity: Some(500),
    reload: Some((delay: 3.0, amount: 200)),
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
- `render_mesh_transform` (optional) - re-seats or resizes this joint's render
  mesh visually without moving the hinge or the collider. It also sizes the
  DEFAULT primitive an unmeshed joint gets, which is a full unit across - scale
  it with the rest, or a small turret wears a hull-sized base plate. The PDC's
  elevator joint is that primitive put to work: no mesh of its own, shrunk and
  lifted into the disc the gun stands on.
- `name` (optional) - names this joint so a section
  [animation track](#animation-tracks) can steer it, exactly as a track matches
  a named node inside a scene mesh (the shipped PDC names its elevator joint
  `stow_lift`, and the `StowLift` track above targets that name). Omit it on a
  joint no track touches.
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
- `stow_open_sound`, `stow_close_sound` (both optional) - the lids over a
  retractable mount, played when the stow state machine commands the doors open
  and shut (`dep://base/sounds/pdc_stow_open.wav` and `pdc_stow_close.wav` are
  the base servos). Two files, because a housing lifting does not sound like one
  sinking. A mount that authors no `StowLift`
  [animation track](#animation-tracks) never stows, so these two mean nothing on
  a fixed gun; omit either for a silent half.
- `muzzle_speed` - projectile launch speed in METERS PER SECOND (shared by all
  muzzles; `fire_rate` is per-muzzle, see the joint fields above).
- `projectile_lifetime` - projectile lifetime in seconds. A turret has no
  range field: `muzzle_speed * projectile_lifetime` IS its reach, in meters (the
  stock PDC reaches 2 km). An AI ship holds fire past 90% of that and settles
  into a fight at ~1 km, so a gun authored with much less reach than that
  belongs on a player ship - an AI carrying it orbits outside its own range and
  never fires.
- `bullet_damage` - authored damage per hit, before the closing-speed curve. It
  is the SAME number against every section: there is no resistance table, so a
  damage type never multiplies what a round deals.
- `bullet_kind` - the damage type of the loaded round (`Kinetic`, `Pierce`, or
  `Explosive`). A type decides how the round TRAVELS, not how much it deals:
  - `Kinetic` is the punch. `bullet_damage` doubles as its budget: it continues
    only through what it DESTROYS, spending the target's health out of that
    budget, and stops at anything it fails to kill. Closing speed scales its
    damage (clamped 0.25x..2.0x).
  - `Pierce` is the rake. It deals `bullet_damage` in full to EVERY section it
    crosses, alive or dead, undiminished by depth, so its total legitimately
    exceeds one round's worth. It pays for travel out of a separate power
    budget: crossing a section costs that section's `health` RATING (not its
    remaining health), and closing speed divides that cost (clamped
    0.5x..3.0x). A rake is bounded by its power and by a hard six-layer cap.
  - `Explosive` on a bullet is spent on its first hit. A torpedo blast uses the
    Explosive pressure rule described below.

  Both curves read exactly 1.0 when the round closes at 1,000 m/s, a stock PDC's
  `muzzle_speed`, so author `bullet_damage` for a station-keeping engagement
  and speed does the rest. Nothing of any type gets through a collider with no
  health of its own - an asteroid or a planetoid is a wall.
- `projectile_render_mesh` (optional) - custom bullet mesh. Omit it and the
  round is drawn from the built-in set, which is keyed by the FIRED round's
  damage type rather than by the turret: a Kinetic slug is stubby, a Pierce
  round a long thin dart, an Explosive shell squat and wide, each in that
  type's HUD colour. Setting this field overrides all three - one mesh,
  whatever the turret has loaded.
- `ammo_capacity` (optional) - magazine size; `None` fires without a limit,
  `Some(n)` gives an ammo slot of `n` rounds.
- `reload` (optional) - idle batch reload for the magazine (needs
  `ammo_capacity`). `Some((delay, amount))`: every successful shot resets the
  timer; after `delay` quiet seconds, `amount` rounds return, clamped to
  capacity. Batches repeat while the weapon stays idle. An empty trigger pull
  does not reset the timer. `None` = a spent magazine stays empty.

## Torpedo

`TorpedoSectionConfig` - a bay that launches guided, proportional-navigation
torpedoes dealing blast damage. The shipped `torpedo_section`:

```ron
kind: Torpedo((
    render_mesh: Some("dep://base/gltf/bay_tube.glb#Scene0"),
    spawn_offset: (0.0, 0.0, -1.0),
    spawn_rotation: (-0.70710677, 0.0, 0.0, 0.70710677),
    spawn_recess: 1.0,
    fire_rate: 1.0,
    spawner_speed: 80.0,
    projectile_lifetime: 100.0,
    arm_time: 0.5,
    arm_distance: 50.0,
    ignition_delay: 0.6,
    nav_constant: 3.0,
    linear_damping: 0.8,
    blast_radius: 300.0,
    blast_damage: 750.0,
    torpedo_type: (
        name: "Serpent",
        tint: Srgba((red: 0.95, green: 0.45, blue: 0.1, alpha: 1.0)),
        max_speed: 320.0,
        weave_angle: 0.44,
        weave_rate: 1.4,
    ),
    ammo_capacity: Some(6),
    reload: Some((delay: 10.0, amount: 1)),
)),
```

- `launch_sound` (optional) - the sound a departing torpedo plays
  (`dep://base/sounds/torpedo_launch.wav` is the base whoosh); omit for a
  silent launch.
- `detonation_sound` (optional) - the sound the warhead plays when it blasts;
  rides the torpedo's own destroy event, so it fires even when a torpedo is shot
  down. Omit for a silent detonation.
- `door_sound` (optional) - the muzzle iris servo
  (`dep://base/sounds/bay_door.wav` is the base one), played on BOTH edges of
  its travel: one servo, two directions, one file. A bay that authors no
  `MuzzleDoor` [animation track](#animation-tracks) has no iris and stays silent
  whatever this says - the cue answers the door moving, and a doorless bay's
  door never moves.
- `render_mesh`, `projectile_render_mesh` (both optional) - the bay mesh and the
  torpedo mesh. Omit `projectile_render_mesh` and the warhead flies as the
  built-in coned body, nose along its direction of travel. The shipped tube
  carries the `door_petal_*` iris nodes its `MuzzleDoor`
  [animation track](#animation-tracks) folds open; a bay drawn with other art
  simply has no doors to move.
- `render_mesh_transform` (optional) - visual-only bay mesh position, rotation
  and scale. It does not move the launch point.
- `spawn_offset` (`Vec3`), `spawn_rotation` (`Quat`, a bare 4-tuple) - the
  MUZZLE point, relative to the section: where the torpedo crosses the hull
  line, and where the launch flash and sound play.
- `spawn_recess` (`f32`, default `0.0`) - how far back along the launch axis
  the torpedo is BORN. The shipped bay recesses a full cell, so the round
  starts inside the tube and slides its whole length out past the muzzle.
  `0.0` births it at the muzzle point itself, and any depth is safe: the cold
  coast means the torpedo has no colliders until its drive lights.
- `fire_rate` - launches per second.
- `spawner_speed` - the ejection charge, in meters per second. A torpedo is not
  fired, it is dropped: this is the cold kick that pushes it clear of the hull,
  not the speed it flies at. That comes from the drive, once it lights.
- `projectile_lifetime` - torpedo lifetime in seconds.
- `arm_time`, `arm_distance` - the torpedo may detonate only after this many
  seconds OR this many METERS from the muzzle (arms on whichever comes first),
  so it clears the firing ship.
- `ignition_delay` - seconds the torpedo coasts INERT before its drive lights.
  For that whole window it has no thrust, no guidance, no fuze and no colliders:
  it can neither be shot down nor touch the ship it is leaving. Size it against
  `spawner_speed` so the motor catches once the torpedo is clear of the hull.
  `0.0` lights it on the first tick, which is the old launch-under-power
  behavior.
- `nav_constant` - the proportional-navigation constant `N` (typically 3-5;
  higher leads a moving target harder).
- `linear_damping` - drag on the torpedo body (gives a real terminal velocity so
  the flight path follows guidance).
- `blast_radius`, `blast_damage` - damage radius in meters and peak centre
  pressure.
  `blast_radius` no longer decides WHERE a torpedo goes off against a locked
  body (see [the fuze](#the-fuze-is-not-a-bay-field)); it is the reach of the
  pressure and the band the weave fades over. The visible sphere is the damage radius; pressure falls
  linearly to zero at its edge, measured from each collider's world centre. Ship
  sections shield sections behind them along a centre ray. A surviving section
  stops pressure; a destroyed section transmits 65 percent. Existing holes
  transmit freely. This transmission is one global Explosive rule, not an
  authored bay field. Structural sections can shield cladding and fixtures
  behind them, but those non-structural targets do not consume penetration
  themselves. Health is charged PER COLLIDER, but a blast cuts one crater PER
  BODY: a warhead over forty sections of one hull is one crater, not forty.
- `blast_effect`, `launch_effect` (both optional) - custom particle effects;
  omit for the built-in bursts.
- `projectile_health` (optional, default `10.0`) - hit points on each of the
  torpedo's two collider sections; either reaching zero shoots it down
  (silently, no blast). The default sits above the hardest single round a stock
  PDC can land (4.0 authored x the 2.0 kinetic speed ceiling), so an intercept
  costs a short burst rather than one lucky tap; author far more for armored
  ordnance point defense has to chew through across the closing window.
- `torpedo_type` (optional, defaults to the Serpent) - **what the bay loads**, as
  opposed to the tube it loads into. A type is DATA, not an enum: base authors
  its three (the straight-running Lance, the weaving Serpent, and the crimson
  siege Breaker - a cruise of 700 m/s with a shallow 0.22 rad weave - that only
  the experimental, deliberately overpowered `heavy_torpedo_section` bay loads),
  and a mod authors its own by writing the
  same five fields:
  - `name` - the ordnance's player-facing name (`"Lance"`, `"Serpent"`,
    `"Breaker"`). It names
    the launched projectile, so a log line or a probe snapshot says WHICH torpedo
    is in the air.
  - `tint` - the warhead's colour in flight, as
    `Srgba((red: .., green: .., blue: .., alpha: 1.0))`. Two types a player is
    meant to tell apart want different colours: it is the only difference visible
    before the flight paths have diverged.
  - `max_speed` (m/s; `350.0` on the Lance, `320.0` on the Serpent) - cruise speed
    cap. The thruster tapers off as the torpedo approaches it, so it decides
    time to target and, with `projectile_lifetime`, how far the ordnance can
    reach. **This is where an evasive type pays for its weave**, and it has to
    be authored: see the note below.
  - `weave_angle` (rad; `0.44`, ~25 degrees, on the Serpent) - the terminal
    weave: how far off the guidance solution an armed torpedo corkscrews. It
    PERTURBS the solution rather than replacing it and fades linearly to nothing
    between three blast radii and HALF a blast radius of the target, so a weaving
    torpedo still arrives on the aim point. The band is measured off
    `blast_radius`, never off the fuze: the terminal sprint has to start out at
    point-defense range, not a unit from the hull. `0.0` flies the bare
    intercept, which is what the Lance authors.
  - `weave_rate` (rad/s; `1.4` on both shipped types) - how fast the weave spins
    about the guidance command. The lateral acceleration a defender's lead
    solution fails to predict scales with
    `max_speed * sin(weave_angle) * weave_rate`, while the helix radius
    (`max_speed * sin(weave_angle) / weave_rate`) shrinks with it - keep that
    radius comfortably inside the terminal band, `blast_radius * 0.5`, or the
    torpedo is still swinging when the weave is meant to be gone. Unread at zero
    amplitude.

  The ANGLE is the exchange: measured against one stock PDC across the shipped
  1,500 m point-defense envelope, a Serpent costs ~370 rounds to stop and is only
  killed ~400 m out, where a Lance costs ~120 and dies ~1,150 m out. The RATE is
  the picture, not the price - the intercept cost barely moves with it, while the
  visible swing runs 240 m at 0.7 rad/s down to 60 m at 2.2.

  A weave does NOT pay for itself, which is why the shipped Serpent authors a
  lower `max_speed`. A corkscrew is a longer path, but only by ~1.7% as the real
  body flies it, and thrust is capped on the ALONG-NOSE speed - so a torpedo
  holding its nose off its own velocity never reaches the taper band, keeps its
  engine lit, and settles FASTER than a straight one. Author a weave without
  dropping the cap and you get evasion for free. Do not try to fix it by capping
  total speed instead: a total-speed cap leaves the torpedo ballistic at cruise
  and unable to steer at all.
- `ammo_capacity` (optional) - magazine size in torpedoes; `None` for unlimited.
- `reload` (optional) - idle batch reload for the bay (needs `ammo_capacity`),
  with the same `Some((delay, amount))` shape as a turret. The shipped bay
  restores one torpedo after ten seconds without a launch. Another launch
  resets that timer. Ammunition is a rate limit, not a permanent budget, but a
  bay must win through its six-round salvo rather than by outwaiting one PDC.

### The fuze is not a bay field

Where a torpedo goes off is engine behaviour, not an authored number. An armed
torpedo that locked an ENTITY fuzes 30 m from the nearest point of any collider
linked to that body, or at the distance the torpedo covers in one frame,
whichever is larger. The margin clears the torpedo's own body and the
next physics step while keeping the pressure and crater on the target. Two other
cases:

- a torpedo holding a target POSITION but no entity - a scripted launch, or one
  whose target died in flight - fuzes at `blast_radius * 0.5`, which is the only
  sense "arrived" has at a bare point;
- a torpedo launched with NO lock never fuzes at all. It flies its
  `projectile_lifetime`, deals a contact ding and is deleted, and the bay still
  spends the round. That is what
  [`ForceTorpedoLaunch`](../actions/#forcetorpedolaunch) produces on a battery
  with no controller.

## Railgun

`RailgunSectionConfig` - a spinal railgun with no traverse of its own: the HULL
aims it, and tapping the trigger COMMITS the shot. The shipped
`railgun_lance_section`:

```ron
kind: Railgun((
    render_mesh: Some("dep://base/gltf/railgun_lance.glb#Scene0"),
    muzzle_offset: (0.0, 0.0, -1.5),
    charge_seconds: 1.5,
    slug_speed: 15000.0,
    slug_damage: 300.0,
    slug_power: 1800.0,
    slug_lifetime: 1.2,
    rake_radius: Some(10.0),
    recoil_impulse: 45.0,
    fire_sound: Some("dep://base/sounds/railgun_fire.wav"),
    charge_sound: Some("dep://base/sounds/railgun_charge.wav"),
    reload_sound: Some("dep://base/sounds/railgun_reload.wav"),
    ammo_capacity: Some(1),
    reload: Some((delay: 12.0, amount: 1)),
)),
```

- `render_mesh`, `render_mesh_transform` (both optional) - the railgun mesh and a
  visual-only transform for it. The transform never moves the collider and
  never moves the bore, so a mesh nudged for looks still fires down the
  authored line. Omit the mesh and the gun draws as a unit cuboid.
- `muzzle_offset` (`Vec3`) - the bore exit in the section's own frame: where the
  slug is born, where the flash and the shot play, and the point the recoil is
  applied AT. Author it on the muzzle face. The recoil lever arm is measured
  from this point, so a muzzle left at the section origin shoves a three-cell
  gun as if it were a one-cell one.
- `charge_seconds` - seconds from the trigger to the shot. It cannot be
  aborted: this is AIMING time, not a hold. The gun does not re-check the nose
  when the charge ends, so a target that slid off the line in that window is
  simply missed.
- `slug_speed`, `slug_lifetime` - muzzle speed in meters per second, and how
  long the slug lives. Their product is the reach: the shipped railgun is
  15,000 m/s for 1.2 s, so 18 km. With no layer cap on penetration, the
  lifetime is also what stops a miss travelling forever.
- `slug_damage` - Pierce damage dealt to EVERY layer the slug rakes. Flat: it
  is not scaled by closing speed and it does not decay with depth, so the tenth
  section in the line takes what the first did.
- `slug_power` - the pierce budget, spent in the MAX health of each section
  the shot takes. This is the ONLY bound; the layer count is deliberately
  unlimited, so a slug stops when it runs out of material to spend rather than
  at an arbitrary layer. A crossing costs that section's max health divided by
  the pierce speed curve, which a slug at 15,000 m/s pins at its 3.0 ceiling, so the
  shipped 1800 buys twenty-seven crossings of 200 hp reinforced hull.
- `rake_radius` (optional) - how wide a corridor the shot cuts, in meters. Omit
  it and the slug is a needle: it cuts exactly the column its bore crossed,
  which is what every railgun did before this field existed.
  Author it and a sphere of that radius TRAILS the tip, its front tangent to
  the slug, sweeping a cylinder out of everything the tip has already reached.
  Three rules make it a rake and not a blast:
  - **The tip has to hit first.** A body the narrow slug only passed near is
    never opened, however far inside the radius it sits, and each separate
    body needs its own direct hit. A widened near miss is still a miss.
  - **It never reaches ahead.** The sphere trails, so a section the slug has
    not arrived at yet cannot be damaged by the shot that is about to arrive.
    It keeps sweeping after the tip leaves the far side, which is what opens
    an exit as wide as the corridor instead of a bore-sized one.
  - **It spends the same budget.** Every section in the corridor takes the
    flat `slug_damage` once and pays out of the same `slug_power`. There is no
    second damage number, no falloff and no blast.
  Wider is not more. Against dense material the budget binds either way, so a
  big radius removes the same total and spends it sideways on the entry face
  instead of forward through the hull: the shipped 10 m bores three cells wide
  through a four-deep wall and out the back, while 40 m strips the front layer
  and stops one cell in. Pick the radius for the SHAPE you want, then let the
  power decide how much of it you get.
- `recoil_impulse` - impulse applied backwards along the bore at the muzzle
  point on the tick the slug leaves. Raw impulse with no `dt`, in the units a
  thruster's magnitude carries. Because it lands at the muzzle and not at the
  centre of mass, an off-axis railgun YAWS the ship every time it fires, which is
  the price a wing mount pays over a nose one.
- `fire_sound` (optional) - the shot, at the muzzle.
- `charge_sound` (optional) - the capacitor bank filling. A LOOP held for the
  whole charge, played back faster as the charge runs, so the gun sounds like
  it is arriving at something. A loop and not a one-shot because
  `charge_seconds` is a number a mod may set to anything, and a fixed-length
  file would either end early or be cut off.
- `reload_sound` (optional) - a shell going back into the breech, played when
  the magazine returns to capacity. For a one-shell railgun that IS the whole of
  its cadence: the reload is the silence, and this is the silence ending.
- `ammo_capacity` (optional) - shells carried; `None` for unlimited, the
  bare-rig default every weapon section shares.
- `reload` (optional) - the same `Some((delay, amount))` batch reload a turret
  and a bay use. For a one-shell magazine it IS the cadence: the shipped railgun
  is one shot every twelve quiet seconds, and the section's ammo gauge is the
  countdown.

The scope below is the walk the game runs, on the block the range measures
the gun with: drag `rake_radius` and read what a shot takes, what it spends and
where it stops. The other faders are the slug's power and the block, so a
mod's own gun and hull can be approximated - a 60 hp cell is light plating,
480 a vector drive.

<div class="widget" data-widget="lance-corridor">
<p>Against a wall of 200 hp cells five across, five tall and four deep - a cell is 10 m on a side - the shipped 10 m radius takes nine cells a layer: the bore column, its four face neighbours and its four diagonals, through all four layers and one cell out the back. That is 28 cells for 1867 of 1800 power, the last crossing landing because the budget was still above zero when it began. A radius of 40 m takes the same 28 as the whole entry face and three of the next layer, and stops there. A radius of 0 is the needle: four cells in a line.</p>
</div>

Author a [`Charge` animation track](#animation-tracks) to give the gun a tell.
The shipped railgun walks a `charge_bolt` node the length of the bore, so how far
the bolt has travelled is how much charge is left to run - readable by the
firing player on their own hull and by an enemy across the gap. Without the
track the gun still charges; it just charges invisibly.

<!-- Grammar verified against crates/nova_ship/src/sections/railgun_section/mod.rs (config :61-147 including rake_radius, commit :179-185, charge state :187-193) and firing.rs (Pierce :207, authored-or-narrow rake :215). Rake rules: crates/nova_gameplay/src/rounds.rs sweep_raking. Values from assets/base/sections/base.content.ron (:2258-2484). Radius comparison measured in examples/systems/system_railgun_lance.rs's stand bank. -->

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
prototype resolves the base's meshes and sounds for you. Vary a ship by grade
with a per-spawn `SetHealth` [modification](../ships/) on the parts you want
weaker - a scavenger flies the same `pdc_kinetic_turret_section` at 60 mount
health - rather than re-authoring the parts. `SectionSource` is `Inline`
(the full config, for a one-off part) or `Prototype` (a catalog reference, the
compact reusable form). Every prototype id base ships - with its kind, so you
can tell structure from a gun - is tabled in the
[base content catalog](../base-content/).
