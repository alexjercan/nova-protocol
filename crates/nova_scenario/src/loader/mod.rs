//! The scenario/campaign registry and the RON-authored config types it holds.
//!
//! Owns what a scenario IS on disk ([`ScenarioConfig`], [`CampaignConfig`]),
//! which ones are loaded ([`GameScenarios`], [`GameCampaigns`]), the lint
//! issues found in them, and the run lifecycle in the `clock`, `lifecycle` and
//! `trackers` submodules.
//!
//! Touch this module when changing the authored scenario schema or how a run
//! starts, ticks and ends.

/// Scenario loader plugin and related types
use bevy::{platform::collections::HashMap, prelude::*};
use bevy_enhanced_input::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;
use nova_input::prelude::*;
use nova_ship::prelude::*;

use crate::prelude::*;

/// The scripted-camera layer: who owns the scenario camera while a script does.
mod camera;
mod clock;
#[cfg(test)]
mod fixtures;
mod lifecycle;
/// The scenario's glTF warm-up: resolved at load, held for the run.
pub mod preload;
mod trackers;
mod wake;

use camera::register_scripted_camera;
pub use camera::{
    CameraOffsetFrame, ScriptedCameraAnchor, ScriptedCameraLookAt, ScriptedCameraPose,
    ScriptedCameraTransform,
};
use clock::register_clock_and_pulse;
#[cfg(any(test, feature = "test-support"))]
pub(crate) use clock::tick_scenario_timers;
pub use lifecycle::ScenarioCameraMarker;
use lifecycle::{
    configure_scenario_gating, on_load_scenario, on_next_input, on_player_spaceship_destroyed,
    on_player_spaceship_spawned, rebuild_scenario_input_on_rebind, register_scenario_scoping,
    scenario_bindings, unload_scenario, ScenarioInputMarker,
};
use preload::register_scenario_preload;
pub use trackers::ORBIT_LAP_GRACE_SECS;
use trackers::{
    track_orbit_transitions, track_player_autopilot_completions, track_player_locks,
    track_ship_order_reports, LockEcho, OrbitEcho,
};
pub(crate) use wake::{configure_scenario_shape, WakeProfile};

/// Glob-import surface: `use nova_scenario::loader::prelude::*` brings the
/// scenario registry resources, load/unload triggers, and markers into scope.
pub mod prelude {
    pub use super::{
        lifecycle::scenario_bindings, preload::prelude::*, scenario_is_live, CameraOffsetFrame,
        CampaignConfig, CampaignId, ContentIssues, CurrentScenario, GameCampaigns, GameScenarios,
        LoadScenario, NewGameStart, ScenarioCameraMarker, ScenarioConfig, ScenarioEventConfig,
        ScenarioId, ScenarioLoaded, ScenarioLoaderPlugin, ScenarioScopedMarker,
        ScenarioStartFailure, ScenarioStartFailureReport, ScriptedCameraAnchor,
        ScriptedCameraLookAt, ScriptedCameraPose, ScriptedCameraTransform, UnloadScenario,
        ORBIT_LAP_GRACE_SECS,
    };
}

/// Type alias for Scenario ID
pub type ScenarioId = String;

/// Type alias for Campaign ID (the stable key of a [`CampaignConfig`]).
pub type CampaignId = String;

/// The collection of available game scenarios
#[derive(Resource, Clone, Debug, Deref, DerefMut, Default)]
pub struct GameScenarios(pub HashMap<ScenarioId, ScenarioConfig>);

/// The collection of available campaigns, keyed by campaign id.
///
/// A campaign owns the ORDERED list of its member scenario ids (see
/// [`CampaignConfig`]); this registry is the single source of truth the
/// Scenarios picker reads to render collapsible campaign groups and to launch
/// any member - including `hidden` chapters that the flat picker filters out
/// but that a campaign lists for replay. Written by the bundle merge from the
/// merged `Content::Campaign` items, mirroring [`GameScenarios`].
#[derive(Resource, Clone, Debug, Deref, DerefMut, Default)]
pub struct GameCampaigns(pub HashMap<CampaignId, CampaignConfig>);

