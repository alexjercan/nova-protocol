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

### Entity cleanup contract (no leftovers between scenarios)

Both `UnloadScenario` and `on_load_scenario` despawn every `ScenarioScopedMarker`
entity and `clear()` the `NovaEventWorld` before the next scenario starts. `despawn()`
is recursive in Bevy, so despawning a scoped root takes its whole child hierarchy with
it. For a scene transition to leave nothing behind, **every** entity spawned while a
scenario is active must fall into one of these buckets:

1. **Explicitly scoped** - spawned with `ScenarioScopedMarker` (the scenario camera,
   directional light, input context, and every object from `SpawnScenarioObject` /
   `CreateScenarioArea`, since `base_scenario_object` includes the marker).
2. **Auto-scoped transients** - entities that spawn dynamically during play get the
   marker retroactively via `on_add_entity_with::<T>` observers in `loader.rs`, which
   tag any new entity carrying `T` while a scenario is loaded. Currently registered:
   `MeshFragmentMarker`, `TurretBulletProjectileMarker`, `TorpedoProjectileMarker`.
3. **Child of a scoped entity** - e.g. turret part meshes and muzzle effects are
   children of a ship section, so recursive despawn removes them.
4. **Self-despawning** - short-lived effects (torpedo blast) carry `TempEntity(secs)`
   and expire on their own; they may briefly outlive a transition but never leak.
5. **Tied to the player ship** - the HUDs spawn on `Add<PlayerSpaceshipMarker>` and
   despawn on `Remove<PlayerSpaceshipMarker>` (which fires when the scoped player ship
   is despawned during the switch).

Rule for new code: any new entity spawned during a scenario must be scoped (bucket 1),
carry a marker registered with `on_add_entity_with` (bucket 2), be a child of a scoped
entity (bucket 3), be a `TempEntity` (bucket 4), or be tied to a `Remove` observer
(bucket 5). If it fits none of these, it will leak across a scene switch.

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

### `OnOrbit`

Fired when a ship has HELD an engaged autopilot ORBIT around a well for
the hold window (5s; the orbit-hold tracker in `loader.rs`), and again
every further window while the hold continues - recurring, so an event
consumed while a handler's guard rejects it (wrong beat) is simply
retried next window instead of soft-locking the script. Disengaging (or
switching wells) restarts the clock. `id` is the well's scenario id and
`other` the ship, the same shape as `OnEnter`, so filters compose
identically; gate handlers on script state to make repeats no-ops. This
is the right gate for "make orbit" objectives: a position gate is
unwinnable because the ORBIT verb rings at max(clearance band, engage
radius).

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
- `SpawnScenarioObject(ScenarioObjectConfig)` - spawn an `Asteroid`, `Spaceship`,
  `Beacon` or `SalvageCrate`.
- `DespawnScenarioObject(DespawnScenarioObjectActionConfig)` - despawn the scenario
  object whose id matches (recursive; scoped entities only, so a ship section that
  happens to share the id string is never touched). The complement of
  `SpawnScenarioObject`, e.g. removing a salvage crate on pickup.
- `CreateScenarioArea(ScenarioAreaConfig)` - create a spherical trigger zone (drives
  `OnEnter` / `OnExit`).
- `SetSpeedCap(SetSpeedCapActionConfig)` - install (`Some(cap)`) or remove (`None`)
  the manual `FlightSpeedCap` on a scoped ship by id (the shakedown training
  governor releases at beacon 1).
- `ObjectiveMarkerAttach` / `ObjectiveMarkerDetach` - attach the gold objective
  marker chip (label + live distance, edge-clamped as a direction cue) to the
  scoped object whose id matches, or take it off. Attaching inserts nova_gameplay's
  `ObjectiveMarkerTarget`; the HUD observer does the rest, and the marked entity
  despawning detaches implicitly (the chip dies with it). While a beacon is marked
  its cyan beacon chip yields to the marker, so one entity shows one chip. An
  attach ordered after a spawn in the same handler sees the fresh entity.
- `HintEmphasisSet` / `HintEmphasisClear` - pulse one keybind-cluster verb row
  toward objective gold (the row names: STOP, GOTO, ORBIT, CANCEL, COMPONENT,
  TARGET), so an objective like "press [G]" can point at its row without new UI.
  Emphasis never changes availability - a dim row pulses dim. Scenario teardown
  clears all emphasis, so death/exit cannot leak a pulsing row into the next
  scenario.
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

- `Asteroid` (`AsteroidConfig`: radius, texture, health, `invulnerable`) - procedural
  destructible asteroid; `invulnerable: true` omits the health node entirely, so a
  designated body (a tutorial planetoid) and its well can never be shot away.
- `Spaceship` (`SpaceshipConfig`) - a ship built from sections, controlled by either a
  `Player` (with an input mapping) or `AI` controller (patrol route, orbit directive,
  and an optional `leash` radius: combat breaks off beyond it and the ship returns to
  its routine - territorial enemies). See [sections.md](sections.md).
- `Area` (`ScenarioAreaConfig`: position, radius) - trigger zone for enter/exit events.
- `Beacon` (`BeaconConfig`: label, radius, color, `area_radius: Option<f32>`) - a nav
  waypoint: emissive blinking orb, on rails (`RigidBody::Static`) but aim-lockable via
  its authored `LockSignature` (the targeting gate admits signed statics), with a HUD
  chip (label + live distance, edge-clamped as a direction cue) that nova_gameplay
  hangs off `BeaconMarker` automatically. With `area_radius` set the beacon doubles as
  its own trigger area: `OnEnter`/`OnExit` fire under the beacon's scenario id, no
  separate `CreateScenarioArea` needed.
- `SalvageCrate` (`SalvageCrateConfig`: size, area_radius) - a minimal proximity
  pickup: a bright tumbling box that is its own sensor trigger. Flying through it
  fires `OnEnter` under the crate's id; the script pairs that with
  `DespawnScenarioObject` and whatever counting it wants ("collected" is a scenario
  variable, not an item system).

## Where the built-in scenarios live

`crates/nova_assets/src/scenario.rs` builds `asteroid_field`, `asteroid_next` and
`menu_ambience` in Rust and inserts them into `GameScenarios`;
`crates/nova_assets/src/scenario/shakedown.rs` adds `shakedown_run`, the starter
scenario New Game loads. This is a stand-in for loading scenarios from data files
(there is an explicit `// This should be loaded from a JSON file` note in
`sections.rs`); a real modding pipeline would deserialize these configs.

`shakedown_run` is the reference example of the beat-chain idiom: one `beat`
counter variable gates every handler (no finished beat can re-fire), pickups
are per-entity `OnEnter` handlers paired with `DespawnScenarioObject`, and
count milestones (the crate tally, the beat advance) run on `OnUpdate`
handlers keyed on the counter value - deliberately NOT piggybacked on the
pickup event, because handler execution order within one event is
query-iteration order and should not be load-bearing.
