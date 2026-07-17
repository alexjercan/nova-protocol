/// Scenario loader plugin and related types
use bevy::{platform::collections::HashMap, prelude::*};
use bevy_common_systems::prelude::*;
use bevy_enhanced_input::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;

use crate::prelude::*;

pub mod prelude {
    pub use super::{
        scenario_is_live, ContentIssues, CurrentScenario, GameScenarios, LoadScenario,
        NewGameStart, ScenarioCameraMarker, ScenarioConfig, ScenarioEventConfig, ScenarioId,
        ScenarioLoaded, ScenarioLoaderPlugin, ScenarioScopedMarker, ScenarioStartFailure,
        ScenarioStartFailureReport, ScriptedCameraPose, UnloadScenario, SCENARIO_ELAPSED_VAR,
    };
}

/// Type alias for Scenario ID
pub type ScenarioId = String;

/// The collection of available game scenarios
#[derive(Resource, Clone, Debug, Deref, DerefMut, Default)]
pub struct GameScenarios(pub HashMap<ScenarioId, ScenarioConfig>);

/// The scenario New Game launches, declared by the BASE bundle's manifest
/// (`new_game_scenario` in `base.bundle.ron`) and written by the bundle merge
/// (task 20260716-155849). Deliberately NOT a scenario flag and NOT
/// overlayable: only the catalog entry marked `base: true` is honored, so a
/// non-base mod can never redirect what New Game starts (mods add PICKER
/// entries and `menu_backdrop` scenarios instead). `None` when base declares
/// nothing; the menu then falls back to the first listed scenario.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct NewGameStart(pub Option<ScenarioId>);

/// Lint findings per registered scenario, written by the bundle merge
/// against the MERGED registries (task 20260716-193949) - the runtime half
/// of the content gate (`nova_scenario::lint` is the shared core). Keyed by
/// scenario id; scenarios without findings have no entry. Error-level
/// findings make [`on_load_scenario`] REFUSE the load.
#[derive(Resource, Clone, Debug, Default)]
pub struct ContentIssues(pub HashMap<ScenarioId, Vec<LintIssue>>);

