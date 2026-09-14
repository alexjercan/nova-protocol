//! bug_outcome_pause: one modal at a time, and never an unwatched frame.
//!
//! Three things that all come down to the same question - who owns the screen,
//! and is the world running behind them - walked over the shipped app:
//!
//! 1. ALT-TAB. Losing the window pauses interactive play, with the ordinary
//!    pause menu and its ordinary Resume. Coming back does not resume: the
//!    player left, and being dropped back into a fight they are not looking at
//!    is how a run is lost to a notification.
//! 2. THE RACE. ESC and a scenario's `Outcome` action can land on the same
//!    frame. The pause menu opens first, the outcome arrives behind it, and ESC
//!    is then inert over an outcome - so the panel could neither be closed nor
//!    resumed from without unfreezing the world under the banner. The outcome
//!    takes the screen. Exactly one modal.
//! 3. THE TIMED ADVANCE. An authored `auto_advance_secs` runs on the wall
//!    clock, so a chain advances while the player is in another window. The
//!    chapter it advances INTO must not get a single unpaused frame: the
//!    outcome's pause is handed to the pause menu rather than released.
//!
//! The unpaused-frame claim is not read off a predicate at the end. A watcher
//! counts every frame whose virtual clock was running from the moment focus is
//! taken away, so one frame of a fight nobody is watching fails the range.
//!
//! The focus policy is OFF under a harness (`FocusPause`): a probe drives an X
//! display nobody is looking at, and a range that paused on its own first frame
//! would stall rather than walk. This one asserts that exemption is live and
//! then switches the player's policy on, which is what makes every pause below
//! the policy's doing rather than the harness's.
//!
//! The fixtures are two `ScenarioConfig` values built in Rust, registered in
//! [`EditorSandboxSystems`] like the editor's own sandbox so the `--scenario`
//! membership check in the same transition cannot run before the id it
//! resolves exists.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example bug_outcome_pause --features debug
//! # look for: `outcome_pause: alt-tab paused the fight`,
//! #           `outcome_pause: the outcome took the screen from the pause menu`,
//! #           `outcome_pause: the chain advanced unwatched, into a pause`,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "bug_outcome_pause")]
#[command(version = "1.0.0")]
#[command(
    about = "Focus loss pauses play, an outcome takes the screen from the pause menu, and a timed advance hands its pause on. Autopilot-only correctness range - play the game to use the menu",
    long_about = None
)]
struct Cli;

/// The fight the run opens in.
#[cfg(feature = "debug")]
const SCENARIO_A: &str = "outcome_pause_probe_a";
/// The chapter the defeat's timer chains into.
#[cfg(feature = "debug")]
const SCENARIO_B: &str = "outcome_pause_probe_b";

/// The player object both fixtures spawn.
#[cfg(feature = "debug")]
const PLAYER_ID: &str = "player_ship";
/// The variables each fixture seeds, so arrival is observed rather than
/// inferred from a switch having been queued.
#[cfg(feature = "debug")]
const IN_A: &str = "in_probe_a";
#[cfg(feature = "debug")]
const IN_B: &str = "in_probe_b";

/// The modals, by `Name`.
#[cfg(feature = "debug")]
const PAUSE_OVERLAY: &str = "Pause Overlay";
#[cfg(feature = "debug")]
const RESUME_BUTTON: &str = "Resume Button";
#[cfg(feature = "debug")]
const OUTCOME_OVERLAY: &str = "Outcome Overlay";
#[cfg(feature = "debug")]
const PAUSE_SETTINGS: &str = "Pause Settings Panel Root";

/// Seconds the authored defeat waits before taking its own chain.
///
/// Long enough that the beats between the overlay coming up and the focus
/// going away cannot be outrun by it on a software-rendered host, and short
/// enough that the run does not idle for the wait.
#[cfg(feature = "debug")]
const AUTO_ADVANCE_SECS: f64 = 8.0;

/// Seconds a boot or a scenario load is given. Sized to outlast a
/// software-rendered CI GPU and kept under the harness completion deadline, so
/// a stall names THIS beat.
#[cfg(feature = "debug")]
const BOOT_SECS: f32 = 90.0;

