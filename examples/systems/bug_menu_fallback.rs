//! bug_menu_fallback: the menu's bare-camera branch is a TEARDOWN, not just a
//! camera.
//!
//! The mod-set case, walked over the shipped app: a leg is live, and the front
//! door the player backs out to has nothing to draw behind it - every backdrop
//! the content published is unflagged and the only flagged one left is refused
//! by the runtime content gate.
//!
//! ONE SUBJECT: what the no-clean-backdrop branch leaves behind. The bare
//! camera is the easy half. The half this range exists for is that menu entry
//! ENDS whatever was running whether or not it finds a scene to load: the
//! branch used to return before the unload, and the scenario the player left
//! went on simulating behind the menu, its clocks running under a screen with
//! no way back to it.
//!
//! The claim is an ORDER, so it is read every frame rather than once at the
//! end: for as long as the fallback camera stands, no scenario may be current
//! and no scoped entity may be alive. A teardown that arrives a frame late
//! shows up as an overlap, not as a clean final state.
//!
//! The fixture is built in Rust rather than shipped as a broken mod: the menu
//! filters its draw on `ContentIssues`, and a lint finding put there directly
//! is the same input a broken mod on disk would produce, held to the same
//! grammar by the compiler. It is registered in [`RuntimeScenarioSystems`],
//! the ordering handle for a scenario with no content file behind it, so the
//! merge that publishes the content has already run - and it re-runs on the
//! content restart that
//! backing out of gameplay performs, which is what keeps the backdrops dark
//! for the menu entry this range is about.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example bug_menu_fallback --features debug
//! # look for: `menu_fallback: the menu came up on a bare camera`,
//! #           `menu_fallback: nothing is left running behind it`,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "bug_menu_fallback")]
#[command(version = "1.0.0")]
#[command(
    about = "A menu with no clean backdrop ends the scenario it came from. Autopilot-only correctness range - play the game to use the menu",
    long_about = None
)]
struct Cli;

/// The live leg the run backs out of.
#[cfg(feature = "debug")]
const SCENARIO_A: &str = "menu_fallback_probe_live";
/// The only `role: Backdrop` scenario left standing, and BROKEN: the draw
/// refuses it, which is what empties the pick.
#[cfg(feature = "debug")]
const BROKEN_BACKDROP: &str = "menu_fallback_probe_backdrop";
/// The content error filed against it - the shape a mod that removed a
/// backdrop's ship leaves behind.
#[cfg(feature = "debug")]
const BACKDROP_ISSUE: &str = "unknown section prototype 'menu_fallback_probe_ghost'";

/// The player object [`SCENARIO_A`] spawns.
#[cfg(feature = "debug")]
const PLAYER_ID: &str = "player_ship";
/// The variable it seeds, which lives in the event world.
#[cfg(feature = "debug")]
const LIVE_MARK: &str = "in_menu_fallback_probe";

/// The camera the empty draw puts up, by `Name`.
#[cfg(feature = "debug")]
const FALLBACK_CAMERA: &str = "Menu Fallback Camera";
/// The pause menu's way out, and the front door it reaches.
#[cfg(feature = "debug")]
const BACK_TO_MENU_BUTTON: &str = "Back To Menu Button";
#[cfg(feature = "debug")]
const NEW_GAME_BUTTON: &str = "New Game Button";

/// Seconds a boot, a content restart or a return to the menu is given. Sized to
/// outlast a software-rendered CI GPU and kept under the harness completion
/// deadline, so a stall names THIS beat.
#[cfg(feature = "debug")]
const BOOT_SECS: f32 = 90.0;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // The binary's `--scenario <id>`: the launch lands on the same
    // OnEnter(Playing) loader a click on Play uses, and the app keeps its menu
    // - which is where this range is going.
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
            register_probe_fixture.in_set(RuntimeScenarioSystems),
        );
        // ...and held there. A live re-merge republishes the content's own
        // scenarios - flags and all - whenever the installed set is touched,
        // which the boot itself does; without this the draw fills back up
        // between the registration and the menu entry it is for.
        app.add_systems(
            PostUpdate,
            hold_the_draw_empty
                .in_set(RuntimeScenarioSystems)
                .run_if(a_backdrop_could_still_draw),
        );
        app.init_resource::<FallbackWatch>();
        // In `Last`, so a frame is judged on what the whole frame did rather
        // than on the state some earlier system had got to.
        app.add_systems(Last, watch_the_fallback_camera);
        // No frame-time claim: the run is a load, a back-out and a menu, and
        // the number that would come out of that window says nothing about the
        // game's speed.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(menu_fallback_script());
    }

    app.run()
}