impl ContentIssues {
    /// The Error-level findings for `id` (empty = startable).
    pub fn errors(&self, id: &str) -> Vec<&LintIssue> {
        self.0
            .get(id)
            .map(|issues| {
                issues
                    .iter()
                    .filter(|i| i.severity == LintSeverity::Error)
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Set when a scenario REFUSED to start (Error-level content issues): the
/// Wesnoth-style player-facing report ("Failed to start 'X': ..."), rendered
/// by the menu's FAILED TO START overlay. Cleared on menu entry.
#[derive(Resource, Clone, Debug, Default)]
pub struct ScenarioStartFailure(pub Option<ScenarioStartFailureReport>);

/// What the failure overlay shows: the scenario's display name and one line
/// per Error-level finding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenarioStartFailureReport {
    pub scenario_name: String,
    pub messages: Vec<String>,
}

/// Configuration for a game scenario.
///
/// `Default` exists so the many code-built literals (examples, tests) can fill
/// the optional `thumbnail`/`hidden` fields with `..Default::default()`. Note a
/// FULLY default `ScenarioConfig` is not serializable: its default `cubemap` is
/// a handle-backed `AssetRef`, which errors on serialize (see `AssetRef`). Every
/// real builder sets `cubemap` to a path, so this never bites; do not serialize
/// `ScenarioConfig::default()` directly.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ScenarioConfig {
    /// Unique identifier for the scenario
    pub id: ScenarioId,
    /// The display name of the scenario
    pub name: String,
    /// A brief description of the scenario
    pub description: String,
    /// The cubemap image used for the scenario's skybox. Authored as an asset
    /// path; resolved to a live handle at load time (see `on_load_scenario`).
    pub cubemap: AssetRef<Image>,
    /// An optional thumbnail image for menus (the Scenarios picker renders it in
    /// the details pane). Authored as an asset path exactly like `cubemap`, so a
    /// mod thumbnail gets the same path handling. Serde-defaulted, so scenarios
    /// authored before this field still parse. In strict RON an `Option` is
    /// written with the variant, never bare: `thumbnail: Some("banner.png")`.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub thumbnail: Option<AssetRef<Image>>,
    /// When true the scenario is hidden from the Scenarios picker (backdrops
    /// like `menu_ambience`, mid-story continuations reached only via
    /// `NextScenario` chaining). Mirrors the mods-catalog `hidden` flag.
    /// Serde-defaulted to false, so most scenarios omit it; author a hidden one
    /// as `hidden: true`.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "is_false"))]
    pub hidden: bool,
    /// When true the scenario is a MENU BACKDROP candidate: on menu entry the
    /// menu collects every registered scenario with this flag and loads one at
    /// random, so several ambience scenes can ship and mods can add their own.
    /// Backdrops normally also set `hidden: true` (the flags are orthogonal -
    /// this one opts INTO the menu rotation, `hidden` opts OUT of the picker).
    /// A backdrop should contain a gravity well with entity id
    /// `menu_planetoid` for the cinematic camera framing; without one the menu
    /// falls back to the scenario's own camera pose after a short grace.
    /// Serde-defaulted to false; author as `menu_backdrop: true`.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "is_false"))]
    pub menu_backdrop: bool,
    /// Events associated with the scenario
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub events: Vec<ScenarioEventConfig>,
}

/// `skip_serializing_if` predicate for a `bool` that defaults to false: omit it
/// from the serialized RON when it is false so unflagged scenarios stay clean.
#[cfg(feature = "serde")]
fn is_false(b: &bool) -> bool {
    !*b
}

/// Configuration for a scenario event
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ScenarioEventConfig {
    /// The name of the event to listen for
    pub name: EventConfig,
    /// Filters to apply to the event
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub filters: Vec<EventFilterConfig>,
    /// Actions to perform when the event is triggered
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
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
        app.init_resource::<CurrentOutcome>();
        // Default None until the bundle merge writes the base declaration, so
        // a `Res<NewGameStart>` never panics on ordering.
        app.init_resource::<NewGameStart>();
        // The runtime content gate's two channels (empty until the merge
        // writes findings / a load refuses).
        app.init_resource::<ContentIssues>();
        app.init_resource::<ScenarioStartFailure>();
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
        // scenario is live - same liveness rule as the ship systems - AND
        // only while Unpaused (task 20260716-231855): unlike the trackers
        // below, this pulse fires UNCONDITIONALLY every frame, so under a
        // pause (the ESC menu OR the outcome frame, both PauseStates::Paused)
        // an OnUpdate handler whose predicate is already true would re-run
        // its action every frame via the pause-independent PostUpdate
        // state_to_world sync. The sibling trackers below stay ungated on
        // pause, but they no longer need to be: they DERIVE their 5s windows
        // from `scenario_elapsed` (task 20260717-151537), the same clock the
        // pulse rides, so a paused frame reads a frozen clock and no window
        // can advance - nothing NEWLY fires under pause by construction, not
        // by the old "Time<Virtual> delta ~= 0" assumption this comment used
        // to lean on. apply_pending_skybox_swaps stays ungated too: it is
        // cosmetic and asset-load driven (not clock driven), and letting a
        // queued swap finish under pause is harmless. Do not blanket-gate.
        //
        // The scenario clock ticks chained ahead of the pulse under the
        // SAME gate: time-gated handlers see this frame's clock, and both
        // freeze together under a pause (task 20260717-112647).
        register_clock_and_pulse(app);

        // The orbit-hold tracker behind `EventConfig::OnOrbit` (task
        // 20260712-110730 finding 5): "in orbit" is autopilot state, not
        // a position - a gate area is unwinnable because the ORBIT verb
        // rings at max(clearance band, engage radius). Ordered `.after` the
        // clock tick so it reads THIS frame's `scenario_elapsed`; the tick is
        // Unpaused-gated while the tracker is not, so `.after` is a pure
        // ordering constraint - when the tick is skipped under pause the
        // tracker still runs and reads the last (frozen) clock value.
        app.register_type::<OrbitHold>();
        app.register_type::<OrbitHoldSecs>();
        app.add_systems(
            Update,
            track_orbit_holds
                .after(tick_scenario_clock)
                .run_if(scenario_is_live),
        );

        // The player-lock bridge behind `EventConfig::OnTravelLock` /
        // `OnCombatLock` (beat-sheet-v2 spike 20260713-140742): lock
        // lessons tick the instant the lock lands. Same clock derivation and
        // `.after(tick_scenario_clock)` ordering as the orbit tracker.
        app.register_type::<LockEcho>();
        app.register_type::<LockRefireSecs>();
        app.add_systems(
            Update,
            track_player_locks
                .after(tick_scenario_clock)
                .run_if(scenario_is_live),
        );

        // Deferred skybox swap behind `EventActionConfig::SetSkybox` (task
        // 20260525-133017): the action tags the scenario camera with a
        // `PendingSkyboxSwap`, and this applier installs the `SkyboxConfig` once
        // the new cubemap image has loaded - the setup observer panics on an
        // unloaded image, so the insert cannot happen synchronously in the action.
        app.register_type::<PendingSkyboxSwap>();
        app.add_systems(Update, apply_pending_skybox_swaps.run_if(scenario_is_live));

        // Scripted-camera override (photo mode / the screenshot reel): the
        // `SetCamera` action pins a `ScriptedCameraPose` on the scenario camera;
        // enforce it in PostUpdate AFTER the WASD sync so the scripted pose wins
        // the last write before render. The free-fly controller's state machine
        // keeps writing the camera Transform every frame (and removing the
        // controller does not stop it - the private state components survive), so
        // a one-shot Transform set would be immediately overwritten; running last
        // is what makes the pose stick.
        app.add_systems(
            PostUpdate,
            enforce_scripted_camera_pose.after(WASDCameraSystems::Sync),
        );
    }
}

/// A scripted camera pose that overrides the free-fly WASD controller, applied
/// every frame by [`enforce_scripted_camera_pose`]. Set by the `SetCamera`
/// scenario action (photo mode) and the screenshot reel; while present it pins
/// the [`ScenarioCameraMarker`] camera at `position` looking at `look_at`.
#[derive(Component, Debug, Clone, Copy)]
pub struct ScriptedCameraPose {
    /// World-space camera position.
    pub position: Vec3,
    /// World-space point the camera looks at (up is +Y).
    pub look_at: Vec3,
}

/// Pin every camera carrying a [`ScriptedCameraPose`] to that pose. Ordered
/// after `WASDCameraSystems::Sync` so it wins the frame's last write to the
/// camera Transform.
fn enforce_scripted_camera_pose(mut cameras: Query<(&mut Transform, &ScriptedCameraPose)>) {
    for (mut transform, pose) in &mut cameras {
        *transform = Transform::from_translation(pose.position).looking_at(pose.look_at, Vec3::Y);
    }
}

/// The reserved scenario-clock variable (task 20260717-112647): seconds of
/// LIVE, UNPAUSED scenario time, maintained by [`tick_scenario_clock`] and
/// readable from any expression filter as
/// `Term(Factor(Name("scenario_elapsed")))`. Authors GATE on it, they never
/// write it - a `VariableSet` on this key is a content_lint ERROR, because
/// the engine overwrites it every tick. It clears with the rest of the
/// event world at teardown, so it is the CURRENT scenario's clock (a retry
/// restarts it), and an early read before the first tick fails closed via
/// the undefined-variable rule.
///
/// One-shots compose with the standard act/flag gate: `elapsed > N` plus
/// an act filter, then advance the act. Repeating waves compose with a
/// rearm write: gate on `elapsed > next_at`, then
/// `VariableSet(next_at, Add(next_at, interval))`.
pub const SCENARIO_ELAPSED_VAR: &str = "scenario_elapsed";

/// Accumulate the scenario clock. Registered CHAINED AHEAD of
/// [`fire_on_update`] under the same live+unpaused gate, so the pulse that
/// evaluates time-gated handlers always sees this frame's clock; pausing
/// (ESC menu or the outcome frame) freezes the clock by construction.
fn tick_scenario_clock(time: Res<Time>, mut world: ResMut<NovaEventWorld>) {
    let elapsed = scenario_elapsed(&world);
    world.insert_variable(
        SCENARIO_ELAPSED_VAR.to_string(),
        VariableLiteral::Number(elapsed + time.delta_secs_f64()),
    );
}

/// Read the current scenario clock (seconds of live-unpaused time) off the
/// event world, with the same `None -> 0.0` fallback as [`tick_scenario_clock`]
/// so a read before the first tick (or after teardown's `world.clear()`) sees a
/// fresh clock. The clock-derived trackers below ([`track_orbit_holds`],
/// [`track_player_locks`]) measure their 5s windows against this instead of
/// accumulating their own `Time` delta, so pausing and teardown/retry freeze
/// and reset every window in one place (task 20260717-151537).
fn scenario_elapsed(world: &NovaEventWorld) -> f64 {
    match world.get_variable(SCENARIO_ELAPSED_VAR) {
        Some(VariableLiteral::Number(n)) => *n,
        _ => 0.0,
    }
}

/// The ONE registration of the clock + pulse pair, shared by the plugin
/// and the test rigs so the load-bearing chain + gate cannot drift between
/// them (review 20260717-112647 R1.2): tick first, pulse second, both
/// gated live + Unpaused.
fn register_clock_and_pulse(app: &mut App) {
    app.add_systems(
        Update,
        (tick_scenario_clock, fire_on_update)
            .chain()
            .run_if(scenario_is_live.and_then(in_state(PauseStates::Unpaused))),
    );
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
const ORBIT_HOLD_SECS: f64 = 5.0;

/// Resolve an author-supplied event-window override against the engine default.
/// A non-finite or non-positive override is rejected (content_lint errors on
/// it), so at runtime we fail closed to `default` rather than ever produce a
/// zero/negative window that would fire every frame. Task 20260717-165031.
fn resolve_window_secs(override_secs: Option<f64>, default: f64) -> f64 {
    match override_secs {
        Some(secs) if secs.is_finite() && secs > 0.0 => secs,
        _ => default,
    }
}

/// Bookkeeping for the orbit-hold tracker, on the orbiting ship: which well
/// and the scenario-clock reading ([`scenario_elapsed`]) when the current
/// window opened - engagement, well switch, or the last fire. The window has
/// elapsed once `now - started_at >= window`, where `window` is the ship's
/// [`OrbitHoldSecs`] override or the [`ORBIT_HOLD_SECS`] default. Disengaging (or
/// switching wells) removes it, restarting the window; the component also dies
/// with its entity on teardown, so a retry re-arms against a fresh clock.
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct OrbitHold {
    pub well: Entity,
    pub started_at: f64,
}

/// Fire [`OnOrbitEvent`] once a ship has HELD an engaged
/// `Autopilot { action: Orbit { well } }` for [`ORBIT_HOLD_SECS`]
/// continuously. Ships are identified by their scenario `EntityId` (ships
/// without one - editor previews - are invisible to the tracker); the
/// event's `id` is the WELL's scenario id, mirroring OnEnter's
/// (area, other) shape so filters compose identically. The hold window is
/// measured against the engine scenario clock ([`scenario_elapsed`]) rather
/// than an accumulated `Time` delta, so it freezes under pause and resets on
/// teardown/retry with the clock itself (task 20260717-151537).
fn track_orbit_holds(
    world: Res<NovaEventWorld>,
    mut commands: Commands,
    mut q_ships: Query<
        (
            Entity,
            &Autopilot,
            Option<&mut OrbitHold>,
            Option<&OrbitHoldSecs>,
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

    let now = scenario_elapsed(&world);

    for (ship, autopilot, hold, hold_override, ship_id, ship_type_name) in &mut q_ships {
        let AutopilotAction::Orbit { well, .. } = autopilot.action else {
            // Engaged, but not an orbit (GOTO/STOP): no hold.
            if hold.is_some() {
                commands.entity(ship).remove::<OrbitHold>();
            }
            continue;
        };

        // Per-ship override (AIControllerConfig::orbit_hold_secs), else the
        // engine default. Task 20260717-165031.
        let window = resolve_window_secs(hold_override.map(|o| o.0), ORBIT_HOLD_SECS);

        match hold {
            Some(mut hold) if hold.well == well => {
                if now - hold.started_at >= window {
                    // Restart the window whether or not the event can be
                    // addressed (review R1.2: a well without a scenario id
                    // must not consume the hold - the next window retries).
                    hold.started_at = now;
                    let Ok(well_id) = q_ids.get(well) else {
                        // A well without a scenario id (despawned or
                        // non-scenario body) has no address to fire under.
                        continue;
                    };
                    debug!(
                        "track_orbit_holds: ship '{}' held orbit around '{}' for {}s",
                        ship_id.0, well_id.0, window
                    );
                    commands.fire::<OnOrbitEvent>(OnOrbitEventInfo {
                        id: well_id.0.clone(),
                        other_id: ship_id.0.clone(),
                        other_type_name: ship_type_name.0.clone(),
                    });
                }
            }
            // New engagement, or the directive switched wells: open a fresh
            // window on the current well, anchored at the current clock.
            _ => {
                commands.entity(ship).insert(OrbitHold {
                    well,
                    started_at: now,
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
const LOCK_REFIRE_SECS: f64 = 5.0;

/// Bookkeeping for the player-lock bridge: per slot, the last target the
/// bridge saw and the scenario-clock reading ([`scenario_elapsed`]) when it
/// last fired for that target. The re-fire window has elapsed once
/// `now - last_fired_at >= refire`, where `refire` is the player's
/// [`LockRefireSecs`] override or the [`LOCK_REFIRE_SECS`] default.
#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct LockEcho {
    travel: Option<(Entity, f64)>,
    combat: Option<(Entity, f64)>,
}

/// One lock slot's tick: returns `Some(target)` when the bridge should
/// fire this frame - on ACQUISITION (the slot's value changed onto a
/// target; the slot writers are equality-skipped, so a held live-radar
/// lock does not churn this) and again every `refire_secs` seconds while
/// the same target stays held. `now` is the engine scenario clock
/// ([`scenario_elapsed`]); the window is `now - last_fired_at`, so it freezes
/// under pause and resets on teardown with the clock. `refire_secs` is the
/// per-player override ([`LockRefireSecs`]) or the [`LOCK_REFIRE_SECS`] default,
/// resolved by the caller. Pure for the unit tests.
fn tick_lock_slot(
    state: &mut Option<(Entity, f64)>,
    current: Option<Entity>,
    now: f64,
    refire_secs: f64,
) -> Option<Entity> {
    match (current, state.as_mut()) {
        (None, _) => {
            *state = None;
            None
        }
        (Some(target), Some((held, last_fired_at))) if *held == target => {
            if now - *last_fired_at >= refire_secs {
                *last_fired_at = now;
                Some(target)
            } else {
                None
            }
        }
        (Some(target), _) => {
            *state = Some((target, now));
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
    world: Res<NovaEventWorld>,
    mut commands: Commands,
    mut q_ships: Query<
        (
            Entity,
            &TravelLock,
            &CombatLock,
            Option<&mut LockEcho>,
            Option<&LockRefireSecs>,
            &EntityId,
            &EntityTypeName,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    q_ids: Query<&EntityId>,
) {
    let now = scenario_elapsed(&world);
    for (ship, travel, combat, echo, refire_override, ship_id, ship_type_name) in &mut q_ships {
        let Some(mut echo) = echo else {
            // First sight of this player ship: arm the bookkeeping; the
            // next frame ticks it (an already-held lock then reads as an
            // acquisition, which is the honest interpretation on spawn).
            commands.entity(ship).insert(LockEcho::default());
            continue;
        };
        // Per-player override (PlayerControllerConfig::lock_refire_secs), else
        // the engine default. Task 20260717-165031.
        let refire = resolve_window_secs(refire_override.map(|o| o.0), LOCK_REFIRE_SECS);
        let fired_travel = tick_lock_slot(&mut echo.travel, travel.0, now, refire);
        let fired_combat = tick_lock_slot(&mut echo.combat, combat.0, now, refire);
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
    outcome: Option<&mut CurrentOutcome>,
    objectives: Option<&mut GameObjectives>,
    story_feed: Option<&mut StoryFeed>,
) {
    world.clear();
    // The objectives HUD mirror dies with the scenario too (same reset class as
    // the event world / emphasis / outcome above). The panel rides the player
    // ship, so it is despawned and rebuilt EMPTY on every (re)load, and bcs only
    // repaints it when `GameObjectives` is `resource_changed`. Without this reset,
    // restarting the SAME scenario re-posts identical objectives, the write-on-diff
    // sync (`state_to_world_system`) sees no change, the resource never re-flags,
    // and the fresh panel stays blank - the objective still works but its text is
    // gone (task 20260716-214338). Guarded so an already-empty mirror is not
    // spuriously re-flagged (mirrors the sync's write-on-diff discipline).
    if let Some(objectives) = objectives {
        if !objectives.objectives.is_empty() {
            objectives.objectives.clear();
        }
    }
    // The comms mirror dies with the scenario for the SAME reason (review
    // 20260717-163033 R1.1): the sync's write-on-diff is length-only, so a
    // retry whose reload pushes as many lines as the old scenario had
    // (clear + repush inside one sync window) would never rewrite the feed
    // and the new scenario's OPENING story line would silently vanish.
    // Clearing here makes the empty state observable to the panel's reset
    // before the next scenario's lines arrive. Guarded like the objectives.
    if let Some(story_feed) = story_feed {
        if !story_feed.0.is_empty() {
            story_feed.0.clear();
        }
    }
    // Scenario-driven HUD emphasis dies with the scenario: a leaked
    // emphasis would pulse a verb row into the next scenario (or the
    // menu's death gap) with nothing left to clear it - the same reset
    // class as the objectives diff (state-diff-aliases-reset,
    // 20260712-125342). Optional: headless rigs run without the HUD.
    if let Some(emphasis) = emphasis {
        emphasis.clear_all();
    }
    // The declared outcome dies with the scenario too (same reset class):
    // a leaked Victory/Defeat would re-show the overlay over the next
    // scenario or the menu. Optional for rigs without the loader resource.
    if let Some(outcome) = outcome {
        outcome.0 = None;
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
    mut outcome: Option<ResMut<CurrentOutcome>>,
    mut objectives: Option<ResMut<GameObjectives>>,
    mut story_feed: Option<ResMut<StoryFeed>>,
) {
    teardown_scenario_entities(
        &mut commands,
        &q_scoped,
        &mut world,
        emphasis.as_deref_mut(),
        outcome.as_deref_mut(),
        objectives.as_deref_mut(),
        story_feed.as_deref_mut(),
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
    mut outcome: Option<ResMut<CurrentOutcome>>,
    mut objectives: Option<ResMut<GameObjectives>>,
    mut story_feed: Option<ResMut<StoryFeed>>,
    asset_server: Res<AssetServer>,
    issues: Option<Res<ContentIssues>>,
    mut failure: Option<ResMut<ScenarioStartFailure>>,
) {
    // The runtime content gate (task 20260716-193949): a scenario with
    // Error-level findings REFUSES to start - better a clear failure than a
    // silently half-spawned scene. Checked BEFORE teardown so whatever was
    // on screen stays; the stale outcome overlay is cleared so the FAILED
    // TO START modal does not stack under it.
    if let Some(issues) = issues.as_ref() {
        let errors = issues.errors(&load.0.id);
        if !errors.is_empty() {
            error!(
                "on_load_scenario: refusing to start '{}' ({} content error(s)):",
                load.0.id,
                errors.len()
            );
            let mut messages = Vec::new();
            for issue in errors {
                error!("  {}", issue.message);
                messages.push(issue.message.clone());
            }
            if let Some(outcome) = outcome.as_deref_mut() {
                outcome.0 = None;
            }
            if let Some(failure) = failure.as_deref_mut() {
                failure.0 = Some(ScenarioStartFailureReport {
                    scenario_name: load.0.name.clone(),
                    messages,
                });
            }
            return;
        }
    }

    teardown_scenario_entities(
        &mut commands,
        &q_scoped,
        &mut world,
        emphasis.as_deref_mut(),
        outcome.as_deref_mut(),
        objectives.as_deref_mut(),
        story_feed.as_deref_mut(),
    );

    let scenario = (**load).clone();
    **current_scenario = Some(scenario.clone());
    debug!("on_load_scenario: scenario {:?}", scenario.name);

    // Setup Scenario Camera. `SfxListenerMarker` makes this the explicit
    // SFX/juice listener (attenuation, camera shake, flash facing); the editor
    // camera deliberately never carries it.
    //
    // The skybox goes on DEFERRED (PendingSkyboxSwap, the SetSkybox action's
    // applier): the bcs skybox setup observer reads the image out of
    // `Assets<Image>` the instant a `SkyboxConfig` lands and panics on a
    // not-yet-loaded handle. Preloaded cubemaps (the GameAssets set) apply on
    // the next frame; a cubemap the collection does NOT preload - broadside's
    // alt sky, any mod shipping its own - loads in and applies when ready
    // instead of crashing the load (found by example 19, task 20260708-203659).
    commands.spawn((
        ScenarioScopedMarker,
        ScenarioCameraMarker,
        SfxListenerMarker,
        Name::new("Scenario Camera"),
        Camera3d::default(),
        PostProcessingCamera,
        WASDCameraController,
        Transform::from_xyz(0.0, 10.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
        PendingSkyboxSwap {
            cubemap: scenario.cubemap.resolve(&asset_server),
            brightness: Some(1000.0),
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

/// What the scenario-advance input does, given the current state. Extracted
/// from the observer so the decision table is unit-testable (synthesizing a
/// bevy_enhanced_input `Start<>` trigger in a rig is not worth the harness).
#[derive(Debug, PartialEq, Eq)]
enum AdvanceDecision {
    /// A lingering `NextScenario` is queued: release it.
    ReleaseQueued,
    /// Nothing queued but an outcome is declared (victory at the end of
    /// content): return to the main menu.
    ExitToMenu,
    /// Nothing to advance; the key does nothing mid-scenario.
    Ignore,
}

fn decide_advance(paused: bool, has_queued: bool, has_outcome: bool) -> AdvanceDecision {
    // A shown outcome frame pauses the sim (task 20260716-214919), but its
    // [Enter] advance is exactly the input we want live under that pause -
    // the overlay is a paused modal with its own Continue/Retry route. A
    // plain pause-menu pause (no outcome) still swallows the key, so ESC
    // mid-scenario cannot skip ahead.
    if paused && !has_outcome {
        return AdvanceDecision::Ignore;
    }
    if has_queued {
        return AdvanceDecision::ReleaseQueued;
    }
    if has_outcome {
        return AdvanceDecision::ExitToMenu;
    }
    AdvanceDecision::Ignore
}

fn on_next_input(
    _: On<Start<NextScenarioInput>>,
    mut world: ResMut<super::world::NovaEventWorld>,
    pause: Res<State<PauseStates>>,
    outcome: Option<Res<CurrentOutcome>>,
    mut game_state: Option<ResMut<NextState<GameStates>>>,
) {
    // Observers bypass system-set gating; freeze intent changes while the
    // pause overlay is up (review R1.1). Releases stay ungated so held keys
    // clear cleanly during a pause.
    let paused = *pause.get() == PauseStates::Paused;
    let has_queued = world.next_scenario.is_some();
    let has_outcome = outcome.map(|o| o.0.is_some()).unwrap_or(false);

    match decide_advance(paused, has_queued, has_outcome) {
        AdvanceDecision::ReleaseQueued => {
            world.release_lingering_next();
        }
        AdvanceDecision::ExitToMenu => {
            // Optional: headless rigs without the states plugin have no
            // NextState resource, and the input must not panic there.
            if let Some(state) = game_state.as_deref_mut() {
                state.set(GameStates::MainMenu);
            }
        }
        AdvanceDecision::Ignore => {}
    }
}

/// Marks the scenario's free-fly camera (the one spawned by
/// [`on_load_scenario`], carrying [`WASDCameraController`] until a player ship
/// swaps it to the chase camera). The `SetCamera` scenario action
/// ([`SetCameraActionConfig`](crate::actions::SetCameraActionConfig)) queries
/// this to pose the camera for a scripted screenshot.
#[derive(Component, Debug, Clone)]
pub struct ScenarioCameraMarker;

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

    /// The runtime content gate (task 20260716-193949): an Error-flagged
    /// scenario REFUSES to start - nothing scenario-scoped spawns, the
    /// failure report is set for the overlay, and a stale outcome is
    /// cleared so its overlay cannot stack under the modal. The clean
    /// control proves the same rig DOES load without the flag
    /// (delivery guard).
    #[test]
    fn error_flagged_scenario_refuses_to_start() {
        let build_app = |issues: ContentIssues| {
            let mut app = App::new();
            app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()));
            app.init_asset::<Image>();
            app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
            app.init_resource::<NovaEventWorld>();
            app.init_resource::<CurrentScenario>();
            app.init_resource::<GameObjectives>();
            app.insert_resource(issues);
            app.init_resource::<ScenarioStartFailure>();
            app.insert_resource(CurrentOutcome(Some(OutcomeActionConfig {
                outcome: ScenarioOutcomeKind::Victory,
                message: None,
            })));
            app.add_observer(on_load_scenario);
            app
        };
        let scenario = ScenarioConfig {
            id: "broken".to_string(),
            name: "Broken Chapter".to_string(),
            description: "gate pin".to_string(),
            cubemap: AssetRef::from("textures/x.png".to_string()),
            events: vec![],
            ..Default::default()
        };

        // Control: no issues -> the scenario loads (scoped entities exist).
        let mut clean = build_app(ContentIssues::default());
        clean.world_mut().trigger(LoadScenario(scenario.clone()));
        clean.update();
        let loaded = clean
            .world_mut()
            .query_filtered::<(), With<ScenarioScopedMarker>>()
            .iter(clean.world())
            .count();
        assert!(
            loaded > 0,
            "delivery guard: the clean load spawns the scene"
        );

        // Gate: an Error finding refuses the load.
        let mut issues = ContentIssues::default();
        issues.0.insert(
            "broken".to_string(),
            vec![LintIssue {
                severity: LintSeverity::Error,
                scenario: "broken".to_string(),
                message: "unknown section prototype 'ghost_hull'".to_string(),
            }],
        );
        let mut app = build_app(issues);
        app.world_mut().trigger(LoadScenario(scenario));
        app.update();

        let spawned = app
            .world_mut()
            .query_filtered::<(), With<ScenarioScopedMarker>>()
            .iter(app.world())
            .count();
        assert_eq!(spawned, 0, "a refused start must spawn nothing");
        let failure = app.world().resource::<ScenarioStartFailure>();
        let report = failure.0.as_ref().expect("the refusal sets the report");
        assert_eq!(report.scenario_name, "Broken Chapter");
        assert!(report.messages[0].contains("ghost_hull"));
        assert!(
            app.world().resource::<CurrentOutcome>().0.is_none(),
            "a stale outcome is cleared so its overlay cannot stack"
        );
        assert!(
            app.world().resource::<CurrentScenario>().is_none(),
            "the refused scenario never becomes current"
        );
    }

    /// The loader's skybox install is DEFERRED (task 20260708-203659, found
    /// by example 19): an eager `SkyboxConfig` insert panics inside the bcs
    /// setup observer for any cubemap not already sitting in
    /// `Assets<Image>` - which is every non-preloaded scenario/mod sky. Pin
    /// the invariant at the loader's own boundary: after a load, the camera
    /// carries `PendingSkyboxSwap` and NOT `SkyboxConfig`.
    #[test]
    fn scenario_load_defers_the_skybox_install() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()));
        app.init_asset::<Image>();
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<CurrentScenario>();
        // state_to_world_system mirrors objectives unconditionally.
        app.init_resource::<GameObjectives>();
        app.add_observer(on_load_scenario);

        app.world_mut().trigger(LoadScenario(ScenarioConfig {
            id: "sky_test".to_string(),
            name: "Sky Test".to_string(),
            description: "deferred skybox pin".to_string(),
            cubemap: AssetRef::from("textures/never_preloaded.png".to_string()),
            events: vec![],
            ..Default::default()
        }));
        app.update();

        let mut q = app.world_mut().query_filtered::<(
            Option<&PendingSkyboxSwap>,
            Option<&SkyboxConfig>,
        ), With<ScenarioCameraMarker>>();
        let (pending, installed) = q.single(app.world()).expect("scenario camera spawned");
        assert!(
            pending.is_some(),
            "the loader hands the skybox to the deferred applier"
        );
        assert!(
            installed.is_none(),
            "no eager SkyboxConfig: the bcs observer would panic on an unloaded image"
        );
    }

    /// The scenario-advance decision table (task 20260716-125856, extended by
    /// 20260716-214919). A plain pause (pause menu, no outcome) always wins; a
    /// queued switch beats the outcome fallback (a Defeat with a queued retry
    /// must retry, not exit); an outcome with nothing queued exits to the
    /// menu; bare Enter mid-scenario stays inert. The outcome frame now
    /// FREEZES the sim (it enters Paused), so its Enter advance must stay live
    /// UNDER that pause - `paused && has_outcome` is no longer ignored.
    #[test]
    fn advance_decision_table() {
        // A plain pause (no outcome) swallows the key regardless of a queue.
        for queued in [false, true] {
            assert_eq!(
                decide_advance(true, queued, false),
                AdvanceDecision::Ignore,
                "a plain pause ignores the advance key (queued={queued})"
            );
        }
        // The outcome frame's own pause does NOT swallow it: the advance is
        // the intended input under the outcome modal. Both paused and unpaused
        // (the pre-freeze phase) resolve the same way.
        for paused in [false, true] {
            assert_eq!(
                decide_advance(paused, true, true),
                AdvanceDecision::ReleaseQueued,
                "a queued retry/continue beats the exit fallback (paused={paused})"
            );
            assert_eq!(
                decide_advance(paused, false, true),
                AdvanceDecision::ExitToMenu,
                "an unqueued outcome exits to the menu (paused={paused})"
            );
        }
        assert_eq!(
            decide_advance(false, true, false),
            AdvanceDecision::ReleaseQueued
        );
        assert_eq!(decide_advance(false, false, false), AdvanceDecision::Ignore);
    }

    /// A declared outcome dies with its scenario (task 20260716-125856,
    /// same reset class as the emphasis clear): the unload teardown resets
    /// `CurrentOutcome` alongside the scoped-entity sweep.
    #[test]
    fn teardown_clears_the_declared_outcome() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<CurrentScenario>();
        app.init_resource::<CurrentOutcome>();
        app.add_observer(unload_scenario);

        app.world_mut().resource_mut::<CurrentOutcome>().0 = Some(OutcomeActionConfig {
            outcome: ScenarioOutcomeKind::Defeat,
            message: None,
        });
        let scoped = app.world_mut().spawn(ScenarioScopedMarker).id();
        app.update();

        app.world_mut().trigger(UnloadScenario);
        app.update();

        assert_eq!(
            app.world().resource::<CurrentOutcome>().0,
            None,
            "unload clears the declared outcome"
        );
        assert!(
            app.world().get_entity(scoped).is_err(),
            "the scoped sweep still runs (the teardown grew a param, not a fork)"
        );
    }

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
                impact_sound: None,
                destroy_sound: None,
                radius: 1.0,
                texture: AssetRef::default(),
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
            cubemap: AssetRef::default(),
            events,
            ..Default::default()
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

    /// Restarting the SAME scenario must re-show its objective text. The
    /// objectives panel is despawned+respawned on every (re)load (it rides the
    /// player ship), and bevy_common_systems repaints its lines only when
    /// `GameObjectives` is `resource_changed` (see
    /// `world::tests::unchanged_objectives_do_not_flag_the_resource`). A restart
    /// re-posts IDENTICAL objectives, so unless teardown resets `GameObjectives`
    /// the write-on-diff sync sees no change, never re-flags the resource, and
    /// the freshly-spawned panel stays blank - the objective still works, but
    /// its text is gone (the reported UI bug). Teardown must reset the objectives
    /// mirror, the same reset class as the event world / emphasis / outcome.
    ///
    /// Driven through the real load path (on_load_scenario fires OnStart, whose
    /// Objective action posts through the event pipeline into `GameObjectives`);
    /// the repaint proxy counts `resource_changed::<GameObjectives>` the way the
    /// bcs panel does.
    #[test]
    fn objective_text_repaints_after_restarting_the_same_scenario() {
        use bevy_common_systems::prelude::{GameEventsPlugin, GameObjectives};

        #[derive(Resource, Default)]
        struct ObjectiveRepaints(usize);

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()));
        app.init_asset::<Image>();
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<CurrentScenario>();
        app.init_resource::<GameObjectives>();
        app.init_resource::<ObjectiveRepaints>();
        app.add_observer(on_load_scenario);
        app.add_systems(
            Update,
            (|mut n: ResMut<ObjectiveRepaints>| n.0 += 1)
                .run_if(resource_changed::<GameObjectives>),
        );

        let scenario = scenario_with(
            "arena",
            vec![event_with(vec![EventActionConfig::Objective(
                ObjectiveActionConfig::new("clear_arena", "Destroy the three derelict rocks."),
            )])],
        );

        // Fresh load: the OnStart objective syncs into GameObjectives and paints.
        app.world_mut().trigger(LoadScenario(scenario.clone()));
        for _ in 0..8 {
            app.update();
        }
        assert_eq!(
            app.world().resource::<GameObjectives>().objectives.len(),
            1,
            "delivery guard: a fresh load posts the objective"
        );
        let painted_on_fresh = app.world().resource::<ObjectiveRepaints>().0;
        assert!(
            painted_on_fresh >= 1,
            "delivery guard: the fresh load repainted the objectives panel"
        );

        // Restart the same scenario: the panel is rebuilt, so it must repaint
        // from the (identical) objective - or it stays blank.
        app.world_mut().trigger(LoadScenario(scenario));
        for _ in 0..8 {
            app.update();
        }
        assert_eq!(
            app.world().resource::<GameObjectives>().objectives.len(),
            1,
            "the objective is active again after the restart"
        );
        let painted_on_restart = app.world().resource::<ObjectiveRepaints>().0;
        assert!(
            painted_on_restart > painted_on_fresh,
            "restarting the same scenario must repaint the objective panel; a \
             teardown that leaves GameObjectives stale makes the identical re-post \
             a no-op and the panel stays blank ({painted_on_fresh} -> {painted_on_restart})"
        );
    }

    /// Review 20260717-163033 R1.1: the comms mirror must be CLEARED at
    /// teardown, or a reload that pushes as many story lines as the old
    /// scenario had (retry, or chapter chain with equal counts) slips the
    /// sync's length-only diff and the NEW scenario's opening line never
    /// reaches the HUD. Two scenarios, one line each, different text: the
    /// feed must say the second scenario's line after the switch.
    #[test]
    fn scenario_switch_replaces_an_equal_length_story_feed() {
        use nova_gameplay::prelude::{StoryFeed, StoryLine};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy_common_systems::prelude::GameEventsPlugin::<
            NovaEventWorld,
        >::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<bevy_common_systems::prelude::GameObjectives>();
        app.init_resource::<StoryFeed>();
        app.init_resource::<CurrentScenario>();
        app.add_observer(unload_scenario);

        // Scenario A's line is on screen (simulate the synced state).
        app.world_mut()
            .resource_mut::<NovaEventWorld>()
            .push_story_message(StoryMessageActionConfig {
                speaker: "Okono".to_string(),
                text: "alpha".to_string(),
                dwell: None,
            });
        app.world_mut()
            .resource_mut::<StoryFeed>()
            .0
            .push(StoryLine {
                speaker: "Okono".to_string(),
                text: "alpha".to_string(),
                dwell: None,
            });
        app.insert_resource(CurrentScenario(Some(scenario_with("a", vec![]))));
        app.update();

        // The aliasing window: teardown AND scenario B's equal-count push
        // land before any sync frame runs (production's on_load_scenario
        // clears the world and fires OnStart in one observer chain). The
        // trigger runs the unload observer synchronously; push B's line
        // immediately after, THEN let the sync run. Without the teardown
        // feed-clear the sync sees len 1 == len 1 and never rewrites -
        // scenario B opens showing scenario A's line (the sabotage shape
        // this test was tightened against: an intermediate update here
        // masks the bug).
        app.world_mut().trigger(UnloadScenario);
        assert!(
            app.world().resource::<StoryFeed>().0.is_empty(),
            "the unload observer itself must clear the comms mirror"
        );
        app.world_mut()
            .resource_mut::<NovaEventWorld>()
            .push_story_message(StoryMessageActionConfig {
                speaker: "Vesh".to_string(),
                text: "beta".to_string(),
                dwell: None,
            });
        app.update();
        app.update();
        let feed = app.world().resource::<StoryFeed>();
        assert_eq!(feed.0.len(), 1);
        assert_eq!(feed.0[0].text, "beta", "the new scenario's line displays");
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

    /// The OnUpdate pulse is Unpaused-gated (task 20260716-231855): a
    /// handler whose predicate is already true must NOT re-run its action
    /// every frame while the game is Paused (the ESC menu / outcome frame),
    /// and must resume firing on unpause. Proven with a filterless OnUpdate
    /// handler that INCREMENTS a counter - a value that would keep climbing
    /// under pause if the pulse leaked through. Uses the exact production
    /// run condition (`scenario_is_live.and_then(in_state(Unpaused))`).
    #[test]
    fn on_update_pulse_freezes_while_paused_and_resumes_on_unpause() {
        use bevy::state::app::StatesPlugin;
        use bevy_common_systems::prelude::{EventHandler, GameEventsPlugin, GameObjectives};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(StatesPlugin);
        app.init_state::<PauseStates>();
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.init_resource::<CurrentScenario>();
        app.add_systems(
            Update,
            fire_on_update.run_if(scenario_is_live.and_then(in_state(PauseStates::Unpaused))),
        );

        // Seed the counter so the increment expression has a number to read
        // (an undefined variable would error the action out).
        app.world_mut()
            .resource_mut::<NovaEventWorld>()
            .insert_variable("count".to_string(), VariableLiteral::Number(0.0));

        // A filterless OnUpdate handler that does `count = count + 1` - its
        // predicate is trivially always true, so every pulse re-runs it.
        let mut handler = EventHandler::<NovaEventWorld>::from(EventConfig::OnUpdate);
        handler.add_action(EventActionConfig::VariableSet(VariableSetActionConfig {
            key: "count".to_string(),
            expression: VariableExpressionNode::new_add(
                VariableTermNode::new_factor(VariableFactorNode::new_name("count")),
                VariableExpressionNode::new_term(VariableTermNode::new_factor(
                    VariableFactorNode::new_literal(VariableLiteral::Number(1.0)),
                )),
            ),
        }));
        app.world_mut().spawn(handler);

        let count = |app: &App| match app
            .world()
            .resource::<NovaEventWorld>()
            .get_variable("count")
        {
            Some(VariableLiteral::Number(n)) => *n,
            other => panic!("count must be a number, got {other:?}"),
        };

        // Live scenario, Unpaused (default): the counter climbs each frame.
        app.insert_resource(CurrentScenario(Some(scenario_with("live", vec![]))));
        app.update();
        app.update();
        let while_unpaused = count(&app);
        assert!(
            while_unpaused > 0.0,
            "an Unpaused live scenario must pulse OnUpdate handlers ({while_unpaused})"
        );

        // Pause: the pulse stops, so the already-true handler stops re-firing
        // and the counter is frozen no matter how many frames pass.
        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::Paused);
        app.update(); // applies the transition
        let at_pause = count(&app);
        app.update();
        app.update();
        app.update();
        assert_eq!(
            count(&app),
            at_pause,
            "a Paused game must freeze the OnUpdate pulse: an already-true \
             handler must not re-run its action while paused"
        );

        // Unpause: delivery-guarded, not dropped - the pulse resumes and the
        // counter climbs again.
        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::Unpaused);
        app.update(); // applies the transition
        app.update();
        app.update();
        assert!(
            count(&app) > at_pause,
            "unpausing must resume the OnUpdate pulse ({} -> {})",
            at_pause,
            count(&app)
        );
    }

    /// The scenario clock (task 20260717-112647): accumulates live unpaused
    /// seconds into the reserved variable and gates a real time-filtered
    /// OnUpdate handler - held before the threshold, fired after. Driven
    /// through the production tick + pulse pair on a manual 0.1s clock
    /// (steps under Time<Virtual>'s 0.25s max_delta clamp - the
    /// manual-time-rig lesson).
    #[test]
    fn scenario_clock_gates_time_filtered_handlers() {
        use core::time::Duration;

        use bevy::time::TimeUpdateStrategy;
        use bevy_common_systems::prelude::{EventHandler, GameEventsPlugin, GameObjectives};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<PauseStates>();
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.init_resource::<CurrentScenario>();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            100,
        )));
        register_clock_and_pulse(&mut app);

        // A one-shot beat the way an author writes it: elapsed > 0.5s AND
        // the flag unfired, then the action raises the flag.
        let mut handler = EventHandler::<NovaEventWorld>::from(EventConfig::OnUpdate);
        handler.add_filter(EventFilterConfig::Expression(ExpressionFilterConfig(
            VariableConditionNode::new_greater_than(
                VariableExpressionNode::new_term(VariableTermNode::new_factor(
                    VariableFactorNode::new_name(SCENARIO_ELAPSED_VAR),
                )),
                VariableExpressionNode::new_term(VariableTermNode::new_factor(
                    VariableFactorNode::new_literal(VariableLiteral::Number(0.5)),
                )),
            ),
        )));
        handler.add_action(EventActionConfig::VariableSet(VariableSetActionConfig {
            key: "beat_fired".to_string(),
            expression: VariableExpressionNode::new_term(VariableTermNode::new_factor(
                VariableFactorNode::new_literal(VariableLiteral::Boolean(true)),
            )),
        }));
        app.world_mut().spawn(handler);

        app.insert_resource(CurrentScenario(Some(scenario_with("live", vec![]))));

        // ~0.3s of scenario time: the gate must hold (fails closed while
        // the clock is below the threshold).
        for _ in 0..3 {
            app.update();
        }
        assert!(
            app.world()
                .resource::<NovaEventWorld>()
                .get_variable("beat_fired")
                .is_none(),
            "the time gate holds before the threshold"
        );

        // Past 0.5s the beat fires - the delivery guard proving the clock
        // is what advanced (with the tick removed this stays None forever).
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(
            app.world()
                .resource::<NovaEventWorld>()
                .get_variable("beat_fired"),
            Some(&VariableLiteral::Boolean(true)),
            "the beat fires once the scenario clock passes the threshold"
        );
    }

    /// The clock freezes under pause exactly like the pulse it feeds (same
    /// chained registration, same run condition), and resumes on unpause.
    #[test]
    fn scenario_clock_freezes_while_paused() {
        use core::time::Duration;

        use bevy::time::TimeUpdateStrategy;
        use bevy_common_systems::prelude::{GameEventsPlugin, GameObjectives};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<PauseStates>();
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.init_resource::<CurrentScenario>();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            100,
        )));
        register_clock_and_pulse(&mut app);
        let elapsed = |app: &App| match app
            .world()
            .resource::<NovaEventWorld>()
            .get_variable(SCENARIO_ELAPSED_VAR)
        {
            Some(VariableLiteral::Number(n)) => *n,
            _ => 0.0,
        };

        app.insert_resource(CurrentScenario(Some(scenario_with("live", vec![]))));
        for _ in 0..3 {
            app.update();
        }
        let before_pause = elapsed(&app);
        assert!(
            before_pause > 0.0,
            "a live unpaused scenario ticks the clock"
        );

        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::Paused);
        app.update(); // applies the transition
        let at_pause = elapsed(&app);
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(
            elapsed(&app),
            at_pause,
            "a paused game must freeze the scenario clock"
        );

        // Unpause: delivery-guarded - the clock climbs again.
        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::Unpaused);
        app.update();
        app.update();
        assert!(
            elapsed(&app) > at_pause,
            "unpausing must resume the scenario clock"
        );
    }

    /// The clock dies with the event world (teardown/retry): after clear()
    /// the variable is gone, so a time gate on the next scenario fails
    /// closed until the fresh clock ticks - never inherits stale seconds.
    #[test]
    fn scenario_clock_resets_with_the_event_world() {
        let mut world = NovaEventWorld::default();
        world.insert_variable(
            SCENARIO_ELAPSED_VAR.to_string(),
            VariableLiteral::Number(42.0),
        );
        world.clear();
        assert!(
            world.get_variable(SCENARIO_ELAPSED_VAR).is_none(),
            "teardown clears the clock with the rest of the event world"
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
        // The tracker now measures its window against `scenario_elapsed`, so
        // the clock has to advance under the same gate production uses. Chain
        // the tick ahead of the tracker so it reads THIS frame's clock, exactly
        // like the plugin's `.after(tick_scenario_clock)` ordering.
        app.add_systems(
            Update,
            (tick_scenario_clock, track_orbit_holds)
                .chain()
                .run_if(scenario_is_live),
        );

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

    /// A per-ship `OrbitHoldSecs` override (task 20260717-165031) shortens the
    /// hold window: a 1s override fires within ~1.2s of hold, long before the
    /// 5s default would.
    #[test]
    fn orbit_hold_honors_a_per_ship_override() {
        use core::time::Duration;

        use bevy::time::TimeUpdateStrategy;
        use bevy_common_systems::prelude::{EventHandler, GameEventsPlugin, GameObjectives};
        use nova_gameplay::prelude::{Autopilot, AutopilotAction, SpaceshipRootMarker};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            0.2,
        )));
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.insert_resource(CurrentScenario(Some(scenario_with("live", vec![]))));
        app.add_systems(
            Update,
            (tick_scenario_clock, track_orbit_holds)
                .chain()
                .run_if(scenario_is_live),
        );

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
        // Override: a 1s hold window on this ship.
        app.world_mut().spawn((
            SpaceshipRootMarker,
            EntityId::new("player_spaceship".to_string()),
            EntityTypeName::new("spaceship".to_string()),
            Autopilot::engage(AutopilotAction::Orbit { well, plan: None }),
            OrbitHoldSecs(1.0),
        ));

        // 3 frames (~0.6s): under the 1s window - no fire yet.
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(orbits(&app), 0.0, "no fire before the 1s override window");

        // 5 more frames (total ~1.6s): past the 1s window - exactly one fire,
        // where the 5s default would still be silent.
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(
            orbits(&app),
            1.0,
            "the 1s override fires well before the 5s default"
        );
    }

