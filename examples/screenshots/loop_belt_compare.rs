//! loop_belt_compare: one scattered belt of four asteroid kinds, flown
//! through - the v0.13.0 half of the news page's belt split.
//!
//! Two `ScatterObjects` actions lay the field either side of a straight
//! flight line, each from its own seed. A rock's position, silhouette seed and
//! radius come from the seed's own streams, and its kind from a third, so the
//! belt the v0.12.0 capsule scene `belt-compare` scatters from the same seeds
//! into the same boxes is the same belt rock for rock; the one difference
//! between the halves is what each rock is made of. The lens flies the same
//! line through both.
//!
//! The field is frozen the frame it lands, so a rock is where its seed put it
//! for the whole take and no release's physics moves one.
//!
//! Two harnessed modes, the fleet's capture idiom:
//! - `NOVA_AUTOPILOT=1`: smoke path - load the belt, fly it, exit clean.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also record the flight as
//!   `news-0130-belt-after.webm` (staged under `NOVA_CAPTURE_DIR`).

use bevy::prelude::*;
use nova_protocol::prelude::*;

/// The two halves of the belt: an id prefix, a seed, and the box each is
/// scattered into, in meters. The boxes stand either side of the flight line
/// (x = 0), a clear lane wide enough that the widest rock's surface stays off
/// the lens.
const FIELDS: [(&str, u64, Meters3, Meters3); 2] = [
    (
        "belt_port_",
        0x0130_BE17,
        Meters3::new(-200.0, -90.0, -520.0),
        Meters3::new(-60.0, 90.0, 200.0),
    ),
    (
        "belt_starboard_",
        0x0130_BE18,
        Meters3::new(60.0, -90.0, -520.0),
        Meters3::new(200.0, 90.0, 200.0),
    ),
];
/// Rocks per half.
const ROCKS_PER_FIELD: u32 = 24;
/// The nominal radius each rock is drawn in. The noise mesh reaches up to
/// `ASTEROID_GEOMETRIC_FACTOR_MAX` times this, so a 7 m rock is a 40 m body.
const ROCK_RADIUS: (Meters, Meters) = (Meters(3.0), Meters(7.0));
/// Two of the widest bodies side by side, so no two rocks land in contact.
const ROCK_SEPARATION: Meters = Meters(84.0);
/// What the belt is made of: mostly stone, the rest split three ways.
const KINDS: [(&str, u32); 4] = [
    (KIND_ROCK, 9),
    (KIND_METAL, 3),
    (KIND_ICE, 3),
    (KIND_CARBON, 3),
];

/// The flight line, in meters: straight down the lane, looking ahead.
#[cfg(feature = "debug")]
const DRIFT_FROM: Meters3 = Meters3::new(0.0, 0.0, 260.0);
#[cfg(feature = "debug")]
const DRIFT_TO: Meters3 = Meters3::new(0.0, 0.0, -180.0);
#[cfg(feature = "debug")]
const LOOK_AHEAD: Meters3 = Meters3::new(0.0, 0.0, -100.0);
/// How long the flight, and the loop, runs.
#[cfg(feature = "debug")]
const LOOP_SECS: f32 = 9.0;

fn main() -> bevy::app::AppExit {
    let mut app = AppBuilder::new().with_game_plugins(belt_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
        app.add_plugins(belt_script());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_systems(Update, (freeze_bodies, drift_lens));
    }

    app.run()
}

fn belt_plugin(app: &mut App) {
    #[cfg(feature = "debug")]
    app.init_resource::<DriftClock>();
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_belt);
}

/// The sim second the flight started on; `None` until the script starts it.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct DriftClock(Option<f32>);

fn load_belt(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.trigger(LoadScenario(belt(&game_assets)));
}

