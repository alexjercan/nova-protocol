/// Scenario loader plugin and related types
use bevy::{platform::collections::HashMap, prelude::*};
use bevy_common_systems::prelude::*;
use bevy_enhanced_input::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;

use crate::prelude::*;

pub mod prelude {
    pub use super::{
        scenario_is_live, CurrentScenario, GameScenarios, LoadScenario, ScenarioConfig,
        ScenarioEventConfig, ScenarioId, ScenarioLoaded, ScenarioLoaderPlugin,
        ScenarioScopedMarker, UnloadScenario,
    };
}

/// Type alias for Scenario ID
pub type ScenarioId = String;

/// The collection of available game scenarios
#[derive(Resource, Clone, Debug, Deref, DerefMut, Default)]
pub struct GameScenarios(pub HashMap<ScenarioId, ScenarioConfig>);

/// Configuration for a game scenario
#[derive(Clone, Debug)]
pub struct ScenarioConfig {
    /// Unique identifier for the scenario
    pub id: ScenarioId,
    /// The display name of the scenario
    pub name: String,
    /// A brief description of the scenario
    pub description: String,
    /// The cubemap image used for the scenario's skybox
    pub cubemap: Handle<Image>,
    /// Events associated with the scenario
    pub events: Vec<ScenarioEventConfig>,
}

/// Configuration for a scenario event
#[derive(Clone, Debug)]
pub struct ScenarioEventConfig {
    /// The name of the event to listen for
    pub name: EventConfig,
    /// Filters to apply to the event
    pub filters: Vec<EventFilterConfig>,
    /// Actions to perform when the event is triggered
    pub actions: Vec<EventActionConfig>,
}

/// Load a scenario given the configuration (this can be read from the GameScenarios resource).
/// e.g we could display all the scenario names in a menu and load the selected one.
#[derive(Event, Clone, Debug, Deref, DerefMut)]
pub struct LoadScenario(pub ScenarioConfig);

/// Unload the current scenario. This event guarantees that all scenario-scoped entities are
/// removed from the world.
#[derive(Event, Clone, Debug)]
pub struct UnloadScenario;

/// Event that is triggered once a scenario has been successfully loaded. Carries a snapshot of
/// the loaded scenario's init status so consumers (e.g. the autopilot/screenshot smoke harness)
/// can assert on it and so scenario init is easier to debug.
#[derive(Event, Clone, Debug)]
pub struct ScenarioLoaded {
    /// The id of the scenario that was loaded.
    pub scenario_id: ScenarioId,
    /// The number of event handlers registered for the scenario (one per `ScenarioEventConfig`).
    pub handler_count: usize,
    /// The number of scenario objects the scenario will spawn, counted from the
    /// `SpawnScenarioObject` actions across all of its events.
    pub object_count: usize,
}

impl ScenarioLoaded {
    /// Build the load-status snapshot from a scenario config. The counts come straight from the
    /// config: one handler per event, and one object per `SpawnScenarioObject` action.
    fn from_config(scenario: &ScenarioConfig) -> Self {
        let object_count = scenario
            .events
            .iter()
            .flat_map(|event| event.actions.iter())
            .filter(|action| matches!(action, EventActionConfig::SpawnScenarioObject(_)))
            .count();
        Self {
            scenario_id: scenario.id.clone(),
            handler_count: scenario.events.len(),
            object_count,
        }
    }
}

/// The current loaded scenario, if any. This will contain the scenario configuration.
#[derive(Resource, Clone, Debug, Deref, DerefMut, Default)]
pub struct CurrentScenario(pub Option<ScenarioConfig>);

/// Marker that indicates that an entity is scoped to the current scenario.
/// When a scenario is unloaded, all entities with this marker will be despawned.
#[derive(Component, Debug, Clone, Reflect)]
pub struct ScenarioScopedMarker;

/// Run condition: a scenario is currently loaded. This is what gates the
/// spaceship input/section system sets (below), so ships fly, fire and hum
/// exactly while a simulation is live - a gameplay scenario, the main menu's
/// ambience backdrop, an example - and stay dead otherwise. The editor's
/// build-mode preview relies on this: its preview sections carry live input
/// bindings, but the Editor state never has a scenario loaded, so nothing
/// acts on them.
pub fn scenario_is_live(current: Res<CurrentScenario>) -> bool {
    current.is_some()
}

pub struct ScenarioLoaderPlugin;

impl Plugin for ScenarioLoaderPlugin {
    fn build(&self, app: &mut App) {
        debug!("ScenarioLoaderPlugin: build");

        configure_scenario_gating(app);

        app.add_observer(on_player_spaceship_spawned);
        app.add_observer(on_player_spaceship_destroyed);

        app.init_resource::<CurrentScenario>();
        app.add_observer(on_load_scenario);

        app.add_observer(on_add_entity_with::<MeshFragmentMarker>);
        app.add_observer(on_add_entity_with::<TurretBulletProjectileMarker>);
        app.add_observer(on_add_entity_with::<TorpedoProjectileMarker>);

        app.add_input_context::<ScenarioInputMarker>();
        app.add_observer(on_next_input);
        app.add_observer(unload_scenario);

        // OnUpdate handlers were dead config until task 20260711-180506:
        // EventConfig::OnUpdate existed (and the docs advertised it) but
        // nothing ever fired the event. The pulse runs only while a
        // scenario is live - same liveness rule as the ship systems.
        app.add_systems(Update, fire_on_update.run_if(scenario_is_live));

        // The orbit-hold tracker behind `EventConfig::OnOrbit` (task
        // 20260712-110730 finding 5): "in orbit" is autopilot state, not
        // a position - a gate area is unwinnable because the ORBIT verb
        // rings at max(clearance band, engage radius).
        app.register_type::<OrbitHold>();
        app.add_systems(Update, track_orbit_holds.run_if(scenario_is_live));

        // The player-lock bridge behind `EventConfig::OnTravelLock` /
        // `OnCombatLock` (beat-sheet-v2 spike 20260713-140742): lock
        // lessons tick the instant the lock lands.
        app.register_type::<LockEcho>();
        app.add_systems(Update, track_player_locks.run_if(scenario_is_live));
    }
}