    // -- the player-lock bridge (beat-sheet-v2 spike 20260713-140742) --

    #[test]
    fn a_lock_slot_fires_on_acquisition_then_echoes_per_window() {
        let a = Entity::from_raw_u32(1).unwrap();
        let b = Entity::from_raw_u32(2).unwrap();
        let mut state = None;

        // `now` is the absolute scenario clock; advance it before each tick
        // exactly as the clock does per frame. The window is `now - last_fire`.
        let mut now = 0.0_f64;
        let mut at = |dt: f64| {
            now += dt;
            now
        };

        // The default engine window; the slot takes it per call.
        let w = LOCK_REFIRE_SECS;

        // Acquisition fires immediately.
        assert_eq!(tick_lock_slot(&mut state, Some(a), at(0.1), w), Some(a));
        // Held: quiet until the echo window elapses, then one re-fire.
        assert_eq!(tick_lock_slot(&mut state, Some(a), at(2.0), w), None);
        assert_eq!(tick_lock_slot(&mut state, Some(a), at(2.0), w), None);
        assert_eq!(
            tick_lock_slot(&mut state, Some(a), at(2.0), w),
            Some(a),
            "a held lock echoes once per window (the anti-soft-lock recurrence)"
        );
        assert_eq!(tick_lock_slot(&mut state, Some(a), at(2.0), w), None);
        // A live-radar retarget is a fresh acquisition on a fresh clock.
        assert_eq!(tick_lock_slot(&mut state, Some(b), at(0.1), w), Some(b));
        assert_eq!(tick_lock_slot(&mut state, Some(b), at(2.0), w), None);
        // Clearing re-arms: the next lock is an acquisition again.
        assert_eq!(tick_lock_slot(&mut state, None, at(0.1), w), None);
        assert_eq!(tick_lock_slot(&mut state, Some(b), at(0.1), w), Some(b));
    }

