# Spaceship sections and integrity

Ships are assembled from modular **sections**. Each section is a child entity of
the ship root with its own collider, mass, and health, and contributes one
behavior (structure, thrust, steering, guns). The **integrity** system tracks
how sections connect and handles damage, disabling, and cascading destruction.

## Sections (`nova_gameplay::sections`)

A section is a `SectionConfig { base: BaseSectionConfig, kind: SectionKind }`.
`BaseSectionConfig` is shared by all kinds: `id`, `name`, `description`, `mass`,
`health`.

`SectionKind` variants (one module per kind under `crates/nova_gameplay/src/sections/`):

| Kind         | What it does |
|--------------|--------------|
| `Hull`       | Passive structure/armor. Just a `render_mesh`. |
| `Thruster`   | Forward thrust (`magnitude`); drives the exhaust visual. |
| `Controller` | PD attitude controller (`frequency`, `damping_ratio`, `max_torque`). Also grants flight `verbs` (STOP/GOTO/ORBIT autopilot capabilities). A ship needs one to be drivable. |
| `Turret`     | Aims and fires bullets. Yaw/pitch speeds and limits, per-part meshes and offsets, `fire_rate`, `muzzle_speed`, authored `bullet_damage` + `bullet_kind`, optional `ammo_capacity`. |
| `Torpedo`    | Torpedo bay. Fires guided torpedoes that detonate an Explosive area blast (`blast_radius`, `blast_damage`), optional `ammo_capacity`. |

`GameSections(Vec<SectionConfig>)` is the resource of section blueprints,
populated in `crates/nova_assets/src/sections.rs`. Look one up with
`sections.get_section("basic_thruster_section")`.

## Building a ship

A `SpaceshipConfig` (`crates/nova_scenario/src/objects/spaceship.rs`) has a
`controller` (`None`, `Player`, or `AI`) and a list of `SpaceshipSectionConfig`,
each placing one section at a local grid `position` + `rotation`. The player
config carries the input mapping (section id -> key/gamepad bindings) plus
`speed_cap` and `infinite_ammo`; the AI config carries `patrol`/`orbit`/`leash`.

Spawning: the base scenario bundle gives the root `RigidBody::Dynamic`; the
spaceship object adds `SpaceshipRootMarker`, and an observer
(`insert_spaceship_sections`) spawns each section as a direct child. Every
section gets `SectionMarker`, a unit cuboid `Collider`, and `Health`
(`base_section` in `sections/base_section.rs`), so the ship is one rigid body
whose child colliders each carry their own health.

See the `asteroid_field` ship in `crates/nova_assets/src/scenario.rs` for a full
example. The editor (`crates/nova_editor`) assembles ships interactively using
`preview_section`, which has no health or rigid body and never enters the
damage pipeline.

## Integrity: damage -> disable -> destroy

The generic destruction core lives in the `bevy_common_systems` (bcs) crate
(`IntegrityPlugin`). Nova wraps it in `NovaIntegrityPlugin`
(`crates/nova_gameplay/src/integrity/`) and adds two nova-specific pieces:

- `glue.rs` - builds the graph and rolls section health up to the ship root.
- `explode.rs` - reacts to destruction: debris, mesh fragments, `OnDestroyedEvent`.

Graph build: when avian links a collider to its body (`ColliderOf`),
`build_integrity_relations` connects sections one grid unit apart via
`ConnectedTo` neighbor lists and marks the body `IntegrityRoot`. A lone body
(asteroid) gets an empty list, so it is a leaf.

Damage flow:

1. A hit triggers bcs `HealthApplyDamage`; bcs subtracts the amount and adds
   `HealthZeroMarker` at zero. The amount also bubbles up `ChildOf`, clamped to
   what the section actually had left - so overkill on one section cannot kill
   the ship (a 1000 hit on a 100 hp section costs the root 100).
2. Zero health -> `IntegrityDisabledMarker`. A disabled non-leaf section is only
   deactivated (`SectionInactiveMarker`); a disabled **leaf** is destroyed.
3. Destruction prunes the node from its neighbors' lists, which can create new
   leaves and cascade: shooting off the structure collapses what hung from it.
4. `aggregate_ship_health` keeps the root's health equal to the sum of its
   living sections; when the last section dies, the root dies with it.

## Typed damage (`crates/nova_gameplay/src/damage.rs`)

Weapon damage is authored, not emergent from bullet physics. A projectile
carries `ProjectileDamage { amount, kind }` with a `DamageType`: `Kinetic`,
`ArmorPiercing`, `Emp`, or `Explosive`. On hit, the amount is scaled by a
`resistance(section class, damage type)` table (for example EMP is 3.0 vs the
Controller but 0.1 vs Hull; Kinetic is always 1.0) and only then applied via
`HealthApplyDamage`. Targets without a `SectionDamageClass` (asteroids) take
the raw amount. Turret bullets are given a near-zero physical mass so bcs's
old mass-times-velocity damage is negligible; torpedoes detonate a typed
`NovaBlast` (Explosive, linear falloff) instead of bcs's untyped blast.

## Ammo

- `SectionAmmo` (`sections/ammo.rs`): optional magazine on a weapon section.
  Absent = unlimited fire; `ammo_capacity` in the turret/torpedo config opts in.
  The player `infinite_ammo` flag builds that ship's weapons without magazines.
- `LoadedBullet` (`sections/turret_section.rs`): the turret's loaded-round slot
  (damage type + amount), seeded from the config. Fired bullets and the HUD ammo
  readout colors read this slot, so swapping ammo types is one component write.