/// Frames the run holds still to prove the world is NOT quietly resuming.
#[cfg(feature = "debug")]
const SETTLE_FRAMES: u32 = 20;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // The binary's `--scenario <id>`: the launch lands on the same
    // OnEnter(Playing) loader a click on Play uses.
    #[cfg(feature = "debug")]
    let startup = Some(StartupScenario::Id(SCENARIO_A.to_string()));
    #[cfg(not(feature = "debug"))]
    let startup = None;

    // The same app the game/binary runs - not a bespoke copy.
    let mut app = editor_app(true, startup);

    #[cfg(feature = "debug")]
    {
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        app.add_systems(
            OnEnter(GameAssetsStates::Loaded),
            register_probe_scenarios.in_set(EditorSandboxSystems),
        );
        app.init_resource::<RunningFrames>();
        // In `Last`, so a frame is judged on what the whole frame did rather
        // than on the state some earlier system had got to.
        app.add_systems(Last, watch_for_a_running_frame);
        // No frame-time claim: the run is a pause, a modal and a chain, and the
        // number that would come out of a window held frozen on purpose says
        // nothing about the game's speed.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(outcome_pause_script());
    }

    app.run()
}

/// Frames the next chapter got to itself while the watcher was armed.
///
/// The unpaused-frame claim in its exact shape, counted two ways, because the
/// two can come apart for a frame: `ticking` is a frame whose virtual clock
/// actually ran, and `owned` is a frame the world was nobody's modal to hold -
/// the scenario load gate keeps the clocks frozen while a scene builds, so a
/// pause that was released too early can hide behind that hold for as long as
/// the build takes and start the fight the moment it ends.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct RunningFrames {
    armed: bool,
    ticking: usize,
    owned: usize,
}

#[cfg(feature = "debug")]
fn watch_for_a_running_frame(
    mut watch: ResMut<RunningFrames>,
    pause: Res<State<PauseStates>>,
    time: Res<Time<Virtual>>,
) {
    if !watch.armed {
        return;
    }
    if !time.is_paused() {
        watch.ticking += 1;
    }
    if *pause.get() == PauseStates::Unpaused {
        watch.owned += 1;
    }
}

/// Register both fixtures into the merged registry.
///
/// The launch request resolves its id against this resource, and so does the
/// queued `NextScenario` the defeat's timer releases - a chain carries an id
/// and nothing else.
#[cfg(feature = "debug")]
fn register_probe_scenarios(
    mut scenarios: ResMut<GameScenarios>,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
) {
    let fight = outcome_pause_probe_a(&game_assets, &sections);
    let next = outcome_pause_probe_b(&game_assets, &sections);
    scenarios.insert(fight.id.clone(), fight);
    scenarios.insert(next.id.clone(), next);
}