/// The per-frame pulse behind `EventConfig::OnUpdate` handlers. Scenarios
/// use it for value-gated milestones (e.g. shakedown's crate tally), which
/// must not depend on handler execution order within another event.
fn fire_on_update(mut commands: Commands) {
    commands.fire::<OnUpdateEvent>(OnUpdateEventInfo);
}

/// How long (seconds) a ship must hold an engaged ORBIT around one well
/// before [`OnOrbitEvent`] fires - and the RE-FIRE period while the hold
/// continues. Recurring, not once-per-engagement (review R1.1 of task
/// 20260712-110730): a single-shot event consumed while a handler's beat
/// guard rejects it would be gone for good, soft-locking any scenario
/// whose beat can advance during a held orbit. Beat-gated handlers make
/// the repeats no-ops.
const ORBIT_HOLD_SECS: f32 = 5.0;

/// Bookkeeping for the orbit-hold tracker, on the orbiting ship: which
/// well and how long the current window has been held. Disengaging (or
/// switching wells) removes it, restarting the clock.
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct OrbitHold {
    pub well: Entity,
    pub held_secs: f32,
}

/// Fire [`OnOrbitEvent`] once a ship has HELD an engaged
/// `Autopilot { action: Orbit { well } }` for [`ORBIT_HOLD_SECS`]
/// continuously. Ships are identified by their scenario `EntityId` (ships
/// without one - editor previews - are invisible to the tracker); the
/// event's `id` is the WELL's scenario id, mirroring OnEnter's
/// (area, other) shape so filters compose identically.
fn track_orbit_holds(
    time: Res<Time>,
    mut commands: Commands,
    mut q_ships: Query<
        (
            Entity,
            &Autopilot,
            Option<&mut OrbitHold>,
            &EntityId,
            &EntityTypeName,
        ),
        With<SpaceshipRootMarker>,
    >,
    q_disengaged: Query<Entity, (With<OrbitHold>, Without<Autopilot>)>,
    q_ids: Query<&EntityId>,
) {
    // Disengaged ships re-arm: the hold dies with the autopilot.
    for ship in &q_disengaged {
        commands.entity(ship).remove::<OrbitHold>();
    }

    for (ship, autopilot, hold, ship_id, ship_type_name) in &mut q_ships {
        let AutopilotAction::Orbit { well, .. } = autopilot.action else {
            // Engaged, but not an orbit (GOTO/STOP): no hold.
            if hold.is_some() {
                commands.entity(ship).remove::<OrbitHold>();
            }
            continue;
        };

        match hold {
            Some(mut hold) if hold.well == well => {
                hold.held_secs += time.delta_secs();
                if hold.held_secs >= ORBIT_HOLD_SECS {
                    // Restart the window whether or not the event can be
                    // addressed (review R1.2: a well without a scenario id
                    // must not consume the hold - the next window retries).
                    hold.held_secs = 0.0;
                    let Ok(well_id) = q_ids.get(well) else {
                        // A well without a scenario id (despawned or
                        // non-scenario body) has no address to fire under.
                        continue;
                    };
                    debug!(
                        "track_orbit_holds: ship '{}' held orbit around '{}' for {}s",
                        ship_id.0, well_id.0, ORBIT_HOLD_SECS
                    );
                    commands.fire::<OnOrbitEvent>(OnOrbitEventInfo {
                        id: well_id.0.clone(),
                        other_id: ship_id.0.clone(),
                        other_type_name: ship_type_name.0.clone(),
                    });
                }
            }
            // New engagement, or the directive switched wells: restart the
            // clock on the current well.
            _ => {
                commands.entity(ship).insert(OrbitHold {
                    well,
                    held_secs: 0.0,
                });
            }
        }
    }
}

/// Re-fire period (seconds) for a HELD lock. Acquisition fires immediately;
/// while the lock stays on the same target the event RECURS on this period
/// - the orbit-hold rationale (review R1.1 of 20260712-110730): a one-shot
/// event consumed under a rejecting beat guard is gone for good, and a
/// scenario whose beat advances while the lock is already held would
/// soft-lock. Beat-gated handlers make the repeats no-ops.
const LOCK_REFIRE_SECS: f32 = 5.0;

/// Bookkeeping for the player-lock bridge: per slot, the last target the
/// bridge saw and the seconds since it last fired for it.
#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct LockEcho {
    travel: Option<(Entity, f32)>,
    combat: Option<(Entity, f32)>,
}