/// The scenario New Game launches, declared by the BASE bundle's manifest
/// (`new_game_scenario` in `base.bundle.ron`) and written by the bundle merge.
/// Deliberately NOT a scenario flag and NOT overlayable: only the catalog entry
/// marked `base: true` is honored, so a non-base mod can never redirect what
/// New Game starts (mods add PICKER entries and `menu_backdrop` scenarios
/// instead). `None` when base declares nothing; the menu then falls back to the
/// first listed scenario.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct NewGameStart(pub Option<ScenarioId>);

/// Lint findings per registered scenario, written by the bundle merge against
/// the MERGED registries - the runtime half of the content gate
/// (`nova_scenario::lint` is the shared core). Keyed by scenario id; scenarios
/// without findings have no entry. Error-level findings make `on_load_scenario`
/// REFUSE the load.
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
    /// The scenario's display name, shown in the overlay title.
    pub scenario_name: String,
    /// One line per Error-level finding that refused the start.
    pub messages: Vec<String>,
}

/// A campaign: a named, ORDERED sequence of member scenario ids.
///
/// A campaign is a first-class content entity (`Content::Campaign`, in
/// nova_modding) that owns its full membership - the single source of truth for
/// which scenarios belong to it and in what order. Membership is the explicit
/// `scenarios` list, so the order is unambiguous (the Vec order) and a `hidden`
/// chapter reached only via `NextScenario` chaining can still be LISTED for
/// replay simply by naming its id here. This replaces the interim per-scenario
/// `campaign` metadata: the picker groups and launches from this mapping
/// instead of reconstructing groups by parsing per-scenario display names.
///
/// `id` is the stable key (e.g. `"nova_protocol"`); `name` is the DISPLAY name
/// (e.g. `"Nova Protocol"`); `scenarios` are member scenario ids in play order,
/// hidden members included. In strict RON a campaign content item is authored
/// as `Campaign((id: "nova_protocol", name: "Nova Protocol", scenarios: ["a",
/// "b"]))`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CampaignConfig {
    /// The stable campaign key, e.g. `"nova_protocol"`.
    pub id: CampaignId,
    /// The campaign's display name, e.g. `"Nova Protocol"`.
    pub name: String,
    /// The member scenario ids, in play order (chapter 1, 2, 3, ...). May name
    /// `hidden` scenarios: they are filtered from the flat picker but reachable
    /// for replay under their campaign header.
    pub scenarios: Vec<ScenarioId>,
}

/// Configuration for a game scenario.
///
/// Build one with [`ScenarioConfig::new`] and fill the optional fields through
/// struct-update syntax: `ScenarioConfig { hidden: true,
/// ..ScenarioConfig::new(id, name, cubemap) }`. There is deliberately no
/// `Default`: a defaulted `cubemap` is a handle-backed `AssetRef`, which errors
/// on serialize (see `AssetRef`), so a fully default `ScenarioConfig` was never
/// a valid scenario.
#[derive(Clone, Debug)]
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
    /// How bright that sky comes up, in lux. Serde-defaulted to
    /// [`DEFAULT_SKYBOX_BRIGHTNESS`], so scenarios authored before this field
    /// still parse and every file that takes the default still writes none.
    ///
    /// Authored here rather than left to the loader because two skies are not
    /// two exposures: a dusk cubemap and a deep-field one want different
    /// numbers under the same rig. `SetSkybox` changes it mid-scenario; this is
    /// what the scenario STARTS at.
    #[cfg_attr(
        feature = "serde",
        serde(
            default = "default_skybox_brightness",
            skip_serializing_if = "is_default_skybox_brightness"
        )
    )]
    pub skybox_brightness: f32,
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
    /// A backdrop POSES ITS OWN CAMERA: it must author a `SetCamera` action
    /// (lint makes a poseless backdrop an Error, and erroring backdrops are
    /// filtered out of the menu draw - the menu derives no pose of its own).
    /// Serde-defaulted to false; author as `menu_backdrop: true`.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "is_false"))]
    pub menu_backdrop: bool,
    /// Read-only queries sampled into auto-updating scenario variables.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub watches: Vec<WatchConfig>,
    /// Events associated with the scenario
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub events: Vec<ScenarioEventConfig>,
}