/// One player ship in a lit arena, and a death that declares a TIMED defeat
/// with the next chapter lingering behind it.
///
/// The defeat is the composed shape a campaign uses: `Outcome(Defeat)` plus a
/// LINGERING `NextScenario`, which is what gives the overlay its primary button
/// - and `auto_advance_secs`, which is what lets the chain go without one.
#[cfg(feature = "debug")]
fn outcome_pause_probe_a(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let events = vec![
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: vec![
                EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
                    base: BaseScenarioObjectConfig {
                        id: PLAYER_ID.to_string(),
                        name: "Player Ship".to_string(),
                        position: Meters3::ZERO,
                        rotation: Quat::IDENTITY,
                    },
                    kind: ScenarioObjectKind::Spaceship(probe_ship(sections)),
                }),
                EventActionConfig::VariableSet(VariableSetActionConfig {
                    key: IN_A.to_string(),
                    expression: nova_authoring::scenario_helpers::number(1.0),
                }),
            ]
            // The scene lights itself: the engine spawns no light, so a
            // scenario that authors none renders black.
            .into_iter()
            .chain(ThreePointRig::around("arena", Meters3::ZERO, 2.0).actions())
            .collect(),
        },
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnDestroyed,
            once: false,
            filters: vec![EventFilterConfig::Entity(EntityFilterConfig {
                id: Some(PLAYER_ID.to_string()),
                type_name: None,
                ..default()
            })],
            actions: vec![
                EventActionConfig::Outcome(OutcomeActionConfig {
                    outcome: ScenarioOutcomeKind::Defeat,
                    message: Some("Hull breached. Falling back.".to_string()),
                    auto_advance_secs: Some(AUTO_ADVANCE_SECS),
                }),
                EventActionConfig::NextScenario(NextScenarioActionConfig {
                    scenario_id: SCENARIO_B.to_string(),
                    linger: true,
                    delay: None,
                }),
            ],
        },
    ];

    ScenarioConfig {
        description: "A fight whose defeat takes its own chain after a while.".to_string(),
        events,
        ..ScenarioConfig::new(
            SCENARIO_A.to_string(),
            "Outcome Pause Probe: The Fight".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The chapter the chain lands in: a real playable scene, because what is being
/// proven about it is that it does NOT get to run.
#[cfg(feature = "debug")]
fn outcome_pause_probe_b(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let events = vec![ScenarioEventConfig {
        label: None,
        name: EventConfig::OnStart,
        once: false,
        filters: vec![],
        actions: vec![
            EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: PLAYER_ID.to_string(),
                    name: "Player Ship".to_string(),
                    position: Meters3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                kind: ScenarioObjectKind::Spaceship(probe_ship(sections)),
            }),
            EventActionConfig::VariableSet(VariableSetActionConfig {
                key: IN_B.to_string(),
                expression: nova_authoring::scenario_helpers::number(1.0),
            }),
        ]
        .into_iter()
        .chain(ThreePointRig::around("arena", Meters3::ZERO, 2.0).actions())
        .collect(),
    }];

    ScenarioConfig {
        description: "The chapter the timer chains into, which must not tick.".to_string(),
        events,
        ..ScenarioConfig::new(
            SCENARIO_B.to_string(),
            "Outcome Pause Probe: The Next Chapter".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The player's hull: a controller and a piece of armor, out of the base
/// catalog, so the fixture is held to the same grammar the game ships.
#[cfg(feature = "debug")]
fn probe_ship(sections: &GameSections) -> SpaceshipConfig {
    let section = |id: &str| {
        sections
            .get_section(id)
            .unwrap_or_else(|| panic!("section '{id}' not found"))
            .clone()
    };
    SpaceshipConfig {
        allegiance: None,
        controller: SpaceshipController::Player(PlayerControllerConfig::default()),
        hull: ShipSource::Inline(ShipHull {
            sections: vec![
                SpaceshipSectionConfig {
                    id: "controller".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                    source: SectionSource::Inline(section("basic_controller_section")),
                    modifications: vec![],
                },
                SpaceshipSectionConfig {
                    id: "hull".to_string(),
                    position: Vec3::new(0.0, 0.0, 1.0),
                    rotation: Quat::IDENTITY,
                    source: SectionSource::Inline(section("reinforced_hull_section")),
                    modifications: vec![],
                },
            ],
            ..default()
        }),
        ..default()
    }
}

/// The walk, one beat per gesture.
#[cfg(feature = "debug")]
fn outcome_pause_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("outcome_pause: the fight opens")
        .until(and(
            state_is(GameStates::Playing),
            and(player_ship_present(), scenario_variable_is(IN_A, 1.0)),
        ))
        .deadline(BOOT_SECS)
        .add()
        // The exemption, asserted where it is live: this process IS a scripted
        // run, and the policy it boots with is the one that keeps every other
        // range from pausing itself on its first unfocused frame.
        .step("outcome_pause: a scripted run is exempt from the focus pause")
        .on_enter(|world: &mut World| {
            let booted = *world.resource::<FocusPause>();
            assert_eq!(
                booted,
                FocusPause(false),
                "a harness run drives a display nobody is looking at: the focus \
                 pause must boot OFF, or every rendered range stalls on frame one"
            );
            nova_probe::probe_marker(
                world,
                "outcome: a scripted run is exempt from the focus pause",
                serde_json::json!({}),
            );
            // ...and from here the run walks the policy a player gets.
            world.insert_resource(FocusPause(true));
        })
        .add()
        .step("outcome_pause: alt-tab out of the fight")
        .on_enter(set_focus(false))
        .until(ui_node_present(PAUSE_OVERLAY))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("outcome_pause: the window going away pauses the fight")
        .on_enter(|world: &mut World| {
            assert!(
                world.resource::<Time<Virtual>>().is_paused(),
                "an unwatched fight does not keep being fought"
            );
            assert_eq!(
                pause_state(world),
                PauseStates::Paused,
                "the focus pause is the ORDINARY pause, on the ordinary axis"
            );
            assert_eq!(
                named_nodes(world, RESUME_BUTTON),
                1,
                "...with the ordinary Resume under the pointer"
            );
            assert_eq!(
                named_nodes(world, OUTCOME_OVERLAY),
                0,
                "nothing else has been raised behind it"
            );
            nova_probe::probe_marker(
                world,
                "outcome: losing the window pauses interactive play",
                serde_json::json!({}),
            );
            info!("outcome_pause: alt-tab paused the fight");
        })
        .add()
        .step("outcome_pause: alt-tab back in")
        .on_enter(set_focus(true))
        .until(settled(SETTLE_FRAMES))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("outcome_pause: coming back never resumes by itself")
        .on_enter(|world: &mut World| {
            assert_eq!(
                pause_state(world),
                PauseStates::Paused,
                "{SETTLE_FRAMES} frames with the window back and the game is still \
                 the player's to resume"
            );
            assert!(world.resource::<Time<Virtual>>().is_paused());
            nova_probe::probe_marker(
                world,
                "outcome: regaining the window never resumes by itself",
                serde_json::json!({ "frames": SETTLE_FRAMES }),
            );
        })
        .add()
        // Resume is the player's, through the real button.
        .step("outcome_pause: click Resume")
        .on_enter(click_named(RESUME_BUTTON))
        .until(pointer_pressed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("outcome_pause: release Resume")
        .on_enter(release_mouse(MouseButton::Left))
        .until(flying())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        // The race, staged in the order it happens: the pause menu gets there
        // first, and the scenario's Outcome lands behind it.
        .step("outcome_pause: the player's own ESC")
        .on_enter(press_key(KeyCode::Escape))
        .until(ui_node_present(PAUSE_OVERLAY))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("outcome_pause: let ESC up")
        .on_enter(release_key(KeyCode::Escape))
        .add()
        .step("outcome_pause: the outcome lands behind the pause menu")
        .on_enter(kill_object(PLAYER_ID))
        .until(ui_node_present(OUTCOME_OVERLAY))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("outcome_pause: the outcome takes the screen")
        .on_enter(|world: &mut World| {
            let panels = named_nodes(world, PAUSE_OVERLAY);
            assert_eq!(
                panels, 0,
                "the pause panel may not stack under the outcome: ESC is inert over \
                 one, so neither its Resume nor its ESC could close it ({panels} up)"
            );
            assert_eq!(
                named_nodes(world, PAUSE_SETTINGS),
                0,
                "nor its Settings modal, which draws ABOVE the outcome"
            );
            assert_eq!(
                named_nodes(world, OUTCOME_OVERLAY),
                1,
                "the outcome is the one modal left"
            );
            assert_eq!(
                pause_state(world),
                PauseStates::Paused,
                "and the freeze is continuous across the handover"
            );
            assert!(world.resource::<Time<Virtual>>().is_paused());
            nova_probe::probe_marker(
                world,
                "outcome: an outcome landing over the pause menu takes the screen",
                serde_json::json!({}),
            );
            info!("outcome_pause: the outcome took the screen from the pause menu");
        })
        .add()
        // ...and the timed chain, taken while the player is somewhere else.
        .step("outcome_pause: leave while the timer runs")
        .on_enter(set_focus(false))
        .on_enter(|world: &mut World| {
            let mut watch = world.resource_mut::<RunningFrames>();
            watch.armed = true;
            watch.ticking = 0;
            watch.owned = 0;
        })
        .until(and(
            current_scenario_is(SCENARIO_B),
            scenario_variable_is(IN_B, 1.0),
        ))
        .deadline(BOOT_SECS)
        .add()
        .step("outcome_pause: the chain advanced, into a pause")
        .on_enter(|world: &mut World| {
            let (ticking, owned) = {
                let watch = world.resource::<RunningFrames>();
                (watch.ticking, watch.owned)
            };
            assert_eq!(
                (ticking, owned),
                (0, 0),
                "the next chapter got {ticking} ticking frame(s) and {owned} frame(s) \
                 of its own with nobody watching: an outcome's pause is HANDED to the \
                 pause menu, not released"
            );
            nova_probe::probe_marker(
                world,
                "outcome: a timed advance leaves no unpaused frame",
                serde_json::json!({ "ticking": ticking, "owned": owned }),
            );

            assert_eq!(
                pause_state(world),
                PauseStates::Paused,
                "the chapter the chain landed in is the player's to start"
            );
            assert_eq!(
                named_nodes(world, OUTCOME_OVERLAY),
                0,
                "the outcome frame went with the chapter it ended"
            );
            assert_eq!(
                named_nodes(world, RESUME_BUTTON),
                1,
                "what is on screen is the ordinary pause menu, with Resume on it"
            );
            nova_probe::probe_marker(
                world,
                "outcome: a timed advance hands its pause to the pause menu",
                serde_json::json!({}),
            );
            info!("outcome_pause: the chain advanced unwatched, into a pause");
        })
        .add()
}

/// Take the window, or give it back - the alt-tab a harnessed run cannot make
/// the window manager perform.
///
/// Writes the focus the engine itself writes (`bevy_winit` sets it from the
/// winit `Focused` event and from nowhere else), so what the policy reads is
/// exactly what a real alt-tab would have left there.
#[cfg(feature = "debug")]
fn set_focus(focused: bool) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        let mut query = world.query_filtered::<&mut Window, With<bevy::window::PrimaryWindow>>();
        let mut window = query
            .single_mut(world)
            .expect("a rendered run has a primary window");
        window.focused = focused;
    }
}

/// Hold for `frames` rendered frames - the way to give the world a chance to do
/// something behind the run's back, on a clock that is deliberately stopped.
///
/// `elapsed` cannot be used here: it counts simulated seconds, and everything
/// this range waits through is frozen.
#[cfg(feature = "debug")]
fn settled(frames: u32) -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    let start = std::sync::atomic::AtomicU32::new(u32::MAX);
    std::sync::Arc::new(move |world: &World| {
        let now = world
            .get_resource::<bevy::diagnostic::FrameCount>()
            .map_or(0, |count| count.0);
        let _ = start.compare_exchange(
            u32::MAX,
            now,
            std::sync::atomic::Ordering::Relaxed,
            std::sync::atomic::Ordering::Relaxed,
        );
        now.saturating_sub(start.load(std::sync::atomic::Ordering::Relaxed)) >= frames
    })
}

