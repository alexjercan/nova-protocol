//! system_world_bounds: a fixed-seed bounded world, left on purpose.
//!
//! Boots the shipped app (via [`editor_app`]), enables the example mod, and
//! starts its `example_bounds` chapter the way the Scenarios list does: the
//! `NewGameScenario` pick and the hand-off to Playing. Then it burns the
//! player straight out of the 2 km `bounds` sphere and reads what the content
//! authored for that.
//!
//! ONE SUBJECT: the `bounds` block of a generated world. The block is one
//! scenario area named `bounds`, spawned by the loader before `OnStart`;
//! leaving it is an ordinary `OnExit` the scenario answers itself, and the
//! engine clamps nothing: the world stays armed and streamed past the edge.
//! What a sector CONTAINS is `system_world_sectors`' range, and New Game's
//! session lifecycle is `system_open_world`'s.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_world_bounds --features debug
//! # look for: `world_bounds: the bounds area stands`,
//! #           `world_bounds: PASS one OnExit, the authored outcome showed`,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
use avian3d::prelude::{Collider, LinearVelocity};
#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use nova_protocol::nova_events::engine::GameEvent;
use nova_protocol::prelude::*;
#[cfg(feature = "debug")]
use nova_world::prelude::{SectorRoot, WorldConfig, WorldObserver};

#[derive(Parser)]
#[command(name = "system_world_bounds")]
#[command(version = "1.0.0")]
#[command(
    about = "The example mod's bounded fixed-seed world: the Scenarios hand-off starts it, the bounds area stands, the player burns out of it, and the authored OnExit fires once over a world still streaming. Autopilot-only correctness range",
    long_about = None
)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // This run's own settings: the example mod starts disabled on a fresh
    // profile, and no player's enabled set or saved world may be in the way.
    std::env::set_var(
        nova_assets::storage::CONFIG_ROOT_ENV,
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "target/example-profiles/{}-{}-{}",
            env!("CARGO_CRATE_NAME"),
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("the clock is after 1970")
                .as_nanos(),
        )),
    );

    let mut app = editor_app(true, None);

    #[cfg(feature = "debug")]
    {
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        // No frame-time claim: the range holds counts and poses.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(BoundsWatchPlugin);
        app.add_plugins(bounds_script());
    }

    app.run()
}

/// The mod and the chapter under test, as the example mod ships them.
#[cfg(feature = "debug")]
const EXAMPLE_MOD: &str = "example";
#[cfg(feature = "debug")]
const SCENARIO: &str = "example_bounds";
/// The seed and radius `example_bounds` authors.
#[cfg(feature = "debug")]
const SEED: u32 = 1701;
#[cfg(feature = "debug")]
const RADIUS: Meters = Meters(2_000.0);
/// The player ship the chapter spawns, by its scenario id.
#[cfg(feature = "debug")]
const PLAYER_ID: &str = "player_spaceship";
/// The counter the chapter's `OnExit` handler advances.
#[cfg(feature = "debug")]
const EXITS_VAR: &str = "bounds_exits";
/// The outcome text the chapter authors for leaving.
#[cfg(feature = "debug")]
const EXIT_MESSAGE: &str = "You crossed the line. Range closed.";

/// How many cells the open world keeps live: radius 2 is a 5x5x5 window.
#[cfg(feature = "debug")]
const LIVE_SECTORS: usize = 125;

/// The burn out of the bounds, held every frame so no helm assist can bleed
/// it: 2 km at this speed is a few seconds of simulation.
#[cfg(feature = "debug")]
const BURN: MetersPerSecond = MetersPerSecond(1_000.0);

/// Seconds a load or a stream gets on a software-rendered CI GPU. Under the
/// harness completion deadline, so a stall names its beat.
#[cfg(feature = "debug")]
const SESSION_SECS: f32 = 90.0;

/// What the range watches over the whole run.
#[cfg(feature = "debug")]
#[derive(Resource, Default, Debug)]
struct BoundsWatch {
    /// `OnExit` events fired for the `bounds` area, with the leaving id.
    exits: Vec<String>,
    /// Whether the burn is held this frame.
    burning: bool,
    /// How many sector roots stood before the burn.
    roots_before: usize,
}

/// The watch, wired to the events it reads.
#[cfg(feature = "debug")]
struct BoundsWatchPlugin;

#[cfg(feature = "debug")]
impl Plugin for BoundsWatchPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BoundsWatch>();
        app.add_observer(|event: On<GameEvent>, mut watch: ResMut<BoundsWatch>| {
            if event.name() != "onexit" {
                return;
            }
            let Some(data) = event.info().data.as_ref() else {
                return;
            };
            if data["id"].as_str() == Some(BOUNDS_AREA_ID) {
                let other = data["other_id"].as_str().unwrap_or_default().to_string();
                info!("world_bounds: OnExit from '{BOUNDS_AREA_ID}' by '{other}'");
                watch.exits.push(other);
            }
        });
    }
}

