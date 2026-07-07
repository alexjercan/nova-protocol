# Spaceship sections and integrity

Ships in Nova Protocol are assembled from modular **sections**. Each section is a
child entity of the ship root, has its own mass and health, and contributes a
behavior (thrust, aiming, firing, structure). The **integrity** system tracks how
sections are connected and handles damage, disabling, and chain-reaction destruction.

## Sections (`nova_gameplay::sections`)

A section is a `SectionConfig { base: BaseSectionConfig, kind: SectionKind }`.

`BaseSectionConfig` (shared by all kinds): `id`, `name`, `description`, `mass`,
`health`.

`SectionKind` variants (`base_section.rs`):

| Kind         | Config highlights |
|--------------|-------------------|
| `Hull`       | `render_mesh`. Passive structure/armor. |
| `Thruster`   | `magnitude`, `render_mesh`. Produces forward thrust; drives the exhaust shader. |
| `Controller` | `frequency`, `damping_ratio`, `max_torque`. PD attitude controller (steering). A ship needs a controller to be player/AI drivable. |
| `Turret`     | yaw/pitch speeds + limits, per-part meshes and offsets (base/yaw/pitch/barrel), `muzzle_offset`, `fire_rate`, `muzzle_speed`, projectile params, optional `muzzle_effect`. Aims and fires bullet projectiles. |
| `Torpedo`    | torpedo bay; fires guided torpedoes that deal **blast** (area) damage. |

`GameSections(Vec<SectionConfig>)` is the resource of available section blueprints,
populated in `crates/nova_assets/src/sections.rs`. Look sections up by id with
`sections.get_section("basic_thruster_section")`.

### Building a ship

A `SpaceshipConfig` has a `controller` (`Player` with an input mapping, or `AI`) and a
`Vec<SpaceshipSectionConfig>`. Each `SpaceshipSectionConfig` places one section at a
local `position` + `rotation` relative to the ship root
(`SpaceshipRootMarker`). See the `asteroid_field` ship in
`crates/nova_assets/src/scenario.rs` for a full worked example (controller + two
hulls + thruster + turret with an input mapping binding `thruster`/`turret` actions to
keys and gamepad buttons).

The editor scene in `crates/nova_editor/src/lib.rs` (`NovaEditorPlugin`) lets you
assemble ships interactively.

## Input (`nova_gameplay::input`)

- `player.rs` - maps `bevy_enhanced_input` actions to section behaviors using the
  per-ship input mapping (named actions like `"thruster"`, `"turret"`).
- `ai.rs` - AI controller that drives the same section behaviors without human input.

## Integrity system (`nova_gameplay::integrity`)

This is the damage and destruction model. Everything is observer-driven (`On<...>`).

Key components (`components.rs`):

- `IntegrityRoot` - marks the body that owns the structure (a ship root, or a lone
  body like an asteroid). Used to find the body for aggregate health and whole-body
  destruction.
- `ConnectedTo(Vec<Entity>)` - the neighbor list stored on each integrity node (a ship
  section, or an asteroid's collider node). This replaces the old central
  `IntegrityGraph(HashMap<...>)`: connectivity now lives per-node instead of in one map
  on the root. Built by `build_integrity_relations` (in `glue.rs`) as colliders link.
- `IntegrityLeafMarker` - a node with at most one neighbor (a leaf in the connectivity
  graph).
- `IntegrityDisabledMarker` - a section whose health hit zero (disabled).
- `IntegrityDestroyMarker` - a node queued for destruction.

Damage/destruction flow (observers in `plugin.rs`):

1. Colliders spawn collision events (`on_collider_of_spawn_insert_collision_events`).
2. Impact and blast collisions deal damage
   (`on_impact_collision_deal_damage`, `on_blast_collision_deal_damage`;
   blast falloff computed by `calculate_blast_damage`).
3. When health reaches zero a `HealthZeroMarker` is added, which inserts
   `IntegrityDisabledMarker` (`on_health_depleted_insert_disabled`).
4. Destruction propagates: `handle_destroy`, `handle_chain_destroy` (a disabled leaf
   destroys, pruning neighbor links and creating new leaves), and `handle_parent_destroy`.
5. Destruction finalizes across two crates: `on_destroyed` (in `plugin.rs`) prunes the
   destroyed node from its neighbors' `ConnectedTo` lists; `on_destroyed_entity` (in
   `integrity/explode.rs`) fires the game `OnDestroyed` event; and the explode systems
   there spawn debris/mesh fragments and despawn the meshless remains.

The chain-reaction rule of thumb: **a section is destroyed when it is both disabled
(zero health) and a leaf** (nothing structurally depends on it). Destroying it can
turn its neighbors into leaves, cascading the failure - so shooting off a ship's
structure can collapse the pieces hanging from it.

Blast damage (`integrity/blast.rs`) is what torpedoes use: area damage that falls off
with distance from the detonation, applied to every section within range.

## HUD (`nova_gameplay::hud`)

Player-facing overlays: `health`, `objectives` (fed by scenario objective actions via
`GameObjectivesHud`), `torpedo_target`, and `velocity`.
