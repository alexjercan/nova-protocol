# Scenario / modding system

The `nova_scenario` crate is the data-driven scenario and modding engine. A scenario
declares a set of **event handlers**, each made of an **event** to listen for, a list
of **filters** that must all pass, and a list of **actions** to run. This is the layer
you touch to build missions, objectives, and reactive world behavior.

It is built on the generic `GameEventsPlugin`/`EventWorld` machinery from
`bevy-common-systems`. `nova_scenario` provides the Nova-specific event world
(`NovaEventWorld`) and the concrete event/filter/action config enums.

## Core types

- `ScenarioConfig` - a whole scenario: `id`, `name`, `description`, `cubemap`
  (skybox handle), and `events: Vec<ScenarioEventConfig>`.
- `ScenarioEventConfig` - one handler: `name: EventConfig` + `filters` + `actions`.
- `GameScenarios(HashMap<ScenarioId, ScenarioConfig>)` - resource holding all
  available scenarios, populated in `nova_assets` at `GameAssetsStates::Loaded`.
- `CurrentScenario(Option<ScenarioConfig>)` - the loaded scenario, if any.

## Loading / unloading

Loading is driven by observers on events (see `loader.rs`):

- `LoadScenario(ScenarioConfig)` - trigger to load a scenario. Typical use: look one
  up in `GameScenarios` by id and `commands.trigger(LoadScenario(cfg.clone()))`.
- `ScenarioLoaded` - fired once a load has finished.
- `UnloadScenario` - despawns everything tagged `ScenarioScopedMarker` and clears the
  `NovaEventWorld`.
- `ScenarioScopedMarker` - component; any entity carrying it is torn down on unload.
  Spawn scenario entities with this marker so cleanup is automatic.

Example (from `examples/03_scenario.rs`):

```rust
app.add_systems(
    OnEnter(GameAssetsStates::Loaded),
    |mut commands: Commands, scenarios: Res<GameScenarios>| {
        let scenario = scenarios.get("asteroid_field").expect("not found");
        commands.trigger(LoadScenario(scenario.clone()));
    },
);
```

## Events (`EventConfig`)

Maps 1:1 onto the event kinds in `nova_events`:

| `EventConfig` | Fires when | Info payload |
|---------------|------------|--------------|
| `OnStart`     | scenario/entity starts | `OnStartEventInfo` |
| `OnUpdate`    | each tick | `OnUpdateEventInfo` |
| `OnDestroyed` | an entity is destroyed | `OnDestroyedEventInfo` |
| `OnEnter`     | an entity enters an area/zone | `OnEnterEventInfo` |
| `OnExit`      | an entity leaves an area/zone | `OnExitEventInfo` |

Entities carry `EntityId(String)` and `EntityTypeName(String)` components so filters
and actions can identify who triggered an event (and, for enter/exit, the "other"
entity via the `other_id` / `other_type_name` fields).

## Filters (`EventFilterConfig`)

All filters on a handler must pass for its actions to run.

- `Entity(EntityFilterConfig)` - match on the triggering entity's id / type name.
- `Expression(ExpressionFilterConfig)` - evaluate a `VariableConditionNode` against
  the scenario variables (see below).
- `Conditional(...)` - `Not` / `And` / `Or` combinators over other filters. Build with
  `ConditionalFilterConfig::not/and/or(...)`.

## Actions (`EventActionConfig`)

Run in order when an event passes its filters:

- `DebugMessage` - log a message (debugging scenarios).
- `VariableSet` - evaluate an expression and store it into a scenario variable.
- `Objective` / `ObjectiveComplete` - add or complete an objective (surfaced in the
  objectives HUD).
- `SpawnScenarioObject(ScenarioObjectConfig)` - spawn an `Asteroid` or `Spaceship`.
- `CreateScenarioArea(ScenarioAreaConfig)` - create a spherical trigger zone (drives
  `OnEnter` / `OnExit`).
- `NextScenario(NextScenarioActionConfig)` - switch to another scenario by id (with a
  `linger` flag to defer the switch).

## Variables and the event world

`NovaEventWorld` (`world.rs`) is the scenario-scoped state that lives between frames.
It implements `EventWorld` with a two-phase model:

- Filters/actions mutate `NovaEventWorld` (variables, objectives, `next_scenario`, and
  a queue of deferred `Commands` closures) - they do **not** touch the Bevy `World`
  directly.
- `state_to_world_system` then applies the effects back to the Bevy world: copies
  objectives into `GameObjectivesHud`, performs a queued `NextScenario` switch, and
  flushes queued spawn/despawn commands via a `CommandQueue`.

This indirection is why actions use `world.push_command(|commands| ...)` to spawn
things rather than spawning inline. `NovaEventWorld::clear()` resets all of it on
load/unload.

Variables are a small expression language (`variables.rs`): literals, condition
nodes, and set-expressions, evaluated by filters (`ExpressionFilterConfig`) and the
`VariableSet` action.

## Scenario objects (`objects/`)

- `Asteroid` (`AsteroidConfig`: radius, texture, health) - procedural destructible
  asteroid.
- `Spaceship` (`SpaceshipConfig`) - a ship built from sections, controlled by either a
  `Player` (with an input mapping) or `AI` controller. See [sections.md](sections.md).
- `Area` (`ScenarioAreaConfig`: position, radius) - trigger zone for enter/exit events.

## Where the built-in scenarios live

`crates/nova_assets/src/scenario.rs` builds `asteroid_field` and `asteroid_next` in
Rust and inserts them into `GameScenarios`. This is a stand-in for loading scenarios
from data files (there is an explicit `// This should be loaded from a JSON file`
note in `sections.rs`); a real modding pipeline would deserialize these configs.
