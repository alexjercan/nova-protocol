//! Scenario load, unload, and teardown: the observers that swap one running
//! scenario for the next, plus the scenario-liveness system gating.

use bevy::prelude::*;
use bevy_common_systems::prelude::*;
use bevy_enhanced_input::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;

use super::{scenario_is_live, ScenarioLoaded};
use crate::prelude::*;

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
    // NOTE: deliberately NOT gated: the PostUpdate instance of
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
    // ship, so it is despawned and rebuilt EMPTY on every (re)load, and bcs
    // only repaints it when `GameObjectives` is `resource_changed`. Without
    // this reset, restarting the SAME scenario re-posts identical objectives,
    // the write-on-diff sync (`state_to_world_system`) sees no change, the
    // resource never re-flags, and the fresh panel stays blank - the objective
    // still works but its text is gone. Guarded so an already-empty mirror is
    // not spuriously re-flagged (mirrors the sync's write-on-diff discipline).
    if let Some(objectives) = objectives {
        if !objectives.objectives.is_empty() {
            objectives.objectives.clear();
        }
    }
    // The comms mirror dies with the scenario for the SAME reason: the sync's
    // write-on-diff is length-only, so a retry whose reload pushes as many
    // lines as the old scenario had (clear + repush inside one sync window)
    // would never rewrite the feed and the new scenario's OPENING story line
    // would silently vanish. Clearing here makes the empty state observable to
    // the panel's reset before the next scenario's lines arrive. Guarded like
    // the objectives.
    if let Some(story_feed) = story_feed {
        if !story_feed.0.is_empty() {
            story_feed.0.clear();
        }
    }
    // Scenario-driven HUD emphasis dies with the scenario: a leaked emphasis
    // would pulse a verb row into the next scenario (or the menu's death gap)
    // with nothing left to clear it - the same reset class as the objectives
    // diff. Optional: headless rigs run without the HUD.
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