fn belt(game_assets: &GameAssets) -> ScenarioConfig {
    ScenarioConfig {
        description: "A belt of four kinds either side of a flight line".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: FIELDS
                .iter()
                .map(|(prefix, seed, min, max)| field(game_assets, prefix, *seed, *min, *max))
                .chain(ThreePointRig::around("belt", Meters3::ZERO, 8.0).actions())
                .collect(),
        }],
        ..ScenarioConfig::new(
            "loop_belt_compare".to_string(),
            "Belt of Kinds".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// One half of the belt, scattered from one seed into one box.
fn field(
    game_assets: &GameAssets,
    prefix: &str,
    seed: u64,
    min: Meters3,
    max: Meters3,
) -> EventActionConfig {
    EventActionConfig::ScatterObjects(ScatterObjectsConfig {
        id_prefix: prefix.to_string(),
        count: ROCKS_PER_FIELD,
        seed,
        region: ScatterRegion::Box { min, max },
        template: ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: prefix.to_string(),
                name: "Belt Rock".to_string(),
                position: Meters3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                radius: ROCK_RADIUS.0,
                texture: game_assets.asteroid_texture.clone().into(),
                material: KIND_ROCK.to_string(),
                destroy_sound: None,
                mass: None,
                invulnerable: false,
                lock_signature: None,
                seed: None,
            }),
        },
        asteroid_radius: Some(ROCK_RADIUS),
        asteroid_kinds: KINDS
            .iter()
            .map(|(kind, weight)| (kind.to_string(), *weight))
            .collect(),
        min_separation: Some(ROCK_SEPARATION),
    })
}

/// Fly the lens down the lane from the clock, looking ahead the whole way.
#[cfg(feature = "debug")]
fn drift_lens(world: &mut World) {
    let Some(started) = world.resource::<DriftClock>().0 else {
        return;
    };
    let now = world.resource::<Time>().elapsed_secs();
    let along = ((now - started) / LOOP_SECS).clamp(0.0, 1.0);
    let eye = Meters3(DRIFT_FROM.get().lerp(DRIFT_TO.get(), along));
    pose_camera(world, eye, eye + LOOK_AHEAD);
}

#[cfg(feature = "debug")]
const LOOP_NAME: &str = "news-0130-belt-after";

/// The belt has landed: at least one rock stands.
#[cfg(feature = "debug")]
fn belt_present() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .try_query_filtered::<Entity, With<AsteroidMarker>>()
            .is_some_and(|mut rocks| rocks.iter(world).next().is_some())
    })
}

/// The scenario is built: its spawn queue has drained and its art is in. The
/// spawn queue lands over several frames and the rig's lights come after the
/// scene, so a loop opened on the first rock would open unlit.
#[cfg(feature = "debug")]
fn settled() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let queue_drained = world
            .get_resource::<NovaEventWorld>()
            .is_some_and(|scenario| !scenario.is_settling());
        let art_in = !world
            .get_resource::<ScenarioPreload>()
            .is_some_and(|preload| preload.is_pending());
        queue_drained && art_in
    })
}

/// Take the HUD down and stand the lens on the line's start.
#[cfg(feature = "debug")]
fn frame_belt(world: &mut World) {
    hide_hud(world);
    pose_camera(world, DRIFT_FROM, DRIFT_FROM + LOOK_AHEAD);
}

/// Start the flight and open the loop on the same frame.
#[cfg(feature = "debug")]
fn open_flight(world: &mut World) {
    let now = world.resource::<Time>().elapsed_secs();
    world.resource_mut::<DriftClock>().0 = Some(now);
    loop_start(world, LOOP_NAME);
}

#[cfg(feature = "debug")]
fn belt_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the belt")
        .enter(GameStates::Loading)
        .until(and(
            state_is(GameStates::Playing),
            and(belt_present(), settled()),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("frame the lane")
        .on_enter(frame_belt)
        .until(elapsed(0.5))
        .add()
        .step("open the loop and fly the lane")
        .on_enter(open_flight)
        .until(elapsed(LOOP_SECS))
        .add()
        .step("close the loop")
        .on_enter(|world| loop_end(world, LOOP_NAME))
        .until(loop_written(LOOP_NAME))
        .deadline(60.0)
        .add()
}