    /// A per-player `refire_secs` override (task 20260717-165031) changes the
    /// echo cadence: a 2s window re-fires after 2s of hold, where the 5s
    /// default would still be quiet.
    #[test]
    fn a_lock_slot_honors_a_custom_refire_window() {
        let a = Entity::from_raw_u32(1).unwrap();
        let mut state = None;
        let mut now = 0.0_f64;
        let mut at = |dt: f64| {
            now += dt;
            now
        };

        // Acquisition fires immediately regardless of window.
        assert_eq!(tick_lock_slot(&mut state, Some(a), at(0.1), 2.0), Some(a));
        // 1.5s held under the 2s window: quiet (and would be quiet at 5s too).
        assert_eq!(tick_lock_slot(&mut state, Some(a), at(1.5), 2.0), None);
        // Crossing 2s of hold: re-fires on the SHORT window, where the 5s
        // default would not have yet.
        assert_eq!(
            tick_lock_slot(&mut state, Some(a), at(1.0), 2.0),
            Some(a),
            "a 2s override echoes at 2s of hold"
        );
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
        // The bridge measures its echo window against `scenario_elapsed`, so
        // tick the clock ahead of the tracker (mirrors the plugin's
        // `.after(tick_scenario_clock)` ordering) - otherwise `now` never moves
        // and "held is quiet" would pass for the wrong reason.
        app.add_systems(
            Update,
            (tick_scenario_clock, track_player_locks)
                .chain()
                .run_if(scenario_is_live),
        );

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
        // on_load_scenario resolves the scenario cubemap through the
        // AssetServer, so the load path needs the asset plugin present.
        app.add_plugins((
            bevy::app::TaskPoolPlugin::default(),
            bevy::asset::AssetPlugin::default(),
        ));
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

    /// The whole scenario config tree round-trips through RON under the
    /// `serde` feature: an asteroid with a path-authored texture, a beacon,
    /// and a player ship whose thruster section carries one keyboard and one
    /// mouse binding. The bindings survive the `Binding <-> BindingInput`
    /// bridge (`binding_map_serde`), and the cubemap/texture author as bare
    /// path strings (`AssetRef`).
    #[cfg(feature = "serde")]
    #[test]
    fn a_scenario_config_round_trips_through_ron() {
        use bevy::platform::collections::HashMap;
        use bevy_enhanced_input::prelude::Binding;
        use nova_gameplay::prelude::{
            BaseSectionConfig, SectionConfig, SectionKind, ThrusterSectionConfig,
        };

        use crate::objects::spaceship::{
            PlayerControllerConfig, SpaceshipConfig, SpaceshipController, SpaceshipSectionConfig,
        };

        let bindings = vec![
            Binding::from(KeyCode::KeyW),
            Binding::from(MouseButton::Left),
        ];
        let mut input_mapping: HashMap<String, Vec<Binding>> = HashMap::default();
        input_mapping.insert("thruster".to_string(), bindings.clone());

        let ship = SpaceshipConfig {
            allegiance: None,
            controller: SpaceshipController::Player(PlayerControllerConfig {
                input_mapping,
                speed_cap: Some(100.0),
                infinite_ammo: true,
                lock_refire_secs: None,
            }),
            sections: vec![SpaceshipSectionConfig {
                id: "thruster".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                source: SectionSource::Inline(SectionConfig {
                    base: BaseSectionConfig {
                        id: "thruster".to_string(),
                        ..default()
                    },
                    kind: SectionKind::Thruster(ThrusterSectionConfig::default()),
                }),
                modifications: vec![],
            }],
        };

        let scenario = ScenarioConfig {
            id: "roundtrip".to_string(),
            name: "Round Trip".to_string(),
            description: "serde smoke".to_string(),
            cubemap: AssetRef::from("scenarios/space.cube.png"),
            events: vec![ScenarioEventConfig {
                name: EventConfig::OnStart,
                filters: vec![],
                actions: vec![
                    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
                        base: BaseScenarioObjectConfig {
                            id: "rock".to_string(),
                            name: "Rock".to_string(),
                            position: Vec3::new(1.0, 2.0, 3.0),
                            rotation: Quat::IDENTITY,
                        },
                        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                            impact_sound: None,
                            destroy_sound: None,
                            radius: 5.0,
                            texture: AssetRef::from("textures/rock.png"),
                            health: 100.0,
                            surface_gravity: None,
                            invulnerable: false,
                            lock_signature: None,
                        }),
                    }),
                    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
                        base: BaseScenarioObjectConfig {
                            id: "beacon_1".to_string(),
                            name: "Beacon".to_string(),
                            position: Vec3::new(10.0, 0.0, 0.0),
                            rotation: Quat::IDENTITY,
                        },
                        kind: ScenarioObjectKind::Beacon(BeaconConfig {
                            label: "BEACON 1".to_string(),
                            radius: 2.0,
                            color: Color::srgb(0.3, 0.9, 1.0),
                            area_radius: Some(40.0),
                            lock_signature: None,
                        }),
                    }),
                    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
                        base: BaseScenarioObjectConfig {
                            id: "player".to_string(),
                            name: "Player".to_string(),
                            position: Vec3::ZERO,
                            rotation: Quat::IDENTITY,
                        },
                        kind: ScenarioObjectKind::Spaceship(ship),
                    }),
                ],
            }],
            ..Default::default()
        };

        let ron = ron::to_string(&scenario).expect("scenario serializes to RON");
        let back: ScenarioConfig = ron::from_str(&ron).expect("scenario deserializes from RON");

        // Top-level scalars and the path-authored cubemap survive.
        assert_eq!(back.id, scenario.id);
        assert_eq!(back.name, scenario.name);
        assert_eq!(back.cubemap.path(), Some("scenarios/space.cube.png"));
        assert_eq!(back.events.len(), 1);

        let actions = &back.events[0].actions;
        assert_eq!(actions.len(), 3);

        // The asteroid's texture round-trips as its path.
        let EventActionConfig::SpawnScenarioObject(rock) = &actions[0] else {
            panic!("first action is the asteroid spawn");
        };
        let ScenarioObjectKind::Asteroid(rock_kind) = &rock.kind else {
            panic!("first spawn is an asteroid");
        };
        assert_eq!(rock_kind.texture.path(), Some("textures/rock.png"));
        assert_eq!(rock_kind.radius, 5.0);

        // The player ship's bindings survive the Binding<->BindingInput bridge.
        let EventActionConfig::SpawnScenarioObject(player) = &actions[2] else {
            panic!("third action is the ship spawn");
        };
        let ScenarioObjectKind::Spaceship(ship_kind) = &player.kind else {
            panic!("third spawn is a spaceship");
        };
        let SpaceshipController::Player(player_config) = &ship_kind.controller else {
            panic!("the ship is player-controlled");
        };
        assert_eq!(player_config.speed_cap, Some(100.0));
        assert!(player_config.infinite_ammo);
        assert_eq!(
            player_config.input_mapping.get("thruster"),
            Some(&bindings),
            "the keyboard + mouse bindings round-trip unchanged"
        );
    }

    /// The `thumbnail`/`hidden`/`menu_backdrop` fields are serde-defaulted, so
    /// a scenario RON authored before they existed still parses
    /// (None/false/false), and a scenario carrying them round-trips. Guards
    /// the back-compat contract the picker and the menu-backdrop rotation
    /// depend on.
    #[test]
    fn thumbnail_and_hidden_default_when_absent_and_round_trip_when_present() {
        // Legacy shape: no thumbnail, no hidden, no menu_backdrop.
        let legacy = r#"(id: "legacy", name: "Legacy", description: "old", cubemap: "sky.png")"#;
        let parsed: ScenarioConfig = ron::from_str(legacy).expect("legacy scenario parses");
        assert_eq!(parsed.thumbnail, None, "absent thumbnail defaults to None");
        assert!(!parsed.hidden, "absent hidden defaults to false");
        assert!(
            !parsed.menu_backdrop,
            "absent menu_backdrop defaults to false"
        );

        // A configured scenario round-trips the fields, and the defaulted
        // fields stay out of the serialized form (skip_serializing_if).
        let configured = ScenarioConfig {
            id: "cfg".to_string(),
            name: "Configured".to_string(),
            description: "new".to_string(),
            cubemap: AssetRef::from("sky.png"),
            thumbnail: Some(AssetRef::from("thumb.png")),
            hidden: true,
            menu_backdrop: true,
            events: vec![],
        };
        // `ron::to_string` is compact (no spaces after colons).
        let ron = ron::to_string(&configured).expect("configured scenario serializes");
        assert!(ron.contains("thumbnail:Some(\"thumb.png\")"), "ron: {ron}");
        assert!(ron.contains("hidden:true"), "ron: {ron}");
        assert!(ron.contains("menu_backdrop:true"), "ron: {ron}");
        let back: ScenarioConfig = ron::from_str(&ron).expect("configured scenario parses");
        assert_eq!(
            back.thumbnail
                .and_then(|t| t.path().map(String::from))
                .as_deref(),
            Some("thumb.png")
        );
        assert!(back.hidden);
        assert!(back.menu_backdrop);

        // The defaulted form omits the keys.
        let bare = ron::to_string(&parsed).expect("legacy re-serializes");
        assert!(!bare.contains("thumbnail"), "ron: {bare}");
        assert!(!bare.contains("hidden"), "ron: {bare}");
        assert!(!bare.contains("menu_backdrop"), "ron: {bare}");
    }
}