/// Advance once the world is running again with a ship to fly.
#[cfg(feature = "debug")]
fn flying() -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(|world: &World| {
        world
            .get_resource::<State<PauseStates>>()
            .is_some_and(|state| *state.get() == PauseStates::Unpaused)
            && world
                .get_resource::<Time<Virtual>>()
                .is_some_and(|time| !time.is_paused())
    })
}

/// Advance once `id` is the LOADED scenario - the chain's arrival, not its
/// queueing.
#[cfg(feature = "debug")]
fn current_scenario_is(id: &'static str) -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(move |world: &World| {
        world
            .get_resource::<CurrentScenario>()
            .and_then(|current| current.0.as_ref())
            .is_some_and(|live| live.id == id)
    })
}

/// The live pause state.
#[cfg(feature = "debug")]
fn pause_state(world: &World) -> PauseStates {
    *world.resource::<State<PauseStates>>().get()
}

/// How many laid-out nodes are called `name`.
#[cfg(feature = "debug")]
fn named_nodes(world: &mut World, name: &str) -> usize {
    let mut query = world.query_filtered::<&Name, With<Node>>();
    query
        .iter(world)
        .filter(|live| live.as_str() == name)
        .count()
}

/// Kill the scenario object `id` with an overkill through the production damage
/// entry point, aimed at the node that actually carries the `Health`.
#[cfg(feature = "debug")]
fn kill_object(id: &'static str) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        let root = {
            let mut query =
                world.query_filtered::<(Entity, &EntityId), With<ScenarioScopedMarker>>();
            query
                .iter(world)
                .find(|(_, live)| live.0 == id)
                .map(|(entity, _)| entity)
        };
        let root =
            root.unwrap_or_else(|| panic!("outcome_pause: no scenario object '{id}' to kill"));
        let target = health_bearer(world, root)
            .unwrap_or_else(|| panic!("outcome_pause: scenario object '{id}' carries no Health"));
        world.trigger(HealthApplyDamage {
            entity: target,
            source: None,
            amount: 1e6,
        });
    }
}

/// The nearest `Health`-carrying node at or beneath `entity`, depth first.
#[cfg(feature = "debug")]
fn health_bearer(world: &World, entity: Entity) -> Option<Entity> {
    if world.get::<Health>(entity).is_some() {
        return Some(entity);
    }
    let children: Vec<Entity> = world
        .get::<Children>(entity)
        .map(|children| children.iter().collect())
        .unwrap_or_default();
    children
        .into_iter()
        .find_map(|child| health_bearer(world, child))
}
