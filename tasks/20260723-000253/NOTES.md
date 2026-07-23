# NOTES - 20260723-000253 Add SetAllegiance scenario action

## What was added

In `crates/nova_scenario/src/actions.rs`, mirroring `SetSpeedCap` exactly:

- `pub struct SetAllegianceActionConfig { pub id: String, pub allegiance: Allegiance }`
  with `#[derive(Clone, Debug)]` + serde-gated derives. Doc comment cites the
  neutral-until-provoked use.
- `EventActionConfig::SetAllegiance(SetAllegianceActionConfig)` enum variant +
  its dispatch arm (`config.action(world, info)`).
- The `EventAction` impl: pushes a command that queries
  `(Entity, &EntityId)` filtered `With<ScenarioScopedMarker>, With<SpaceshipRootMarker>`,
  finds the ship whose `entity_id.0 == id`, warns-and-returns if absent
  (`warn!("SetAllegiance: no scoped ship with id '{}'", id)`), else
  `world.entity_mut(ship).insert(allegiance)` (overwrites the component).
- Added `SetAllegianceActionConfig` to the module `prelude` re-export list.

## Allegiance import path

`Allegiance` is defined in `nova_gameplay::relations` (`pub enum Allegiance`) and
re-exported through `nova_gameplay::relations::prelude` -> `nova_gameplay::prelude`.
`actions.rs` already has `use nova_gameplay::prelude::*;`, so `Allegiance` was
ALREADY in scope - NO new import/dependency was needed. This is the same path the
spaceship spawn config uses (`objects/spaceship.rs` also imports
`nova_gameplay::prelude::*` and reads `allegiance: Option<Allegiance>`).

## Test approach

Two tests in the `#[cfg(test)] mod tests` at the bottom of actions.rs:

- `set_allegiance_action_round_trips_through_ron` (feature = "serde"): builds the
  `EventActionConfig::SetAllegiance` with id "x" + `Allegiance::Enemy`, serializes
  and deserializes via ron, asserts id and allegiance survive. Mirrors
  `set_skybox_action_round_trips_through_ron`.
- `set_allegiance_flips_the_scoped_ship`: follows the
  `set_controller_verb_flips_only_the_scoped_ship` pattern. Builds a `World`,
  `init_resource::<NovaEventWorld>()` + `init_resource::<GameObjectives>()` (the
  flush needs GameObjectives), spawns a ship with `ScenarioScopedMarker`,
  `SpaceshipRootMarker`, `EntityId::new("magpie")`, `Allegiance::Neutral`. Calls
  the action to flip to Enemy, flushes with `NovaEventWorld::state_to_world_system`
  (`EventWorld` imported from `bevy_common_systems::prelude`), asserts the
  component is now `Enemy`. Fail-first: without the apply path the component
  stays Neutral. Also drives a bad id ("nope"), flushes, asserts no panic and the
  ship is unchanged.

## Doc surface updated

- `web/src/wiki/dev/scenario-system.md` - added `SetAllegiance` to the action
  bullet list next to `SetControllerVerb`.
- `web/src/wiki/dev/guide-author-scenario.md` - added a `### SetAllegiance`
  section with a RON example, between `SetControllerVerb` and `CreateScenarioArea`.

## Verification (nix develop)

- `cargo test -p nova_scenario --lib set_allegiance`: 2 passed.
- `cargo check -p nova_scenario`: clean.
- `cargo fmt` + `cargo fmt --check`: clean.
- `cargo run -p nova_assets --bin content -- lint`: 0 error(s), 0 warning(s).