/// One lock slot's tick: returns `Some(target)` when the bridge should
/// fire this frame - on ACQUISITION (the slot's value changed onto a
/// target; the slot writers are equality-skipped, so a held live-radar
/// lock does not churn this) and again every [`LOCK_REFIRE_SECS`] while
/// the same target stays held. Pure for the unit tests.
fn tick_lock_slot(
    state: &mut Option<(Entity, f32)>,
    current: Option<Entity>,
    delta_secs: f32,
) -> Option<Entity> {
    match (current, state.as_mut()) {
        (None, _) => {
            *state = None;
            None
        }
        (Some(target), Some((held, secs))) if *held == target => {
            *secs += delta_secs;
            if *secs >= LOCK_REFIRE_SECS {
                *secs = 0.0;
                Some(target)
            } else {
                None
            }
        }
        (Some(target), _) => {
            *state = Some((target, 0.0));
            Some(target)
        }
    }
}

/// Fire [`OnTravelLockEvent`]/[`OnCombatLockEvent`] when the PLAYER's lock
/// slots land on scenario objects. Player-scoped on purpose: the AI combat
/// mirror (nova_gameplay input/ai.rs) writes `CombatLock` on every engaged
/// AI ship, and an unscoped bridge would fire for all of them. The event's
/// `id` is the locked TARGET's scenario id (a target without one - debris,
/// editor previews - fires nothing; the re-fire window retries, mirroring
/// the orbit tracker's R1.2), `other` is the player ship - OnEnter's
/// (area, other) shape, so filters compose identically.
fn track_player_locks(
    time: Res<Time>,
    mut commands: Commands,
    mut q_ships: Query<
        (
            Entity,
            &TravelLock,
            &CombatLock,
            Option<&mut LockEcho>,
            &EntityId,
            &EntityTypeName,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    q_ids: Query<&EntityId>,
) {
    for (ship, travel, combat, echo, ship_id, ship_type_name) in &mut q_ships {
        let Some(mut echo) = echo else {
            // First sight of this player ship: arm the bookkeeping; the
            // next frame ticks it (an already-held lock then reads as an
            // acquisition, which is the honest interpretation on spawn).
            commands.entity(ship).insert(LockEcho::default());
            continue;
        };
        let fired_travel = tick_lock_slot(&mut echo.travel, travel.0, time.delta_secs());
        let fired_combat = tick_lock_slot(&mut echo.combat, combat.0, time.delta_secs());
        if let Some(target_id) = fired_travel.and_then(|target| q_ids.get(target).ok()) {
            commands.fire::<OnTravelLockEvent>(OnTravelLockEventInfo {
                id: target_id.0.clone(),
                other_id: ship_id.0.clone(),
                other_type_name: ship_type_name.0.clone(),
            });
        }
        if let Some(target_id) = fired_combat.and_then(|target| q_ids.get(target).ok()) {
            commands.fire::<OnCombatLockEvent>(OnCombatLockEventInfo {
                id: target_id.0.clone(),
                other_id: ship_id.0.clone(),
                other_type_name: ship_type_name.0.clone(),
            });
        }
    }
}

/// Ships act only while a scenario is live: gate the spaceship input/section
/// sets on [`scenario_is_live`]. Owned here rather than by the editor (which
/// used to gate these on its private Scenario state): scenario-liveness is
/// the gate's real meaning, and it holds for every consumer - editor sandbox,
/// menu ambience, examples. Composes by AND with nova_gameplay's pause
/// gating (run conditions from separate configure_sets calls compose).
/// Factored out so the tests below exercise the production wiring, same as
/// nova_gameplay's configure_pause_gating.
pub(crate) fn configure_scenario_gating(app: &mut App) {
    app.configure_sets(
        Update,
        (SpaceshipInputSystems, SpaceshipSectionSystems).run_if(scenario_is_live),
    );
    app.configure_sets(
        FixedUpdate,
        SpaceshipSectionSystems.run_if(scenario_is_live),
    );
    // Deliberately NOT gated: the PostUpdate instance of
    // SpaceshipSectionSystems (the turret aim chain). It was never gated by
    // the old editor-state gate either (parity), and it is read-only pose
    // math feeding cosmetics/HUD - gating it is a separate decision.
}

/// Tear down the currently-loaded scenario: clear the event world and despawn every
/// scenario-scoped entity (despawn is recursive, so their children go too). Shared by
/// both the unload path and the load path (which tears the old scenario down before
/// spawning the next), so teardown is identical no matter how a scenario ends.
fn teardown_scenario_entities(
    commands: &mut Commands,
    q_scoped: &Query<Entity, With<ScenarioScopedMarker>>,
    world: &mut NovaEventWorld,
    emphasis: Option<&mut HintEmphasis>,
) {
    world.clear();
    // Scenario-driven HUD emphasis dies with the scenario: a leaked
    // emphasis would pulse a verb row into the next scenario (or the
    // menu's death gap) with nothing left to clear it - the same reset
    // class as the objectives diff (state-diff-aliases-reset,
    // 20260712-125342). Optional: headless rigs run without the HUD.
    if let Some(emphasis) = emphasis {
        emphasis.clear_all();
    }
    for entity in q_scoped.iter() {
        commands.entity(entity).despawn();
    }
}

fn unload_scenario(
    _: On<UnloadScenario>,
    mut commands: Commands,
    q_scoped: Query<Entity, With<ScenarioScopedMarker>>,
    mut current_scenario: ResMut<CurrentScenario>,
    mut world: ResMut<NovaEventWorld>,
    mut emphasis: Option<ResMut<HintEmphasis>>,
) {
    teardown_scenario_entities(
        &mut commands,
        &q_scoped,
        &mut world,
        emphasis.as_deref_mut(),
    );
    **current_scenario = None;
}

fn on_load_scenario(
    load: On<LoadScenario>,
    mut commands: Commands,
    mut current_scenario: ResMut<CurrentScenario>,
    q_scoped: Query<Entity, With<ScenarioScopedMarker>>,
    mut world: ResMut<NovaEventWorld>,
    mut emphasis: Option<ResMut<HintEmphasis>>,
) {
    teardown_scenario_entities(
        &mut commands,
        &q_scoped,
        &mut world,
        emphasis.as_deref_mut(),
    );

    let scenario = (**load).clone();
    **current_scenario = Some(scenario.clone());
    debug!("on_load_scenario: scenario {:?}", scenario.name);

    // Setup Scenario Camera. `SfxListenerMarker` makes this the explicit
    // SFX/juice listener (attenuation, camera shake, flash facing); the editor
    // camera deliberately never carries it.
    commands.spawn((
        ScenarioScopedMarker,
        ScenarioCameraMarker,
        SfxListenerMarker,
        Name::new("Scenario Camera"),
        Camera3d::default(),
        PostProcessingCamera,
        WASDCameraController,
        Transform::from_xyz(0.0, 10.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
        SkyboxConfig {
            cubemap: scenario.cubemap.clone(),
            brightness: 1000.0,
        },
    ));

    // Setup directional light
    commands.spawn((
        ScenarioScopedMarker,
        DirectionalLight {
            illuminance: 10000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -std::f32::consts::FRAC_PI_2,
            0.0,
            0.0,
        )),
        GlobalTransform::default(),
    ));

    // Setup scenario input context
    commands.spawn((
        ScenarioScopedMarker,
        Name::new(format!("Scenario Input Context: {}", scenario.name)),
        ScenarioInputMarker,
        actions!(
            ScenarioInputMarker[(
                Name::new("Input: Next Scenario"),
                Action::<NextScenarioInput>::new(),
                // DPadDown: moved off South, which ORBIT now uses (player.rs).
                bindings![KeyCode::Enter, GamepadButton::DPadDown]
            )]
        ),
    ));

    // Setup scenario events
    for event in scenario.events.iter() {
        let mut event_handler = EventHandler::<NovaEventWorld>::from(event.name);
        for filter in event.filters.iter() {
            event_handler.add_filter(filter.clone());
        }
        for action in event.actions.iter() {
            event_handler.add_action(action.clone());
        }
        commands.spawn((
            ScenarioScopedMarker,
            Name::new(format!("Event Handler: {:?}", event.name)),
            event_handler,
        ));
    }

    // Trigger ScenarioLoaded event with a snapshot of the init status.
    let loaded = ScenarioLoaded::from_config(&scenario);
    debug!(
        "on_load_scenario: loaded scenario '{}' with {} handler(s) and {} object(s)",
        loaded.scenario_id, loaded.handler_count, loaded.object_count
    );
    commands.trigger(loaded);

    // Fire onstart event
    commands.fire::<OnStartEvent>(OnStartEventInfo);
}

fn on_add_entity_with<T: Component>(
    add: On<Add, T>,
    mut commands: Commands,
    current_scenario: Res<CurrentScenario>,
) {
    if let Some(scenario) = &**current_scenario {
        trace!(
            "on_add_entity_with: Added entity {:?} in scenario {:?}",
            add.entity,
            scenario.name
        );

        commands.entity(add.entity).insert(ScenarioScopedMarker);
    }
}

#[derive(Component, Debug, Clone)]
struct ScenarioInputMarker;

#[derive(InputAction)]
#[action_output(bool)]
struct NextScenarioInput;

fn on_next_input(
    _: On<Start<NextScenarioInput>>,
    mut world: ResMut<super::world::NovaEventWorld>,
    pause: Res<State<PauseStates>>,
) {
    // Observers bypass system-set gating; freeze intent changes while the
    // pause overlay is up (review R1.1). Releases stay ungated so held keys
    // clear cleanly during a pause.
    if *pause.get() == PauseStates::Paused {
        return;
    }

    let Some(mut next_scenario) = world.next_scenario.clone() else {
        return;
    };

    next_scenario.linger = false;
    world.next_scenario = Some(next_scenario);
}

#[derive(Component, Debug, Clone)]
struct ScenarioCameraMarker;

fn on_player_spaceship_spawned(
    add: On<Add, PlayerSpaceshipMarker>,
    mut commands: Commands,
    camera: Single<Entity, With<ScenarioCameraMarker>>,
) {
    trace!("on_player_spaceship_spawned: {:?}", add.entity);

    let camera = camera.into_inner();

    commands
        .entity(camera)
        .remove::<WASDCameraController>()
        .insert(SpaceshipCameraController);
}

fn on_player_spaceship_destroyed(
    _remove: On<Remove, PlayerSpaceshipMarker>,
    mut commands: Commands,
    camera: Single<Entity, With<SpaceshipCameraController>>,
) {
    trace!("on_player_spaceship_destroyed: switching camera back to WASD");

    let camera = camera.into_inner();

    // Plain remove/insert are safe here even during the unload sweep: this
    // observer's commands apply BEFORE the queue's remaining despawns
    // (probed in 20260712-115902 - see
    // `a_player_ships_despawn_does_not_race_the_cameras`, which pins that
    // ordering), and the `Single` only resolves a live camera; when the
    // camera's despawn applied first, the observer skips entirely. Only
    // commands targeting the entity whose OWN despawn triggered the
    // observer race - that is remove_maneuver_telemetry's case, not this
    // one.
    commands
        .entity(camera)
        .remove::<SpaceshipCameraController>()
        .insert(WASDCameraController);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// In-memory log sink for asserting on command warns (duplicate of
    /// nova_gameplay's crate-private `test_log::CapturedLog` - a shared
    /// test-util crate is not worth one 20-line helper). `remove`/`despawn`
    /// bake in the WARN handler at queue time, so warns are only observable
    /// through the log (task 20260712-115902).
    #[derive(Clone, Default)]
    struct CapturedLog(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

    impl CapturedLog {
        fn contents(&self) -> String {
            String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
        }
        fn clear(&self) {
            self.0.lock().unwrap().clear();
        }
    }

    impl std::io::Write for CapturedLog {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// ORDERING PIN (task 20260712-115902): cross-entity observer commands
    /// do NOT race the unload sweep. Sweep-style queue [despawn(ship),
    /// despawn(camera)]: the ship's despawn fires
    /// `on_player_spaceship_destroyed` while the camera is still live, and
    /// bevy applies the observer's remove+insert BEFORE the camera's
    /// pending despawn - probed by sabotage during the task: the plain
    /// commands produced NO warn in this exact rig, refuting the assumed
    /// race (and when the camera despawns first, the `Single` fails and
    /// the observer skips). The plain commands in the observer are
    /// therefore correct; if bevy ever moves observer commands behind the
    /// pending queue (breadth-first), this test fails with "Entity
    /// despawned" and the observer needs try_ variants.
    #[test]
    fn a_player_ships_despawn_does_not_race_the_cameras() {
        use bevy::log::tracing_subscriber::{self, util::SubscriberInitExt};

        let log = CapturedLog::default();
        let writer = log.clone();
        let _guard = tracing_subscriber::fmt()
            .with_writer(move || writer.clone())
            .set_default();

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_observer(on_player_spaceship_spawned);
        app.add_observer(on_player_spaceship_destroyed);

        // Delivery guard 1: the capture sees this warn class.
        let stale = app.world_mut().spawn_empty().id();
        app.world_mut().entity_mut(stale).despawn();
        app.world_mut()
            .commands()
            .entity(stale)
            .remove::<SpaceshipCameraController>();
        app.update();
        assert!(
            log.contents().contains("Entity despawned"),
            "the log capture must see a deliberate stale-command warn; got: {}",
            log.contents()
        );

        // A WASD camera hands control to the ship camera on player spawn
        // (delivery guard 2: both observers demonstrably rewire a LIVE
        // camera).
        let camera = app
            .world_mut()
            .spawn((ScenarioCameraMarker, WASDCameraController))
            .id();
        app.update();
        let ship = app.world_mut().spawn(PlayerSpaceshipMarker).id();
        app.update();
        assert!(
            app.world()
                .get::<SpaceshipCameraController>(camera)
                .is_some(),
            "player spawn hands the camera to the ship controller"
        );

        // The race: sweep-style queued despawns, ship first, camera second.
        log.clear();
        app.world_mut().commands().entity(ship).despawn();
        app.world_mut().commands().entity(camera).despawn();
        app.update();
        assert!(
            !log.contents().contains("Entity despawned"),
            "teardown must not race stale camera commands; got: {}",
            log.contents()
        );
    }

    fn spawn_object_action() -> EventActionConfig {
        EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "obj".to_string(),
                name: "Obj".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                radius: 1.0,
                texture: Handle::default(),
                health: 1.0,
                surface_gravity: None,
                invulnerable: false,
                lock_signature: None,
            }),
        })
    }

    fn event_with(actions: Vec<EventActionConfig>) -> ScenarioEventConfig {
        ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions,
        }
    }

    fn scenario_with(id: &str, events: Vec<ScenarioEventConfig>) -> ScenarioConfig {
        ScenarioConfig {
            id: id.to_string(),
            name: "Test Scenario".to_string(),
            description: "For tests".to_string(),
            cubemap: Handle::default(),
            events,
        }
    }

    /// Scenario teardown clears the HUD hint emphasis: a leaked emphasis
    /// would pulse a verb row into the next scenario with nothing left to
    /// clear it (the state-reset class of 20260712-125342). Driven through
    /// the real UnloadScenario observer; scoped entities and the event
    /// world are cleared by the same helper, so the emphasis assert is the
    /// new behavior under test.
    #[test]
    fn teardown_clears_hint_emphasis() {
        use nova_gameplay::prelude::HintEmphasis;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<CurrentScenario>();
        app.init_resource::<HintEmphasis>();
        app.add_observer(unload_scenario);

        app.world_mut().resource_mut::<HintEmphasis>().set("GOTO");
        assert!(
            app.world().resource::<HintEmphasis>().contains("GOTO"),
            "delivery guard: the emphasis is set before teardown"
        );

        app.world_mut().trigger(UnloadScenario);
        app.update();

        assert!(
            app.world().resource::<HintEmphasis>().is_empty(),
            "unloading the scenario drops every emphasis"
        );
    }

    /// The OnUpdate pulse: fires every frame while a scenario is live and
    /// stays silent otherwise. Proven through a real OnUpdate handler
    /// mutating a variable - a handler that could not fire without the
    /// pulse (OnUpdate was dead config before task 20260711-180506).
    #[test]
    fn on_update_pulses_only_while_a_scenario_is_live() {
        use bevy_common_systems::prelude::{EventHandler, GameEventsPlugin, GameObjectives};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.init_resource::<CurrentScenario>();
        app.add_systems(Update, fire_on_update.run_if(scenario_is_live));

        let mut handler = EventHandler::<NovaEventWorld>::from(EventConfig::OnUpdate);
        handler.add_action(EventActionConfig::VariableSet(VariableSetActionConfig {
            key: "pulsed".to_string(),
            expression: VariableExpressionNode::new_term(VariableTermNode::new_factor(
                VariableFactorNode::new_literal(VariableLiteral::Boolean(true)),
            )),
        }));
        app.world_mut().spawn(handler);

        // No scenario: silent.
        app.update();
        app.update();
        assert!(
            app.world()
                .resource::<NovaEventWorld>()
                .get_variable("pulsed")
                .is_none(),
            "no pulse without a live scenario"
        );

        // Scenario live: the handler fires within a frame or two.
        app.insert_resource(CurrentScenario(Some(scenario_with("live", vec![]))));
        app.update();
        app.update();
        assert_eq!(
            app.world()
                .resource::<NovaEventWorld>()
                .get_variable("pulsed"),
            Some(&VariableLiteral::Boolean(true)),
            "a live scenario pulses OnUpdate handlers"
        );
    }

    /// The orbit-hold tracker: an engaged ORBIT fires OnOrbit once per
    /// HOLD WINDOW - never before the window, never per frame, and the
    /// window recurs while the hold continues (a single-shot event
    /// consumed under a rejecting beat guard would soft-lock any scenario
    /// whose beat advances mid-orbit; review R1.1). Driven through the
    /// real event pipeline into a real handler counting into a scenario
    /// variable.
    #[test]
    fn orbit_hold_fires_once_per_window_and_recurs() {
        use core::time::Duration;

        use bevy::time::TimeUpdateStrategy;
        use bevy_common_systems::prelude::{EventHandler, GameEventsPlugin, GameObjectives};
        use nova_gameplay::prelude::{Autopilot, AutopilotAction, SpaceshipRootMarker};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        // 0.2s steps: Time<Virtual> clamps any single delta at its
        // default max_delta of 0.25s, so bigger manual steps silently
        // accumulate slower than wall time.
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            0.2,
        )));
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.insert_resource(CurrentScenario(Some(scenario_with("live", vec![]))));
        app.add_systems(Update, track_orbit_holds.run_if(scenario_is_live));

        // The counting handler: orbits = orbits + 1 on every OnOrbit
        // under the well's id.
        let mut handler = EventHandler::<NovaEventWorld>::from(EventConfig::OnOrbit);
        handler.add_filter(EventFilterConfig::Entity(EntityFilterConfig {
            id: Some("planetoid".to_string()),
            other_id: Some("player_spaceship".to_string()),
            ..default()
        }));
        handler.add_action(EventActionConfig::VariableSet(VariableSetActionConfig {
            key: "orbits".to_string(),
            expression: VariableExpressionNode::new_add(
                VariableTermNode::new_factor(VariableFactorNode::new_name("orbits".to_string())),
                VariableExpressionNode::new_term(VariableTermNode::new_factor(
                    VariableFactorNode::new_literal(VariableLiteral::Number(1.0)),
                )),
            ),
        }));
        app.world_mut().spawn(handler);
        app.world_mut()
            .resource_mut::<NovaEventWorld>()
            .insert_variable("orbits".to_string(), VariableLiteral::Number(0.0));

        let orbits = |app: &App| -> f64 {
            match app
                .world()
                .resource::<NovaEventWorld>()
                .get_variable("orbits")
            {
                Some(VariableLiteral::Number(n)) => *n,
                other => panic!("orbits variable missing: {:?}", other),
            }
        };

        let well = app
            .world_mut()
            .spawn(EntityId::new("planetoid".to_string()))
            .id();
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                EntityId::new("player_spaceship".to_string()),
                EntityTypeName::new("spaceship".to_string()),
                Autopilot::engage(AutopilotAction::Orbit { well, plan: None }),
            ))
            .id();

        // ~2 seconds of hold (10 frames at 0.2s): under the 5s window.
        for _ in 0..10 {
            app.update();
        }
        assert_eq!(orbits(&app), 0.0, "no fire before the hold window");

        // Push just past the window: exactly one fire, not one per frame.
        // (~1.8s held so far; 18 frames = 3.6s more puts the total at
        // ~5.4s, 0.4s into the next window.)
        for _ in 0..18 {
            app.update();
        }
        assert_eq!(orbits(&app), 1.0, "one fire per window, not per frame");

        // Keep holding through a second full window: the event RECURS
        // during one continuous engagement - this is what saves a beat
        // that advances mid-orbit from a consumed one-shot (R1.1, and the
        // assertion round 2 claimed to add but did not, R2.2).
        for _ in 0..25 {
            app.update();
        }
        assert_eq!(
            orbits(&app),
            2.0,
            "a continued hold fires again next window"
        );

        // Disengage, re-engage: the clock restarts from zero and the next
        // window fires again.
        app.world_mut().entity_mut(ship).remove::<Autopilot>();
        app.update();
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Orbit {
                well,
                plan: None,
            }));
        for _ in 0..30 {
            app.update();
        }
        assert_eq!(
            orbits(&app),
            3.0,
            "a fresh engagement fires on a fresh clock"
        );
    }

    // -- the player-lock bridge (beat-sheet-v2 spike 20260713-140742) --

    #[test]
    fn a_lock_slot_fires_on_acquisition_then_echoes_per_window() {
        let a = Entity::from_raw_u32(1).unwrap();
        let b = Entity::from_raw_u32(2).unwrap();
        let mut state = None;

        // Acquisition fires immediately.
        assert_eq!(tick_lock_slot(&mut state, Some(a), 0.1), Some(a));
        // Held: quiet until the echo window elapses, then one re-fire.
        assert_eq!(tick_lock_slot(&mut state, Some(a), 2.0), None);
        assert_eq!(tick_lock_slot(&mut state, Some(a), 2.0), None);
        assert_eq!(
            tick_lock_slot(&mut state, Some(a), 2.0),
            Some(a),
            "a held lock echoes once per window (the anti-soft-lock recurrence)"
        );
        assert_eq!(tick_lock_slot(&mut state, Some(a), 2.0), None);
        // A live-radar retarget is a fresh acquisition on a fresh clock.
        assert_eq!(tick_lock_slot(&mut state, Some(b), 0.1), Some(b));
        assert_eq!(tick_lock_slot(&mut state, Some(b), 2.0), None);
        // Clearing re-arms: the next lock is an acquisition again.
        assert_eq!(tick_lock_slot(&mut state, None, 0.1), None);
        assert_eq!(tick_lock_slot(&mut state, Some(b), 0.1), Some(b));
    }

    /// The bridge end to end through the real event pipeline: a travel
    /// lock ticks a travel handler, a combat lock a combat handler, an AI
    /// ship's combat lock ticks NOTHING, and a target without a scenario
    /// id is quiet (delivery-guarded by the fires before it).
    #[test]
    fn player_locks_fire_their_events_and_ai_locks_never_do() {
        use core::time::Duration;

        use bevy::time::TimeUpdateStrategy;
        use bevy_common_systems::prelude::{EventHandler, GameEventsPlugin, GameObjectives};
        use nova_gameplay::prelude::{
            CombatLock, PlayerSpaceshipMarker, SpaceshipRootMarker, TravelLock,
        };

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            0.2,
        )));
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.insert_resource(CurrentScenario(Some(scenario_with("live", vec![]))));
        app.add_systems(Update, track_player_locks.run_if(scenario_is_live));

        // Counting handlers: one per slot, filtered on the beacon's id.
        let count_into = |key: &str| -> EventActionConfig {
            EventActionConfig::VariableSet(VariableSetActionConfig {
                key: key.to_string(),
                expression: VariableExpressionNode::new_add(
                    VariableTermNode::new_factor(VariableFactorNode::new_name(key.to_string())),
                    VariableExpressionNode::new_term(VariableTermNode::new_factor(
                        VariableFactorNode::new_literal(VariableLiteral::Number(1.0)),
                    )),
                ),
            })
        };
        for (config, key) in [
            (EventConfig::OnTravelLock, "travel_locks"),
            (EventConfig::OnCombatLock, "combat_locks"),
        ] {
            let mut handler = EventHandler::<NovaEventWorld>::from(config);
            handler.add_filter(EventFilterConfig::Entity(EntityFilterConfig {
                id: Some("beacon_3".to_string()),
                other_id: Some("player_spaceship".to_string()),
                ..default()
            }));
            handler.add_action(count_into(key));
            app.world_mut().spawn(handler);
            app.world_mut()
                .resource_mut::<NovaEventWorld>()
                .insert_variable(key.to_string(), VariableLiteral::Number(0.0));
        }
        let count = |app: &App, key: &str| -> f64 {
            match app.world().resource::<NovaEventWorld>().get_variable(key) {
                Some(VariableLiteral::Number(n)) => *n,
                other => panic!("{key} variable missing: {:?}", other),
            }
        };

        let beacon = app
            .world_mut()
            .spawn(EntityId::new("beacon_3".to_string()))
            .id();
        let unnamed = app.world_mut().spawn_empty().id();
        let player = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                EntityId::new("player_spaceship".to_string()),
                EntityTypeName::new("spaceship".to_string()),
                TravelLock(None),
                CombatLock(None),
            ))
            .id();
        // An AI ship with a combat lock on the SAME beacon (the combat
        // mirror writes these constantly): must never fire.
        app.world_mut().spawn((
            SpaceshipRootMarker,
            EntityId::new("scavenger".to_string()),
            EntityTypeName::new("spaceship".to_string()),
            TravelLock(None),
            CombatLock(Some(beacon)),
        ));
        // Arm the echo bookkeeping (first frame inserts it).
        app.update();
        app.update();

        // Travel acquisition: one travel fire, no combat fire.
        app.world_mut().get_mut::<TravelLock>(player).unwrap().0 = Some(beacon);
        app.update();
        app.update();
        assert_eq!(count(&app, "travel_locks"), 1.0, "travel lock ticks");
        assert_eq!(count(&app, "combat_locks"), 0.0);

        // Combat acquisition on the same target: the combat handler ticks.
        app.world_mut().get_mut::<CombatLock>(player).unwrap().0 = Some(beacon);
        app.update();
        app.update();
        assert_eq!(count(&app, "combat_locks"), 1.0, "combat lock ticks");

        // Holding both under the echo window: quiet (once per acquisition).
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(count(&app, "travel_locks"), 1.0);
        assert_eq!(count(&app, "combat_locks"), 1.0);

        // A target with no scenario id: quiet (the fires above are the
        // delivery guard that the pipeline works).
        app.world_mut().get_mut::<TravelLock>(player).unwrap().0 = Some(unnamed);
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(
            count(&app, "travel_locks"),
            1.0,
            "an id-less target fires nothing"
        );

        // The AI ship's lock sat on beacon_3 the whole test: still zero
        // fires beyond the player's own (the player-scope pin).
        assert_eq!(count(&app, "combat_locks"), 1.0, "AI locks never fire");
    }

    #[test]
    fn snapshot_reports_id_and_handler_count() {
        // One ScenarioLoaded handler_count per event, regardless of the actions inside.
        let scenario = scenario_with(
            "asteroid_field",
            vec![event_with(vec![]), event_with(vec![]), event_with(vec![])],
        );

        let loaded = ScenarioLoaded::from_config(&scenario);

        assert_eq!(loaded.scenario_id, "asteroid_field");
        assert_eq!(loaded.handler_count, 3);
    }

    #[test]
    fn snapshot_counts_spawn_object_actions_across_events() {
        // object_count counts SpawnScenarioObject actions everywhere, and ignores other
        // action kinds (here a bare DebugMessage-free event and a mixed one).
        let scenario = scenario_with(
            "mixed",
            vec![
                event_with(vec![spawn_object_action(), spawn_object_action()]),
                event_with(vec![]),
                event_with(vec![spawn_object_action()]),
            ],
        );

        let loaded = ScenarioLoaded::from_config(&scenario);

        assert_eq!(loaded.handler_count, 3);
        assert_eq!(loaded.object_count, 3);
    }

    #[test]
    fn empty_scenario_reports_zero_counts() {
        let loaded = ScenarioLoaded::from_config(&scenario_with("empty", vec![]));

        assert_eq!(loaded.scenario_id, "empty");
        assert_eq!(loaded.handler_count, 0);
        assert_eq!(loaded.object_count, 0);
    }

    /// Ticks counted per gated set, so each probe proves its own schedule's
    /// gate (task 20260711-212519).
    #[derive(Resource, Default)]
    struct Ticks {
        input: u32,
        sections: u32,
        sections_fixed: u32,
    }

    /// A minimal app with the PRODUCTION gating wiring
    /// (configure_scenario_gating) and one probe system in each gated set.
    fn gated_app() -> App {
        let mut app = App::new();
        app.init_resource::<CurrentScenario>();
        app.init_resource::<Ticks>();
        configure_scenario_gating(&mut app);
        app.add_systems(
            Update,
            (|mut t: ResMut<Ticks>| t.input += 1).in_set(SpaceshipInputSystems),
        );
        app.add_systems(
            Update,
            (|mut t: ResMut<Ticks>| t.sections += 1).in_set(SpaceshipSectionSystems),
        );
        app.add_systems(
            FixedUpdate,
            (|mut t: ResMut<Ticks>| t.sections_fixed += 1).in_set(SpaceshipSectionSystems),
        );
        app
    }

    /// One Update pass plus one manual FixedUpdate pass (headless apps have
    /// no time accumulation to drive FixedUpdate on its own).
    fn step(app: &mut App) {
        app.update();
        app.world_mut().run_schedule(FixedUpdate);
    }

    fn ticks(app: &App) -> (u32, u32, u32) {
        let t = app.world().resource::<Ticks>();
        (t.input, t.sections, t.sections_fixed)
    }

    /// The spaceship sets run exactly while a scenario is live. The live
    /// phase in the middle is the delivery guard for the two frozen phases:
    /// the same probes demonstrably CAN run in this app.
    #[test]
    fn spaceship_sets_run_only_while_a_scenario_is_live() {
        let mut app = gated_app();

        step(&mut app);
        assert_eq!(ticks(&app), (0, 0, 0), "no scenario: all sets frozen");

        app.world_mut()
            .resource_mut::<CurrentScenario>()
            .replace(scenario_with("live", vec![]));
        step(&mut app);
        assert_eq!(ticks(&app), (1, 1, 1), "live scenario: all sets run");

        app.world_mut().resource_mut::<CurrentScenario>().take();
        step(&mut app);
        assert_eq!(ticks(&app), (1, 1, 1), "unloaded again: all sets frozen");
    }

    /// The same gate driven through the real load/unload observers, so the
    /// whole delivery chain is covered: LoadScenario -> CurrentScenario ->
    /// sets run; UnloadScenario -> sets freeze.
    #[test]
    fn load_and_unload_scenario_drive_the_gate() {
        let mut app = gated_app();
        app.init_resource::<NovaEventWorld>();
        app.add_observer(on_load_scenario);
        app.add_observer(unload_scenario);

        step(&mut app);
        assert_eq!(ticks(&app), (0, 0, 0), "nothing loaded yet");

        app.world_mut()
            .trigger(LoadScenario(scenario_with("ambience", vec![])));
        assert!(
            app.world().resource::<CurrentScenario>().is_some(),
            "LoadScenario must set CurrentScenario"
        );
        step(&mut app);
        assert_eq!(ticks(&app), (1, 1, 1), "loaded: sets run");

        app.world_mut().trigger(UnloadScenario);
        assert!(
            app.world().resource::<CurrentScenario>().is_none(),
            "UnloadScenario must clear CurrentScenario"
        );
        step(&mut app);
        assert_eq!(ticks(&app), (1, 1, 1), "unloaded: sets frozen again");
    }
}