impl ScenarioConfig {
    /// A scenario with only its three REQUIRED fields set: everything else
    /// (`description`, `thumbnail`, `hidden`, `menu_backdrop`, `events`) takes
    /// its empty value, to be overridden through struct-update syntax.
    ///
    /// # Panics
    ///
    /// Debug builds only: `id` and `name` are adjacent `String`s, so swapping
    /// them compiles and silently yields a scenario whose id is its display
    /// name. Scenario ids are snake_case slugs and display names are not, so
    /// this asserts the slug shape to make that swap loud instead of silent.
    /// Only in-repo Rust builders reach here - authored RON deserializes
    /// straight into the struct.
    // `cubemap` stays a concrete `AssetRef<Image>`: an `impl Into` there makes
    // the callers' own `handle.into()` ambiguous, since half of bevy converts
    // a `Handle<Image>`.
    pub fn new(
        id: impl Into<ScenarioId>,
        name: impl Into<String>,
        cubemap: AssetRef<Image>,
    ) -> Self {
        let id = id.into();
        debug_assert!(
            !id.is_empty()
                && id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
            "scenario id {id:?} is not a snake_case slug - are `id` and `name` swapped?"
        );
        Self {
            id,
            name: name.into(),
            description: String::new(),
            cubemap,
            skybox_brightness: DEFAULT_SKYBOX_BRIGHTNESS,
            thumbnail: None,
            hidden: false,
            menu_backdrop: false,
            watches: Vec::new(),
            events: Vec::new(),
        }
    }

    /// Every typed query the scenario's HANDLERS read as an inline expression
    /// factor, in source order. [`ScenarioConfig::watches`] declares the rest.
    ///
    /// Two readers, and they must see the same list: the lint checks each
    /// query's target against what the scenario can spawn, and the loader
    /// decides from it whether the per-frame entity sampler has any reader at
    /// all. An arm missed here makes an inline query unavailable at runtime,
    /// which expressions fail closed on.
    ///
    /// Walks NESTED actions and `Sequence` gate filters through
    /// [`EventActionConfig::walk`], so a query inside a step reads like one
    /// inside a handler.
    pub fn inline_queries(&self) -> Vec<&QueryConfig> {
        fn condition<'a>(filter: &'a EventFilterConfig, out: &mut Vec<&'a QueryConfig>) {
            if let EventFilterConfig::Expression(config) = filter {
                collect_condition_queries(&config.0, out);
            }
        }

        let mut out = Vec::new();
        for event in &self.events {
            for filter in &event.filters {
                condition(filter, &mut out);
            }
            for action in &event.actions {
                action.walk_filters(&mut |filter| condition(filter, &mut out));
                action.walk(&mut |action| match action {
                    EventActionConfig::VariableSet(config) => {
                        collect_expression_queries(&config.expression, &mut out);
                    }
                    EventActionConfig::TimerStart(config) => {
                        collect_expression_queries(&config.seconds, &mut out);
                    }
                    _ => {}
                });
            }
        }
        out
    }

    /// Whether anything in the scenario can READ an entity query - a watch, or
    /// an inline expression factor.
    ///
    /// The per-frame entity sampler is gated on this. It walks every `EntityId`
    /// in the world and allocates two `String`s per match, so its cost scales
    /// with the WORLD (1,839 ids in a headless duel, most of them ship
    /// sections) and not with the scenario.
    pub fn reads_an_entity_query(&self) -> bool {
        fn is_entity(query: &QueryConfig) -> bool {
            matches!(query, QueryConfig::Entity(_))
        }
        self.watches.iter().any(|watch| is_entity(&watch.query))
            || self.inline_queries().into_iter().any(is_entity)
    }
}

/// `skip_serializing_if` predicate for a `bool` that defaults to false: omit it
/// from the serialized RON when it is false so unflagged scenarios stay clean.
#[cfg(feature = "serde")]
fn is_false(b: &bool) -> bool {
    !*b
}

/// The `serde` default for [`ScenarioConfig::skybox_brightness`].
#[cfg(feature = "serde")]
fn default_skybox_brightness() -> f32 {
    DEFAULT_SKYBOX_BRIGHTNESS
}

/// `skip_serializing_if` predicate for the skybox brightness: a scenario that
/// takes the default writes no field, so adding this one rewrote no content.
#[cfg(feature = "serde")]
fn is_default_skybox_brightness(brightness: &f32) -> bool {
    *brightness == DEFAULT_SKYBOX_BRIGHTNESS
}