/// Hold the burn on the player every frame it is on.
///
/// The script's `input` hook, which runs in `PreUpdate` before the physics
/// step reads velocities, so the hull leaves at the held speed whatever the
/// helm would do with it.
#[cfg(feature = "debug")]
fn hold_burn(world: &mut World, _elapsed: f32, _frame: u32) {
    if !world.resource::<BoundsWatch>().burning {
        return;
    }
    let Some(player) = the_player(world) else {
        return;
    };
    // Engine boundary: avian integrates in world units.
    world
        .entity_mut(player)
        .insert(LinearVelocity(Vec3::new(BURN.to_engine(), 0.0, 0.0)));
}

/// The one player ship, if exactly one stands.
#[cfg(feature = "debug")]
fn the_player(world: &World) -> Option<Entity> {
    let mut players = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?;
    let mut players = players.iter(world);
    let player = players.next()?;
    players.next().is_none().then_some(player)
}

#[cfg(feature = "debug")]
fn count<M: Component>(world: &World) -> usize {
    world
        .try_query_filtered::<(), With<M>>()
        .map_or(0, |mut q| q.iter(world).count())
}

/// Every scenario area named `bounds`: its centre, radius and scoping.
#[cfg(feature = "debug")]
fn bounds_areas(world: &mut World) -> Vec<(Vec3, f32, bool)> {
    let mut areas = world.query_filtered::<(
        &EntityId,
        &GlobalTransform,
        &Collider,
        Has<ScenarioScopedMarker>,
    ), With<ScenarioAreaMarker>>();
    areas
        .iter(world)
        .filter(|(id, _, _, _)| id.0 == BOUNDS_AREA_ID)
        .map(|(_, pose, collider, scoped)| {
            let radius = collider
                .shape()
                .as_ball()
                .map_or(f32::NAN, |ball| ball.radius);
            (pose.translation(), radius, scoped)
        })
        .collect()
}

/// How far the player stands from the bounds centre, in metres.
#[cfg(feature = "debug")]
fn player_range(world: &mut World) -> Meters {
    let player = the_player(world).expect("world_bounds: exactly one player ship");
    let at = world
        .get::<GlobalTransform>(player)
        .expect("a ship has a pose")
        .translation();
    Meters::from_engine(at.length())
}

#[cfg(feature = "debug")]
fn exits_var(world: &World) -> f64 {
    match world.resource::<NovaEventWorld>().get_variable(EXITS_VAR) {
        Some(VariableLiteral::Number(value)) => *value,
        other => panic!("world_bounds: '{EXITS_VAR}' is {other:?}, not a number"),
    }
}

/// Advance once the world is armed around one player ship.
#[cfg(feature = "debug")]
fn armed_around_a_player() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world.contains_resource::<WorldConfig<NovaLayeredWorld>>() && the_player(world).is_some()
    })
}

/// Advance once the whole live window stands.
#[cfg(feature = "debug")]
fn the_window_streamed() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| count::<SectorRoot>(world) == LIVE_SECTORS)
}

/// The walk, one beat per gesture.
#[cfg(feature = "debug")]
fn bounds_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .input(hold_burn)
        .step("world_bounds: reach the main menu")
        .until(state_is(GameStates::MainMenu))
        .deadline(SESSION_SECS)
        .add()
        // The example mod ships disabled; the merge re-runs on the enabled set.
        .step("world_bounds: enable the example mod")
        .on_enter(|world: &mut World| {
            world
                .resource_mut::<EnabledMods>()
                .0
                .insert(EXAMPLE_MOD.to_string());
        })
        .until(frames(SETTLE_FRAMES * 2))
        .add()
        .step("world_bounds: start the bounded range the way the picker does")
        .on_enter(|world: &mut World| {
            let listed = world
                .resource::<GameScenarios>()
                .get(SCENARIO)
                .unwrap_or_else(|| panic!("world_bounds: '{SCENARIO}' is not registered"))
                .picker_lists();
            assert!(listed, "a fixed-seed chapter is a picker row");
            world.resource_mut::<NewGameScenario>().0 = Some(SCENARIO.to_string());
            *world.resource_mut::<GameMode>() = GameMode::NewGame;
            world
                .resource_mut::<NextState<GameStates>>()
                .set(GameStates::Playing);
        })
        .until(state_is(GameStates::Playing))
        .deadline(SESSION_SECS)
        .add()
        .step("world_bounds: the world arms around the player")
        .until(armed_around_a_player())
        .deadline(SESSION_SECS)
        .add()
        .step("world_bounds: the fixed seed armed with no session")
        .on_enter(assert_the_fixed_seed)
        .add()
        .step("world_bounds: the bounds area stands")
        .on_enter(assert_the_bounds_area)
        .add()
        .step("world_bounds: the sectors stream in")
        .until(the_window_streamed())
        .deadline(SESSION_SECS)
        .add()
        .step("world_bounds: report the stream")
        .on_enter(|world: &mut World| {
            let roots = count::<SectorRoot>(world);
            world.resource_mut::<BoundsWatch>().roots_before = roots;
            nova_probe::probe_marker(
                world,
                "outcome: the sectors stream in around the bounded range",
                serde_json::json!({ "roots": roots }),
            );
            info!("world_bounds: {roots} sectors stand around the range");
        })
        .add()
        .step("world_bounds: burn out of the bounds")
        .on_enter(|world: &mut World| {
            assert_eq!(exits_var(world), 0.0, "no exit before the burn");
            assert!(
                world.resource::<BoundsWatch>().exits.is_empty(),
                "no OnExit before the burn"
            );
            world.resource_mut::<BoundsWatch>().burning = true;
        })
        .until(scenario_variable_is(EXITS_VAR, 1.0))
        .deadline(SESSION_SECS)
        .add()
        .step("world_bounds: let the exit settle")
        .on_enter(|world: &mut World| {
            world.resource_mut::<BoundsWatch>().burning = false;
        })
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("world_bounds: one exit, the authored outcome, the world still up")
        .on_enter(assert_the_exit)
        .add()
}