/// What the fallback camera stood over, counted frame by frame.
///
/// The ordering claim in its exact shape: the camera and the teardown are
/// queued by ONE system, so a camera that ever shares a frame with a live
/// scenario is a teardown that came second (or not at all).
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct FallbackWatch {
    frames: usize,
    over_a_scenario: usize,
    over_scoped_entities: usize,
    most_cameras: usize,
}

#[cfg(feature = "debug")]
fn watch_the_fallback_camera(
    mut watch: ResMut<FallbackWatch>,
    q_cameras: Query<&Name, With<Camera3d>>,
    q_scoped: Query<(), With<ScenarioScopedMarker>>,
    current: Option<Res<CurrentScenario>>,
) {
    let cameras = q_cameras
        .iter()
        .filter(|name| name.as_str() == FALLBACK_CAMERA)
        .count();
    if cameras == 0 {
        return;
    }
    watch.frames += 1;
    watch.most_cameras = watch.most_cameras.max(cameras);
    if current.is_some_and(|current| current.0.is_some()) {
        watch.over_a_scenario += 1;
    }
    if q_scoped.iter().next().is_some() {
        watch.over_scoped_entities += 1;
    }
}

/// Empty the backdrop draw and register the leg the run plays.
///
/// Two halves, because the draw has two ways to come up empty and a mod set
/// reaches both at once: every backdrop the content published loses its flag
/// (the ABSENT half), and the one flagged scenario left carries an Error-level
/// finding (the ERRORING half, which the draw filters out because the loader
/// would refuse it).
///
/// It runs on every `OnEnter(GameAssetsStates::Loaded)`, which is what keeps
/// the fixture standing across the content restart that leaving gameplay
/// performs - a merge republishes the content's own flags.
#[cfg(feature = "debug")]
fn register_probe_fixture(
    mut scenarios: ResMut<GameScenarios>,
    issues: Option<ResMut<ContentIssues>>,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
) {
    for scenario in scenarios.values_mut() {
        if scenario.role == ScenarioRole::Backdrop {
            scenario.role = ScenarioRole::Chapter;
        }
    }

    let live = menu_fallback_probe_live(&game_assets, &sections);
    let backdrop = menu_fallback_probe_backdrop(&game_assets);
    scenarios.insert(live.id.clone(), live);
    scenarios.insert(backdrop.id.clone(), backdrop);

    let mut issues = issues.expect("the content merge files its findings before the menu opens");
    file_the_backdrop_error(&mut issues);
}

/// The finding the draw filters [`BROKEN_BACKDROP`] out on.
#[cfg(feature = "debug")]
fn file_the_backdrop_error(issues: &mut ContentIssues) {
    issues.0.insert(
        BROKEN_BACKDROP.to_string(),
        vec![LintIssue {
            severity: LintSeverity::Error,
            scenario: BROKEN_BACKDROP.to_string(),
            message: BACKDROP_ISSUE.to_string(),
        }],
    );
}

/// Whether the backdrop draw could still take something: a flagged scenario
/// the content gate has nothing against.
#[cfg(feature = "debug")]
fn a_backdrop_could_still_draw(
    scenarios: Option<Res<GameScenarios>>,
    issues: Option<Res<ContentIssues>>,
) -> bool {
    // Optional rather than `resource_exists`-gated: neither resource exists
    // for the first frames of a boot, and a condition that reads them
    // unguarded is a command error before the game has any content.
    let (Some(scenarios), Some(issues)) = (scenarios, issues) else {
        return false;
    };
    scenarios.values().any(|scenario| {
        scenario.role == ScenarioRole::Backdrop && issues.errors(&scenario.id).is_empty()
    })
}

/// Put the draw back where [`register_probe_fixture`] left it after a re-merge
/// has republished the content's own flags.
#[cfg(feature = "debug")]
fn hold_the_draw_empty(mut scenarios: ResMut<GameScenarios>, mut issues: ResMut<ContentIssues>) {
    for scenario in scenarios.values_mut() {
        if scenario.id != BROKEN_BACKDROP && scenario.role == ScenarioRole::Backdrop {
            scenario.role = ScenarioRole::Chapter;
        }
    }
    file_the_backdrop_error(&mut issues);
}

