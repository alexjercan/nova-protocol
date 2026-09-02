# Length inventory and classification

The reasoning behind the meters migration: every value whose dimension contains
length, traced from where it is written to where it is spent, and the rule that
decides which side of the engine boundary it lands on.

## The four classes

| Class | Meaning | What happens to it |
| --- | --- | --- |
| **SERIALIZED** | A field a creator writes into a `.content.ron`, a scenario, or a mod. | Becomes `Meters` / `MetersPerSecond` / `MetersPerSecondSquared` / `Meters3`. Authored numbers become ten times what they were. |
| **DOMAIN** | A named constant, resource field, or public API a designer reasons about in the same terms a creator does - an engagement range, a speed cap, a standoff. | Becomes the same typed quantity. Converted at the point it meets the engine. |
| **ENGINE** | A value that only exists because Bevy, avian, a shader, a mesh, or the build grid needs it in world units. | Stays an `f32` in world units. Its module says so. |
| **NON-DISTANCE** | A count, ratio, normalized value, angle, time, pixel, colour, grid index, or hit-point figure that merely reads like a distance. | Untouched. |

## The rule

A value is SI (typed) when it is **authored, or is the default, cap, or direct
comparand of something authored**. It stays ENGINE when it exists only to feed
a Bevy transform or mesh, an avian collider, body, cast or impulse, a shader or
light, an audio emitter, or a build-grid cell.

The boundary is crossed with `Meters::to_engine()` / `Meters::from_engine()`
(and the matching methods on the other quantities) and nowhere else. There is
no other multiplication or division by ten anywhere in the repository.

## Why some length-bearing values deliberately stay ENGINE

Three families of value contain length but are not distances a creator or a
player ever reads, and converting them would change physics for no gain:

1. **Higher powers of length.** A gravitational parameter `mu` is `L^3/T^2`, so
   an SI `mu` would be a thousand times its world-unit value; carve pricing
   (`DAMAGE_PER_UNIT_VOLUME`, hit points per cubic unit) is `L^-3`; the thrust
   planner's `LATERAL_PENALTY` is a lever arm SQUARED; `MIN_IMPACT_SPEED_SQUARED`
   is `L^2/T^2`. Typing these needs a general dimensional-analysis library,
   which the task puts out of scope. They stay in engine units beside the
   physics that consumes them, and each says so.
2. **Values calibrated against an engine constant.** rodio clamps spatial
   attenuation at a distance of exactly 1, so `SPATIAL_EAR_GAP` and
   `SPATIAL_EMITTER_RADIUS` are tuned to that clamp, not to a room. Mesh UVs are
   an unnormalized world-length projection, so texture density is a length
   coefficient. Bevy's `PointLight::range` and `Collider::*` take engine units.
3. **The build grid.** A cell is one world unit by decision; a section mount
   position is a cell index, not a distance. Its physical side is 10 m, and that
   is what the documentation says.

`AsteroidConfig::mass` and `AnchorConfig::mass` are the one authored field that
stays in engine units. They are a well STRENGTH stat, not an SI mass: the
gravitational parameter above. The radius beside them is a real length and does
migrate, so the physics is unchanged and a creator still reads a rock's size in
meters. The published figures derived from the pair - surface gravity in m/s^2,
sphere of influence in km - are SI, and the docs quote those.

---

## Per-subsystem inventory

### `crates/nova_events`

| Site | Class | Action |
| --- | --- | --- |
| `scale::METERS_PER_UNIT` (10.0) | the boundary itself | Moved to `units`, beside the quantity types that carry it. |
| `scale::LOAD_LIMIT` (8 G, already SI) | DOMAIN | Typed `MetersPerSecondSquared`. |
| everything else (events, engine) | NON-DISTANCE | - |

New: `units::{Meters, MetersPerSecond, MetersPerSecondSquared, Meters3}` and
`METERS_PER_UNIT`, exported through the crate prelude. `nova_events` is the
deepest crate both the physics side (`nova_ship`) and the display side
(`nova_ui`) already reach, so nothing gains a dependency edge.

### `crates/nova_ship/src/sections` - the section content format

Every distance and speed on an authored section config is SERIALIZED and
becomes typed. The conversion happens where the config is snapshotted onto the
runtime entity: a spawner's ejection speed, a blast collider's radius, a
muzzle's offset. Those sites are the section's engine boundary and each carries
a note.

Stays ENGINE: collider shapes and half-extents (avian takes world units), mesh
primitives and render-mesh transforms, link-point sockets and section
footprints (build-grid cells), the placeholder-art geometry, damage-crack and
plume sizes, and the shell/skin geometry that plates a cell.

### `crates/nova_ship` - flight, physics, camera, AI, targeting

The authored side is typed: a scenario's ship overrides (`speed_cap`,
`arrival_standoff`, `engage_range`, `leash` and its centre, `pd_range`,
`waypoint_slack`) and every waypoint arrive as quantities and cross once, in the
spawner, where they land on a runtime component.