/// The chapter armed its own seed with no `OpenWorldSession`, around the one
/// player ship the world streams from.
#[cfg(feature = "debug")]
fn assert_the_fixed_seed(world: &mut World) {
    let current = world
        .resource::<CurrentScenario>()
        .0
        .as_ref()
        .map(|scenario| scenario.id.clone());
    assert_eq!(current.as_deref(), Some(SCENARIO));
    nova_probe::probe_marker(
        world,
        "outcome: the bounded range is a picker row the scenarios hand-off starts",
        serde_json::json!({ "scenario": SCENARIO }),
    );

    let armed = world.resource::<WorldConfig<NovaLayeredWorld>>().seed;
    assert_eq!(armed, SEED, "the world arms on the chapter's fixed seed");
    assert!(
        !world.contains_resource::<OpenWorldSession>(),
        "a fixed seed takes no session"
    );
    let player = the_player(world).expect("world_bounds: exactly one player ship");
    assert!(
        world.get::<WorldObserver>(player).is_some(),
        "the world streams around the player ship"
    );
    assert_eq!(count::<WorldObserver>(world), 1);
    nova_probe::probe_marker(
        world,
        "outcome: the fixed seed arms the world with no session around the one player ship",
        serde_json::json!({ "seed": armed }),
    );
    info!("world_bounds: seed {armed} armed with no session");
}

/// Exactly one scenario area named `bounds`, scoped, at the authored centre
/// and radius, with the player inside it.
#[cfg(feature = "debug")]
fn assert_the_bounds_area(world: &mut World) {
    let areas = bounds_areas(world);
    assert_eq!(areas.len(), 1, "one area named bounds, got {areas:?}");
    let (centre, radius, scoped) = areas[0];
    assert!(scoped, "the bounds area dies with the scenario");
    assert!(
        centre.length() < 1e-3,
        "the area stands at the authored centre, not {centre:?}"
    );
    assert!(
        (radius - RADIUS.to_engine()).abs() < 1e-3,
        "the area carries the authored radius ({radius} u)"
    );
    let range = player_range(world);
    assert!(
        range.0 < RADIUS.0,
        "the player starts inside the bounds ({} m out)",
        range.0
    );
    nova_probe::probe_marker(
        world,
        "outcome: the bounds block spawns one scenario area named bounds",
        serde_json::json!({ "radius_m": RADIUS.0, "player_range_m": range.0 }),
    );
    info!("world_bounds: the bounds area stands");
}

/// One `OnExit` for the player, the chapter's own counter at one, the authored
/// Defeat on screen, and the world armed and streamed as before.
#[cfg(feature = "debug")]
fn assert_the_exit(world: &mut World) {
    let range = player_range(world);
    assert!(
        range.0 > RADIUS.0,
        "the player is past the radius ({} m out)",
        range.0
    );
    let exits = world.resource::<BoundsWatch>().exits.clone();
    assert_eq!(
        exits,
        vec![PLAYER_ID.to_string()],
        "exactly one OnExit from bounds, by the player"
    );
    assert_eq!(exits_var(world), 1.0, "the chapter's counter saw one exit");
    nova_probe::probe_marker(
        world,
        "outcome: leaving the bounds fires OnExit for the player exactly once",
        serde_json::json!({ "exits": exits.len(), "player_range_m": range.0 }),
    );

    let outcome = world
        .resource::<CurrentOutcome>()
        .0
        .clone()
        .expect("the authored outcome is up");
    assert_eq!(outcome.outcome, ScenarioOutcomeKind::Defeat);
    assert_eq!(outcome.message.as_deref(), Some(EXIT_MESSAGE));
    nova_probe::probe_marker(
        world,
        "outcome: the authored OnExit outcome shows",
        serde_json::json!({ "message": EXIT_MESSAGE }),
    );

    let roots = count::<SectorRoot>(world);
    let before = world.resource::<BoundsWatch>().roots_before;
    assert!(
        world.contains_resource::<WorldConfig<NovaLayeredWorld>>(),
        "the world stays armed past the edge"
    );
    assert_eq!(roots, before, "the window stands as it did before the burn");
    assert_eq!(
        bounds_areas(world).len(),
        1,
        "the bounds area outlives the exit"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the world stays armed and streamed past the edge",
        serde_json::json!({ "roots": roots }),
    );
    info!("world_bounds: PASS one OnExit, the authored outcome showed");
}
