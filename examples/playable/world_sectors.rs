//! world_sectors: fly the streamed world and watch it come and go.
//!
//! The hand-driven half of the streamed-world work. One empty scenario loads,
//! the free-fly camera IS the [`WorldObserver`], and the 5x5x5 sectors around
//! it are prepared off the frame, a poolful at a time, and materialized one a
//! frame while that scenario stays live. Fly far enough along an axis and the
//! sectors behind you retire while the ones ahead come up - no load screen, no
//! scenario switch, and the readout names the cell you are in, what is still
//! PREPARING, and what is waiting for a frame while it happens.
//!
//! What a human is here to judge is the part a headless assert cannot:
//! whether a boundary crossing READS as continuous space or as the world
//! rebuilding itself, and whether the world arriving a sector at a time is
//! visible as popping. `system_world_sectors` owns the counts.
//!
//! The generator and the streaming loop are `nova_world`'s; the seed, the
//! cell edge and the content tables are `examples/shared/world_fixture/mod.rs`'s,
//! shared with the range, so what is flown here is what is asserted there.
//!
//! Hand-run (WASD + right-drag to look; HOLD a key - a 32 km sector is about
//! 530 s at the base 60 m/s and about 17 s at the 32x ramp, so this one is
//! flown under way rather than drifted across):
//! ```text
//! cargo run --example world_sectors --features debug
//! # fly +X and watch the readout's cell change and twenty-five sectors swap
//! ```
//!
//! Harnessed mode, the fleet's run gate:
//! - `NOVA_AUTOPILOT=1`: load, arm, cross one boundary, come back, exit clean.
//!   This is the path `probe run` takes.

#[path = "../shared/world_fixture/mod.rs"]
pub mod world_fixture;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;
use nova_world::prelude::*;
#[cfg(feature = "debug")]
use world_fixture::EXAMPLE_ACTIVE_RADIUS;
use world_fixture::{
    free_play_scenario, uniform_world_config, world_observer_plugin, UniformAsteroids,
};

#[derive(Parser)]
#[command(name = "world_sectors")]
#[command(version = "1.0.0")]
#[command(
    about = "Fly a free-fly observer through streamed procedural sectors over one live scenario",
    long_about = None
)]
struct Cli;

/// The empty bootstrap this session runs inside.
const SCENARIO_ID: &str = "world_sectors_observer";

/// Marks the readout line.
#[derive(Component)]
struct SectorReadout;

/// How bright the observer's key light is.
///
/// The example lights itself instead of authoring `Light` objects into the
/// bootstrap, because the bootstrap has to stay EMPTY - that is the property
/// the range asserts, and a scenario with three lights in it is not it.
const KEY_ILLUMINANCE: f32 = 6_000.0;

/// Where the key light points from.
const KEY_DIRECTION: Vec3 = Vec3::new(-0.4, -1.0, -0.6);

/// In-step seconds a harnessed beat gets before the run aborts naming it.
///
/// A hang backstop, not a budget. Raised for the job lifetime: a 125-cell
/// window is 125 preparations, taken a poolful at a time, and then 125
/// separate materialization frames, and on a software rasterizer those frames
/// are the slow ones.
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 240.0;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new()
        .with_game_plugins((
            observer_plugin,
            world_observer_plugin,
            NovaWorldPlugin::<UniformAsteroids>::default(),
        ))
        .build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(observer_script());
    }

    app.run()
}

fn observer_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), boot_observer);
    app.add_systems(Update, update_readout);
}