Everything downstream of that spawn stays ENGINE, and this is the one place the
draft inventory guessed wrong. `FlightSettings` (`arrival_standoff`,
`stop_speed_epsilon`, `min_approach_speed`, `rcs_speed_cap`, `rcs_accel`), the
AI envelope constants (`AI_ENGAGE_RANGE`, `AI_POINT_DEFENSE_RANGE`,
`AI_WAYPOINT_SLACK`, `AI_STANDOFF_RANGE` and the ranges beside them), the
targeting settings that decide lock range, and the runtime components
(`FlightArrivalStandoff`, `FlightSpeedCap`, `AIEngageRange`, `AILeash`, ...) all
keep world units. They are the DEFAULTS AND COMPARANDS of avian positions and
velocities, read every tick; typing them would put a conversion in the hot loop
instead of at the spawn that authored the override, and would not make a single
authored file any clearer. Each states its unit and its metric figure.

`ItemHighlight::world_radius` stays engine for the same reason: the HUD sizes
its bracket from it in camera space, never as a distance a player reads.

Stays ENGINE and documented: `structural_arm` (measured off colliders),
`AttitudeEnvelope` (its one conversion into SI is the fulcrum of the attitude
model), the thrust planner (`LATERAL_PENALTY` carries a squared lever arm), the
camera rig offsets, and every avian impulse and torque.

Four doc comments claimed metres for a world-unit value
(`input/ai/passive.rs`, `input/ai/acquisition.rs`, `input/ai/torpedo.rs`,
`input/targeting/state.rs`). They were wrong before this change and are fixed
by it.

### `crates/nova_gameplay`

No serialized length exists here. Typed: the published blast radius, the
carve/rake radii that content authors reach through a section config, and the
objective highlight radius. Everything else is engine-side by construction -
gravity integration, carve and chunk geometry, the round sweep, spark and
shard particles, camera shake, transient lights, spatial audio - and each
module states its unit where it was not already stated.

### `crates/nova_scenario` - the scenario document format

SERIALIZED and typed: object positions (`Meters3`), anchor and asteroid radii,
beacon radius and area radius, salvage size and area radius, light range and
radius, scatter region bounds and minimum separation, trigger-area radius and
position, camera position and look-at, the ship overrides (speed cap, arrival
standoff, engage range, leash radius and centre, point-defense range, waypoint
slack) and every waypoint.

NOT migrated, by the grid decision: `SpaceshipSectionConfig::position`, which is
a build-grid cell index.

The spawners convert once, at the point the config becomes a `Transform`, an
avian body or a runtime component.

### `crates/nova_authoring` - the base content and its generated RON

Every authored distance literal in `base_content/**` becomes ten times its old
value, and `content gen` rewrites `assets/base/**/*.content.ron` accordingly.
The balance audit compares authored reach against the AI envelope, so its
constants move with them.

### The editor

`FieldSpec::scale` existed only to show meters over a world-unit file. With the
file in meters it is identity, so the mechanism goes; `metered()` became
identical to `floored()` and was deleted with it. Step values grow by ten to
keep the same drag feel. The nested vector rows (`position`, `min`, `max`,
`look_at`) that had no spec, and so drew raw world units next to a metered pose
row, get one, and the panel's one engine seam is `pose_rows`, which reads a
live `Transform` through `Meters3::from_engine`.

Three rows do NOT say meters. `width`, and the `radius`/`height` a thruster's
exhaust cone carries, are mesh geometry the section builds inside its own
build-grid cell: a 0.8 there is 8 m of a 10 m cell, and the number a builder
types is the fraction of the cell they want lit. Those rows say `cells`. The one
genuinely metric `*radius` in the same family, a railgun's `rake_radius`, gets
its own metered spec so it keeps its floor and its unit. A behavior test pins
the distinction.

### The player-facing layer

`nova_ui::units` stops converting: it takes a `Meters` or a `MetersPerSecond`
and formats it. The multiplication that made a readout ten times its stored
value now happens at the one place a world-space distance is READ - the HUD's
`Meters::from_engine(a.distance(b))` - so nothing is converted twice. The
console's `speed_cap` command stops dividing the number a player types.

### Web, book and widgets

Creator pages stop asking for the conversion; the glossary keeps the world unit
only as the build-grid cell's engine name. Every quoted figure is re-derived
from the migrated code. The widget scopes that model an AUTHORED quantity
(blast, reach, torpedo run, ignition, corridor) work in meters end to end; the
scopes that model an ENGINE one (gravity, the closing-speed damage curve, the
structural arm, thruster impulse) keep world units and cross once, in a named
`engine*` formatter.

### Cameras and lighting

`ThreePointRig::scale` is a DIMENSIONLESS framing multiplier, not a distance:
the rig's offset table carries the lengths. The table moved to meters and the
scale arguments were left exactly as they were. `ScriptedCameraPose` converts
where it writes the `Transform`.