/// The leg the player is in: a player ship, a seeded variable and a lit arena.
///
/// It carries a ship and a variable because those are what a leak shows up as -
/// an empty scenario would prove the teardown ran without proving what it
/// reached.
#[cfg(feature = "debug")]
fn menu_fallback_probe_live(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
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
                key: LIVE_MARK.to_string(),
                expression: nova_authoring::scenario_helpers::number(1.0),
            }),
        ]
        // The scene lights itself: the engine spawns no light, so a scenario
        // that authors none renders black.
        .into_iter()
        .chain(ThreePointRig::around("arena", Meters3::ZERO, 2.0).actions())
        .collect(),
    }];

    ScenarioConfig {
        description: "A live leg the player backs out of.".to_string(),
        events,
        ..ScenarioConfig::new(
            SCENARIO_A.to_string(),
            "Menu Fallback Probe: The Leg".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The one scenario still carrying `role: Backdrop`, and broken: the draw reads
/// the same `ContentIssues` the loader would refuse it on, so it never enters
/// the pick and the menu takes the bare-camera branch.
#[cfg(feature = "debug")]
fn menu_fallback_probe_backdrop(game_assets: &GameAssets) -> ScenarioConfig {
    ScenarioConfig {
        description: "The only backdrop left, and the gate refuses it.".to_string(),
        role: ScenarioRole::Backdrop,
        events: vec![],
        ..ScenarioConfig::new(
            BROKEN_BACKDROP.to_string(),
            "Menu Fallback Probe: The Broken Backdrop".to_string(),
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
        design: ShipDesignSource::Inline(ShipDesign {
            sections: vec![
                SpaceshipSectionConfig {
                    id: "controller".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                    source: SectionSource::Inline(section("basic_controller_section")),
                },
                SpaceshipSectionConfig {
                    id: "hull".to_string(),
                    position: Vec3::new(0.0, 0.0, 1.0),
                    rotation: Quat::IDENTITY,
                    source: SectionSource::Inline(section("reinforced_hull_section")),
                },
            ],
            ..default()
        }),
        ..default()
    }
}

/// The walk, one beat per gesture.
#[cfg(feature = "debug")]
fn menu_fallback_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("menu_fallback: the leg opens and is live")
        .until(and(
            state_is(GameStates::Playing),
            and(player_ship_present(), scenario_variable_is(LIVE_MARK, 1.0)),
        ))
        .deadline(BOOT_SECS)
        .add()
        // The guard the whole range rests on, on both of its halves: there is
        // a flagged backdrop, and there is nothing the draw is allowed to take.
        .step("menu_fallback: the menu has nothing clean to draw")
        .on_enter(|world: &mut World| {
            let (flagged, clean) = backdrop_counts(world);
            assert!(
                flagged > 0,
                "the fixture leaves one flagged backdrop standing, so the empty \
                 draw is the GATE's doing rather than an empty registry"
            );
            assert_eq!(
                clean, 0,
                "{clean} backdrop(s) would still draw: the menu must reach its \
                 bare-camera branch for this range to be about anything"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the menu has no clean backdrop to draw",
                serde_json::json!({ "flagged": flagged, "clean": clean }),
            );
        })
        .add()
        .step("menu_fallback: the leg being left is really there")
        .on_enter(|world: &mut World| {
            assert!(
                scoped_entities(world) > 0,
                "the live leg has scenario-scoped entities to tear down"
            );
            // Armed here rather than at boot: the count is about the MENU's
            // camera, and nothing before this beat can have put one up.
            assert_eq!(
                world.resource::<FallbackWatch>().frames,
                0,
                "no fallback camera stands while the leg is being played"
            );
        })
        .add()
        // The gesture: ESC, then the pause menu's own way out.
        .step("menu_fallback: press Escape")
        .on_enter(press_key(KeyCode::Escape))
        .until(ui_node_present(BACK_TO_MENU_BUTTON))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("menu_fallback: let Escape go")
        .on_enter(release_key(KeyCode::Escape))
        .add()
        .step("menu_fallback: click Main Menu")
        .on_enter(click_named(BACK_TO_MENU_BUTTON))
        .until(pointer_pressed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        // Leaving gameplay restarts the content, so the road back to the front
        // door goes through the loading screen: the deadline is a boot's.
        .step("menu_fallback: release Main Menu")
        .on_enter(release_mouse(MouseButton::Left))
        .until(and(
            state_is(GameStates::MainMenu),
            ui_node_present(NEW_GAME_BUTTON),
        ))
        .deadline(BOOT_SECS)
        .add()
        .step("menu_fallback: the menu stands on a bare camera")
        .on_enter(|world: &mut World| {
            let fallbacks = named_cameras(world, FALLBACK_CAMERA);
            assert_eq!(
                fallbacks, 1,
                "an empty draw puts up exactly one bare camera so the interface \
                 still renders; {fallbacks} standing"
            );
            let cameras = count::<Camera3d>(world);
            assert_eq!(
                cameras, 1,
                "...and it is the only 3D camera left: {cameras} standing, so a \
                 torn-down scenario's camera is still in the frame"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the fallback camera is the menu's only 3D camera",
                serde_json::json!({ "cameras": cameras }),
            );
            info!("menu_fallback: the menu came up on a bare camera");
        })
        .add()
        .step("menu_fallback: nothing is left running behind it")
        .on_enter(|world: &mut World| {
            let scoped = scoped_entities(world);
            assert_eq!(
                scoped, 0,
                "the leg the player left is over: {scoped} scoped entities still live"
            );
            assert_eq!(
                count::<PlayerSpaceshipMarker>(world),
                0,
                "the player ship must not fly on behind the front door"
            );
            assert!(
                world
                    .get_resource::<CurrentScenario>()
                    .and_then(|current| current.0.as_ref())
                    .is_none(),
                "no scenario is current at the menu, so `scenario_is_live` is false"
            );
            let variables = world.resource::<NovaEventWorld>().variables().count();
            assert_eq!(
                variables, 0,
                "the event world goes with the scenario: {variables} variable(s) survived"
            );
            nova_probe::probe_marker(
                world,
                "outcome: a bare menu ends the scenario it came from",
                serde_json::json!({ "scoped": scoped, "variables": variables }),
            );
            info!("menu_fallback: nothing is left running behind it");
        })
        .add()
        .step("menu_fallback: the camera never stood over a live scene")
        .on_enter(|world: &mut World| {
            let watch = world.resource::<FallbackWatch>();
            let (frames, over_scenario, over_scoped, most) = (
                watch.frames,
                watch.over_a_scenario,
                watch.over_scoped_entities,
                watch.most_cameras,
            );
            assert!(
                frames > 0,
                "the watcher saw no fallback camera at all - this range proved nothing"
            );
            assert_eq!(
                over_scenario, 0,
                "the teardown is queued BEFORE the camera, so no frame may have \
                 both: the bare menu stood over a live scenario for {over_scenario} \
                 of {frames} frame(s)"
            );
            assert_eq!(
                over_scoped, 0,
                "...and none over a scenario's leftovers: {over_scoped} of {frames} \
                 frame(s) had scoped entities under the bare menu"
            );
            assert_eq!(
                most, 1,
                "one menu entry puts up one bare camera; {most} stood at once"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the fallback camera never stands over a live scene",
                serde_json::json!({ "frames": frames, "overlapping": over_scenario }),
            );
        })
        .add()
}

/// The backdrop draw's two inputs: how many scenarios carry the role, and how
/// many of those the content gate would let through.
#[cfg(feature = "debug")]
fn backdrop_counts(world: &World) -> (usize, usize) {
    let scenarios = world.resource::<GameScenarios>();
    let issues = world.get_resource::<ContentIssues>();
    let flagged: Vec<&ScenarioConfig> = scenarios
        .values()
        .filter(|scenario| scenario.role == ScenarioRole::Backdrop)
        .collect();
    let clean = flagged
        .iter()
        .filter(|scenario| issues.is_none_or(|issues| issues.errors(&scenario.id).is_empty()))
        .count();
    (flagged.len(), clean)
}

/// How many scenario-scoped entities are alive.
#[cfg(feature = "debug")]
fn scoped_entities(world: &World) -> usize {
    count::<ScenarioScopedMarker>(world)
}

/// How many entities carry `M`.
#[cfg(feature = "debug")]
fn count<M: Component>(world: &World) -> usize {
    world
        .try_query_filtered::<(), With<M>>()
        .map_or(0, |mut query| query.iter(world).count())
}

/// How many 3D cameras are called `name`.
#[cfg(feature = "debug")]
fn named_cameras(world: &mut World, name: &str) -> usize {
    let mut query = world.query_filtered::<&Name, With<Camera3d>>();
    query
        .iter(world)
        .filter(|live| live.as_str() == name)
        .count()
}