pub(super) fn unload_scenario(
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

pub(super) fn on_load_scenario(
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
    // The runtime content gate: a scenario with Error-level findings REFUSES to
    // start - better a clear failure than a silently half-spawned scene.
    // Checked BEFORE teardown so whatever was on screen stays; the stale
    // outcome overlay is cleared so the FAILED TO START modal does not stack
    // under it.
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

    // `SfxListenerMarker` makes this the explicit
    // SFX/juice listener (attenuation, camera shake, flash facing); the editor
    // camera deliberately never carries it.
    //
    // The skybox goes on DEFERRED (PendingSkyboxSwap, the SetSkybox action's
    // applier): the bcs skybox setup observer reads the image out of
    // `Assets<Image>` the instant a `SkyboxConfig` lands and panics on a
    // not-yet-loaded handle. Preloaded cubemaps (the GameAssets set) apply on
    // the next frame; a cubemap the collection does NOT preload - broadside's
    // alt sky, any mod shipping its own - loads in and applies when ready
    // instead of crashing the load.
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

    commands.spawn((
        ScenarioScopedMarker,
        Name::new(format!("Scenario Input Context: {}", scenario.name)),
        ScenarioInputMarker,
        actions!(
            ScenarioInputMarker[(
                Name::new("Input: Next Scenario"),
                Action::<NextScenarioInput>::new(),
                // DPadDown: moved off South, which ORBIT now uses (input/player/flight_rig.rs).
                bindings![KeyCode::Enter, GamepadButton::DPadDown]
            )]
        ),
    ));

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

    let loaded = ScenarioLoaded::from_config(&scenario);
    debug!(
        "on_load_scenario: loaded scenario '{}' with {} handler(s) and {} object(s)",
        loaded.scenario_id, loaded.handler_count, loaded.object_count
    );
    commands.trigger(loaded);

    commands.fire::<OnStartEvent>(OnStartEventInfo);
}

pub(super) fn on_add_entity_with<T: Component>(
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
pub(super) struct ScenarioInputMarker;

#[derive(InputAction)]
#[action_output(bool)]
pub(super) struct NextScenarioInput;

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
    // A shown outcome frame pauses the sim, but its [Enter] advance is exactly
    // the input we want live under that pause - the overlay is a paused modal
    // with its own Continue/Retry route. A plain pause-menu pause (no outcome)
    // still swallows the key, so ESC mid-scenario cannot skip ahead.
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

pub(super) fn on_next_input(
    _: On<Start<NextScenarioInput>>,
    mut world: ResMut<crate::world::NovaEventWorld>,
    pause: Res<State<PauseStates>>,
    outcome: Option<Res<CurrentOutcome>>,
    mut game_state: Option<ResMut<NextState<GameStates>>>,
) {
    // NOTE: observers bypass system-set gating; freeze intent changes while the pause
    // overlay is up. Releases stay ungated so held keys clear cleanly during a
    // pause.
    let paused = pause.get().is_frozen();
    let has_queued = world.next_scenario.is_some();
    let has_outcome = outcome.map(|o| o.0.is_some()).unwrap_or(false);

    match decide_advance(paused, has_queued, has_outcome) {
        AdvanceDecision::ReleaseQueued => {
            world.release_lingering_next();
        }
        AdvanceDecision::ExitToMenu => {
            // NOTE: optional - headless rigs without the states plugin have no
            // NextState resource, and the input must not panic there.
            if let Some(state) = game_state.as_deref_mut() {
                state.set(GameStates::MainMenu);
            }
        }
        AdvanceDecision::Ignore => {}
    }
}

/// Marks the scenario's free-fly camera (the one spawned by
/// `on_load_scenario`, carrying [`WASDCameraController`] until a player ship
/// swaps it to the chase camera). The `SetCamera` scenario action
/// ([`SetCameraActionConfig`]) queries
/// this to pose the camera for a scripted screenshot.
#[derive(Component, Debug, Clone)]
pub struct ScenarioCameraMarker;

pub(super) fn on_player_spaceship_spawned(
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

pub(super) fn on_player_spaceship_destroyed(
    _remove: On<Remove, PlayerSpaceshipMarker>,
    mut commands: Commands,
    camera: Single<Entity, With<SpaceshipCameraController>>,
) {
    trace!("on_player_spaceship_destroyed: switching camera back to WASD");

    let camera = camera.into_inner();

    // NOTE: plain remove/insert are safe here even during the unload sweep: this
    // observer's commands apply BEFORE the queue's remaining despawns (pinned by
    // `a_player_ships_despawn_does_not_race_the_cameras`), and the `Single` only
    // resolves a live camera; when the camera's despawn applied first, the
    // observer skips entirely. Only commands targeting the entity
    // whose OWN despawn triggered the observer race - that is
    // remove_maneuver_telemetry's case, not this one.
    commands
        .entity(camera)
        .remove::<SpaceshipCameraController>()
        .insert(WASDCameraController);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::fixtures::*;

    /// The runtime content gate: an Error-flagged scenario REFUSES to start -
    /// nothing scenario-scoped spawns, the failure report is set for the
    /// overlay, and a stale outcome is cleared so its overlay cannot stack
    /// under the modal. The clean control proves the same rig DOES load without
    /// the flag (delivery guard).
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
                auto_advance_secs: None,
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

    /// The loader's skybox install is DEFERRED: an eager `SkyboxConfig` insert
    /// panics inside the bcs setup observer for any cubemap not already sitting
    /// in `Assets<Image>` - which is every non-preloaded scenario/mod sky. Pin
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

    /// The scenario-advance decision table. A plain pause (pause menu, no
    /// outcome) always wins; a queued switch beats the outcome fallback (a
    /// Defeat with a queued retry must retry, not exit); an outcome with
    /// nothing queued exits to the menu; bare Enter mid-scenario stays inert.
    /// The outcome frame now FREEZES the sim (it enters Paused), so its Enter
    /// advance must stay live UNDER that pause - `paused && has_outcome` is no
    /// longer ignored.
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

    /// A declared outcome dies with its scenario: the unload teardown resets
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
            auto_advance_secs: None,
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
    /// through the log.
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

    /// ORDERING PIN: cross-entity observer commands do NOT race the unload
    /// sweep. Sweep-style queue [despawn(ship), despawn(camera)]: the ship's
    /// despawn fires `on_player_spaceship_destroyed` while the camera is still
    /// live, and bevy applies the observer's remove+insert BEFORE the camera's
    /// pending despawn - probed by sabotage during the task: the plain commands
    /// produced NO warn in this exact rig, refuting the assumed race (and when
    /// the camera despawns first, the `Single` fails and the observer skips).
    /// The plain commands in the observer are therefore correct; if bevy ever
    /// moves observer commands behind the pending queue (breadth-first), this
    /// test fails with "Entity despawned" and the observer needs try_ variants.
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
    /// Scenario teardown clears the HUD hint emphasis: a leaked emphasis would
    /// pulse a verb row into the next scenario with nothing left to clear it.
    /// Driven through the real UnloadScenario observer; scoped entities and the
    /// event world are cleared by the same helper, so the emphasis assert is
    /// the new behavior under test.
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

    /// 1: the comms mirror must be CLEARED at teardown, or a reload that pushes
    /// as many story lines as the old scenario had (retry, or chapter chain
    /// with equal counts) slips the sync's length-only diff and the NEW
    /// scenario's opening line never reaches the HUD. Two scenarios, one line
    /// each, different text: the feed must say the second scenario's line after
    /// the switch.
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
                icon: None,
            });
        app.world_mut()
            .resource_mut::<StoryFeed>()
            .0
            .push(StoryLine {
                speaker: "Okono".to_string(),
                text: "alpha".to_string(),
                dwell: None,
                icon: None,
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
                icon: None,
            });
        app.update();
        app.update();
        let feed = app.world().resource::<StoryFeed>();
        assert_eq!(feed.0.len(), 1);
        assert_eq!(feed.0[0].text, "beta", "the new scenario's line displays");
    }

    /// Ticks counted per gated set, so each probe proves its own schedule's
    /// gate.
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
}