/// Load the empty bootstrap, arm the stream, light the scene, and put the
/// readout up.
///
/// Armed immediately, unlike the range: a human opening this wants the world
/// already there, and there is no emptiness claim to protect here.
///
/// `OnEnter` rather than `Update`: nova_world requires a `WorldConfig` writer
/// to land ahead of `NovaWorldSystems::Cleanup`, and a state transition runs
/// before `Update` at all.
fn boot_observer(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.trigger(LoadScenario(free_play_scenario(
        &game_assets,
        SCENARIO_ID,
        "World Sectors Observer",
    )));
    commands.insert_resource(uniform_world_config());

    commands.spawn((
        Name::new("Observer Key Light"),
        DirectionalLight {
            illuminance: KEY_ILLUMINANCE,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_translation(Vec3::ZERO).looking_to(KEY_DIRECTION, Vec3::Y),
    ));

    // Top LEFT and two lines, not one centered line: the dev overlay's fps and
    // version bar sits along the top right, and a single full-width readout
    // ran straight through it.
    commands
        .spawn((
            Name::new("Sector Readout"),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                left: Val::Px(12.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                SectorReadout,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

/// Name the cell the observer is in, where the observer stands inside it,
/// what is live around it, and what is still on its way.
///
/// The offset is what makes a crossing readable: the metres run up to the
/// edge length and then reset as the cell index steps, which is the moment
/// twenty-five sectors swap. The PREPARING and READY counts are what make the
/// streaming readable - PREPARING is shown against its cap, so a crossing
/// shows the pool held full while twenty-five cells go through it and drain
/// back one materialization a frame, and a sector arriving late is a number on
/// screen instead of a mystery.
fn update_readout(
    config: Option<Res<WorldConfig<UniformAsteroids>>>,
    current: Option<Res<CurrentSector>>,
    ready: Res<ReadySectors>,
    observer: Query<&GlobalTransform, With<WorldObserver>>,
    roots: Query<&SectorRoot>,
    jobs: Query<&SectorJob>,
    bodies: Query<&AsteroidMarker>,
    mut readout: Query<&mut Text, With<SectorReadout>>,
) {
    let (Some(config), Some(current)) = (config, current) else {
        return;
    };
    let Ok(transform) = observer.single() else {
        return;
    };
    let Ok(mut text) = readout.single_mut() else {
        return;
    };

    // Engine boundary: a bevy transform counts world units, the readout is in
    // meters.
    let position = Meters3::from_engine(transform.translation());
    let centre = current.0.centre(config.sector_edge);
    let offset = position - centre;
    **text = format!(
        "SECTOR {}  {:+.0} {:+.0} {:+.0} m in a {:.0} m cell\nlive {} sectors, {} bodies\n\
         preparing {}/{}, ready {}",
        current.0,
        offset.x().get(),
        offset.y().get(),
        offset.z().get(),
        config.sector_edge.get(),
        roots.iter().count(),
        bodies.iter().count(),
        jobs.iter().count(),
        bevy::tasks::AsyncComputeTaskPool::get().thread_num().max(1),
        ready.0.len(),
    );
}

/// The run gate: cross one boundary and come back, so a harnessed run walks
/// the same lifetime a hand-run flies instead of idling in the first cell.
#[cfg(feature = "debug")]
fn observer_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let origin = SectorCoord::ORIGIN;
    let across = origin.offset(1, 0, 0);
    let edge = uniform_world_config().sector_edge;

    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("wait for the streamed world")
        .enter(GameStates::Loading)
        .until(and(scenario_is_built(), sector_set_is(origin)))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("fly across the +X boundary")
        .on_enter(move |world: &mut World| {
            pose_camera(
                world,
                across.centre(edge),
                across.offset(1, 0, 0).centre(edge),
            );
        })
        .until(sector_set_is(across))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("fly back")
        .on_enter(move |world: &mut World| {
            pose_camera(world, origin.centre(edge), across.centre(edge));
        })
        .until(sector_set_is(origin))
        .deadline(STEP_DEADLINE_SECS)
        .add()
}

/// Advance once the live root set is exactly the desired set around `centre`.
#[cfg(feature = "debug")]
fn sector_set_is(centre: SectorCoord) -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(move |world: &World| {
        let Some(mut query) = world.try_query::<&SectorRoot>() else {
            return false;
        };
        let live: std::collections::BTreeSet<SectorCoord> =
            query.iter(world).map(|root| root.0).collect();
        live == desired_sectors(centre, EXAMPLE_ACTIVE_RADIUS)
    })
}