/// Configuration for a scenario event
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ScenarioEventConfig {
    /// What this handler is FOR, in the author's own words.
    ///
    /// The trigger is not a name: a scenario has six handlers on `OnEnter` and
    /// nothing on the file or in the editor tells them apart. This is the line
    /// that does - `picket warden wakes` beside the `OnEnter` that wakes it.
    ///
    /// Optional, and omitted when absent: a file written before it existed
    /// parses unchanged, and one that never names a handler is not padded with
    /// empty strings.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub label: Option<String>,
    /// The name of the event to listen for
    pub name: EventConfig,
    /// Retire this handler the first time its filters pass.
    ///
    /// The engine holds the "we have already done this" fact so content does
    /// not have to. Without it a beat that can only happen once still needs a
    /// latch variable of its own, a filter reading it and an action writing
    /// it - three lines of ceremony per beat, none of them about the game -
    /// and the handler is walked every frame for the rest of the scenario
    /// anyway.
    ///
    /// Defaulted, so a file written before it existed parses unchanged.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "is_false"))]
    pub once: bool,
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

impl ScenarioEventConfig {
    /// Build the runtime handler this config describes.
    ///
    /// The loader and every headless rig go through here, so a field added to
    /// the config cannot be honoured by one and quietly dropped by the other.
    pub fn build_handler(&self) -> EventHandler<NovaEventWorld> {
        let mut handler = EventHandler::<NovaEventWorld>::from(self.name);
        handler.set_once(self.once);
        for filter in &self.filters {
            handler.add_filter(filter.clone());
        }
        for action in &self.actions {
            handler.add_action(action.clone());
        }
        handler
    }

    /// The wake handlers for every gated `Sequence` step this event can start.
    ///
    /// Separate from [`Self::build_handler`] because they are the ENGINE's
    /// handlers, not the author's: they carry no authored action, they are not
    /// counted as scenario handlers, and they exist only to move a cursor.
    pub fn gate_handlers(&self) -> Vec<EventHandler<NovaEventWorld>> {
        sequence_gate_handlers(&self.actions)
    }

    /// Every list of actions this handler can run in ONE frame: its own, then
    /// one per RUN of `Sequence` steps that land together, however deeply
    /// nested.
    ///
    /// The handler's own actions are always a group of their own: a sequence
    /// step lands at least a frame after the handler that queued it, because
    /// the driver sits ahead of the pulse in the scenario chain.
    ///
    /// Steps are NOT one group each. The driver drains every ready step in one
    /// `while` loop at one timestamp, so a step that waits for nothing runs in
    /// the SAME frame as the step before it - see
    /// [`SequenceStepConfig::runs_with_the_step_before`]. Those steps join
    /// their predecessor's group, and only a delay or a gate opens a new one.
    ///
    /// Every rule phrased "in the same frame" (an objective beside a comms
    /// line, an attach before its spawn) has to read groups rather than the
    /// handler's own list.
    pub fn action_groups(&self) -> Vec<Vec<&EventActionConfig>> {
        let mut groups: Vec<Vec<&EventActionConfig>> = vec![self.actions.iter().collect()];
        for action in &self.actions {
            action.walk(&mut |action| {
                if let EventActionConfig::Sequence(config) = action {
                    for (index, step) in config.steps.iter().enumerate() {
                        // The FIRST step always opens a group: it is a frame
                        // after the handler however short its delay.
                        match groups.last_mut() {
                            Some(open) if index > 0 && step.runs_with_the_step_before() => {
                                open.extend(step.actions.iter());
                            }
                            _ => groups.push(step.actions.iter().collect()),
                        }
                    }
                }
            });
        }
        groups
    }
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
        let mut object_count = 0;
        for action in scenario.events.iter().flat_map(|event| &event.actions) {
            action.walk(&mut |action| {
                if matches!(action, EventActionConfig::SpawnScenarioObject(_)) {
                    object_count += 1;
                }
            });
        }
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

/// The scenario lifecycle plugin: owns loading, unloading and the live
/// scenario's clock and event pulse. Registers the [`LoadScenario`] /
/// [`UnloadScenario`] observers, gates the spaceship input/section sets on
/// [`scenario_is_live`], and adds the clock tick + OnUpdate pulse plus the
/// orbit/lock/skybox/scripted-camera trackers (mostly `Update`, with the
/// scripted-camera enforce in `PostUpdate`).
pub struct ScenarioLoaderPlugin {
    /// Whether section render meshes are built at all; gates the
    /// [`preload`] warm-up. Mirrors `NovaScenarioPlugin::render`.
    pub render: bool,
}

impl Plugin for ScenarioLoaderPlugin {
    fn build(&self, app: &mut App) {
        trace!("ScenarioLoaderPlugin: build");

        configure_scenario_gating(app);
        register_scenario_preload(app, self.render);

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

        register_scenario_scoping(app);

        app.register_input_actions(scenario_bindings());
        app.add_input_context::<ScenarioInputMarker>();
        app.add_systems(
            Update,
            rebuild_scenario_input_on_rebind.run_if(resource_changed::<InputBindings>),
        );
        app.add_observer(on_next_input);
        app.add_observer(unload_scenario);

        // The OnUpdate pulse is live-AND-Unpaused gated, unlike the
        // state-transition trackers below. It fires UNCONDITIONALLY every frame, so under a
        // pause an OnUpdate handler whose predicate is already true would
        // re-run its action every frame via the pause-independent PostUpdate
        // state_to_world sync. State-transition trackers need no pause gate:
        // their source state is itself frozen while paused.
        //
        // apply_pending_skybox_swaps stays ungated too - it is cosmetic and
        // asset-load driven, and letting a queued swap finish under pause is
        // harmless. Do not blanket-gate.
        register_clock_and_pulse(app);

        // ORBIT lifecycle events follow maneuver and Hold-phase edges. "In
        // orbit" is autopilot state, not a position gate: the verb may choose a
        // ring outside any authored trigger volume.
        app.register_type::<OrbitEcho>();
        app.add_systems(
            FixedUpdate,
            track_orbit_transitions
                .after(NovaFlightSystems)
                .run_if(scenario_is_live),
        );

        // Successful player STOP/GOTO edges come directly from the flight
        // completion condition. Reading them after the flight set preserves
        // the target entity until its scenario handler can retire it.
        app.add_systems(
            FixedUpdate,
            track_player_autopilot_completions
                .after(NovaFlightSystems)
                .run_if(scenario_is_live),
        );

        // Ship helm-order lifecycle. After the flight layer, like the orbit
        // tracker and for the same reason: the driver queues an outcome the
        // same tick the physics produced it, and draining a tick late would
        // put every beat sequenced off an order one tick behind.
        app.add_systems(
            FixedUpdate,
            track_ship_order_reports
                .after(NovaFlightSystems)
                .run_if(scenario_is_live),
        );

        // Player lock lifecycle edges. AI locks remain gameplay-internal.
        app.register_type::<LockEcho>();
        app.add_systems(
            Update,
            track_player_locks
                .after(SpaceshipTargetingSystems)
                .run_if(scenario_is_live),
        );

        // Deferred skybox swap behind `EventActionConfig::SetSkybox`: the
        // action tags the scenario camera with a `PendingSkyboxSwap`, and this
        // applier installs the `SkyboxConfig` once the new cubemap image has
        // loaded - the setup observer fires once on insert and gives up (logged
        // error, no retry) on an unloaded image, so a synchronous insert in the
        // action would drop the sky for good.
        app.register_type::<PendingSkyboxSwap>();
        app.add_systems(Update, apply_pending_skybox_swaps.run_if(scenario_is_live));

        // Scripted-camera override (photo mode, the capture scripts, the
        // mainline cinematic). The chain it runs in is normally declared by
        // nova_ship's camera authority layer; add it when that plugin is absent
        // (a scenario-only test app) so the override phase is never an
        // unordered set.
        if !app.is_plugin_added::<CameraAuthorityPlugin>() {
            app.add_plugins(CameraAuthorityPlugin);
        }
        register_scripted_camera(app);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::fixtures::*;

    /// `new(id, name, ..)` takes two adjacent `String`s, so a swap compiles.
    /// The slug guard is what makes it loud in debug builds.
    #[test]
    #[should_panic(expected = "are `id` and `name` swapped?")]
    fn a_display_name_in_the_id_slot_is_rejected() {
        let _ = ScenarioConfig::new(
            "Sky Test".to_string(),
            "sky_test".to_string(),
            AssetRef::from("textures/sky.png".to_string()),
        );
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

    /// The whole scenario config tree round-trips through RON under the
    /// `serde` feature: an asteroid with a path-authored texture, a beacon,
    /// and a player ship whose thruster section carries one keyboard and one
    /// mouse binding. The bindings are `InputSource`s and serialize as
    /// themselves - there is no bridge left to survive - and the
    /// cubemap/texture author as bare path strings (`AssetRef`).
    #[cfg(feature = "serde")]
    #[test]
    fn a_scenario_config_round_trips_through_ron() {
        use std::collections::BTreeMap;

        use nova_input::prelude::InputSource;
        use nova_ship::prelude::{
            BaseSectionConfig, SectionConfig, SectionKind, ThrusterSectionConfig,
        };

        use crate::objects::{
            ship::prelude::{ShipHull, ShipSource},
            spaceship::{
                PlayerControllerConfig, SpaceshipConfig, SpaceshipController,
                SpaceshipSectionConfig,
            },
        };

        let bindings = vec![
            InputSource::from(KeyCode::KeyW),
            InputSource::from(MouseButton::Left),
        ];
        let mut input_mapping: BTreeMap<String, Vec<InputSource>> = BTreeMap::new();
        input_mapping.insert("thruster".to_string(), bindings.clone());

        let ship = SpaceshipConfig {
            controller: SpaceshipController::Player(PlayerControllerConfig {
                input_mapping,
                speed_cap: Some(MetersPerSecond(1_000.0)),
            }),
            hull: ShipSource::Inline(ShipHull {
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
                ..default()
            }),
            ..default()
        };

        let scenario = ScenarioConfig {
            description: "serde smoke".to_string(),
            events: vec![ScenarioEventConfig {
                label: None,
                name: EventConfig::OnStart,
                once: false,
                filters: vec![],
                actions: vec![
                    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
                        base: BaseScenarioObjectConfig {
                            id: "rock".to_string(),
                            name: "Rock".to_string(),
                            position: Meters3::new(10.0, 20.0, 30.0),
                            rotation: Quat::IDENTITY,
                        },
                        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                            material: KIND_ROCK.to_string(),
                            destroy_sound: None,
                            radius: Meters(50.0),
                            texture: AssetRef::from("textures/rock.png"),
                            mass: None,
                            invulnerable: false,
                            seed: None,
                            lock_signature: None,
                        }),
                    }),
                    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
                        base: BaseScenarioObjectConfig {
                            id: "beacon_1".to_string(),
                            name: "Beacon".to_string(),
                            position: Meters3::new(100.0, 0.0, 0.0),
                            rotation: Quat::IDENTITY,
                        },
                        kind: ScenarioObjectKind::Beacon(BeaconConfig {
                            label: "BEACON 1".to_string(),
                            radius: Meters(20.0),
                            color: Color::srgb(0.3, 0.9, 1.0),
                            area_radius: Some(Meters(400.0)),
                            lock_signature: None,
                        }),
                    }),
                    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
                        base: BaseScenarioObjectConfig {
                            id: "player".to_string(),
                            name: "Player".to_string(),
                            position: Meters3::ZERO,
                            rotation: Quat::IDENTITY,
                        },
                        kind: ScenarioObjectKind::Spaceship(ship),
                    }),
                ],
            }],
            ..ScenarioConfig::new(
                "roundtrip".to_string(),
                "Round Trip".to_string(),
                AssetRef::from("scenarios/space.cube.png"),
            )
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
        assert_eq!(rock_kind.radius, Meters(50.0));

        // The player ship's bindings come back as the sources they went in as.
        let EventActionConfig::SpawnScenarioObject(player) = &actions[2] else {
            panic!("third action is the ship spawn");
        };
        let ScenarioObjectKind::Spaceship(ship_kind) = &player.kind else {
            panic!("third spawn is a spaceship");
        };
        let SpaceshipController::Player(player_config) = &ship_kind.controller else {
            panic!("the ship is player-controlled");
        };
        assert_eq!(player_config.speed_cap, Some(MetersPerSecond(1_000.0)));
        assert_eq!(
            player_config.input_mapping.get("thruster"),
            Some(&bindings),
            "the keyboard + mouse bindings round-trip unchanged"
        );
    }

    /// The `thumbnail`/`hidden`/`menu_backdrop`/`skybox_brightness` fields are
    /// serde-defaulted, so a scenario RON authored before they existed still
    /// parses, and a scenario carrying them round-trips. Guards the
    /// back-compat contract the picker, the menu-backdrop rotation and the
    /// skybox applier depend on.
    #[test]
    fn thumbnail_and_hidden_default_when_absent_and_round_trip_when_present() {
        // Legacy shape: none of the optional fields.
        let legacy = r#"(id: "legacy", name: "Legacy", description: "old", cubemap: "sky.png")"#;
        let parsed: ScenarioConfig = ron::from_str(legacy).expect("legacy scenario parses");
        assert_eq!(parsed.thumbnail, None, "absent thumbnail defaults to None");
        assert!(!parsed.hidden, "absent hidden defaults to false");
        assert!(
            !parsed.menu_backdrop,
            "absent menu_backdrop defaults to false"
        );
        assert_eq!(
            parsed.skybox_brightness, DEFAULT_SKYBOX_BRIGHTNESS,
            "absent skybox_brightness comes up at the shipped default"
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
            skybox_brightness: 250.0,
            watches: vec![],
            events: vec![],
        };
        // `ron::to_string` is compact (no spaces after colons).
        let ron = ron::to_string(&configured).expect("configured scenario serializes");
        assert!(ron.contains("thumbnail:Some(\"thumb.png\")"), "ron: {ron}");
        assert!(ron.contains("hidden:true"), "ron: {ron}");
        assert!(ron.contains("menu_backdrop:true"), "ron: {ron}");
        assert!(ron.contains("skybox_brightness:250"), "ron: {ron}");
        let back: ScenarioConfig = ron::from_str(&ron).expect("configured scenario parses");
        assert_eq!(
            back.thumbnail
                .and_then(|t| t.path().map(String::from))
                .as_deref(),
            Some("thumb.png")
        );
        assert!(back.hidden);
        assert!(back.menu_backdrop);
        assert_eq!(back.skybox_brightness, 250.0);

        // The defaulted form omits the keys.
        let bare = ron::to_string(&parsed).expect("legacy re-serializes");
        assert!(!bare.contains("thumbnail"), "ron: {bare}");
        assert!(!bare.contains("hidden"), "ron: {bare}");
        assert!(!bare.contains("menu_backdrop"), "ron: {bare}");
        assert!(!bare.contains("skybox_brightness"), "ron: {bare}");
    }

    /// A campaign parses from a HAND-WRITTEN RON string (not just a
    /// self-authored round-trip) and round-trips, preserving member order
    /// including hidden ids. A campaign is a first-class content entity, so
    /// this documented author-facing syntax is the contract the picker reads.
    #[test]
    fn campaign_parses_from_authored_ron_and_round_trips() {
        // As an author would WRITE it: id, display name, ordered member ids.
        // Parsing this literal (not a serialize-then-deserialize of our own
        // value) is what proves the documented syntax actually loads.
        let authored = r#"(
            id: "nova_protocol",
            name: "Nova Protocol",
            scenarios: ["shakedown_run", "broadside", "broadside_gunship", "lifeline", "final_tally"],
        )"#;
        let campaign: CampaignConfig = ron::from_str(authored).expect("campaign parses");
        assert_eq!(campaign.id, "nova_protocol");
        assert_eq!(campaign.name, "Nova Protocol");
        assert_eq!(
            campaign.scenarios,
            vec![
                "shakedown_run",
                "broadside",
                "broadside_gunship",
                "lifeline",
                "final_tally",
            ],
            "member ids parse in declared order, hidden ones included"
        );

        // Round-trips through serialize -> deserialize unchanged.
        let ron = ron::to_string(&campaign).expect("campaign serializes");
        let back: CampaignConfig = ron::from_str(&ron).expect("campaign re-parses");
        assert_eq!(back, campaign, "campaign round-trips");
    }
}
