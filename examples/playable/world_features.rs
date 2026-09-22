//! world_features: fly a streamed world that has PLACES in it.
//!
//! The second hand-driven half of the streamed-world work, and the one that
//! answers a different question from `world_sectors`. That example shows the
//! streaming lifetime with every cell filled the same way, so a missing sector
//! is obvious. This one fills a cell from a world that exists above it: three
//! independent global noise fields gate feature spheres on a coarse 128 km
//! lattice, and a cell contains whatever reaches it - rocks from the combined
//! asteroid influence, a real planetoid where a planet sphere is centred, a
//! few neutral moored hulls where an anchorage sphere is.
//!
//! What a human is here to judge is the part a headless assert cannot: whether
//! the world reads as PLACES - an empty run of cells, then a rock field, then
//! a world with a mooring beside it - or as noise scattered evenly over a
//! grid. `system_world_sectors` owns the counts and the identities.
//!
//! The generator and the streaming loop are `nova_world`'s; the seed, the
//! cell edge and the content tables are `examples/shared/world_fixture/mod.rs`'s,
//! shared with that range and with `world_sectors`, so what is flown here is
//! what is asserted there.
//!
//! Drawn every frame, so the field is visible and not only its consequences:
//!
//! | colour | what it rings |
//! | - | - |
//! | amber | an asteroid sphere: every cell it reaches gets rocks |
//! | cyan | a planet sphere: its OWNER cell holds a planetoid at the centre |
//! | magenta | an anchorage sphere: its owner cell holds one to three hulls |
//!
//! A sphere is drawn once by the cell that OWNS it and is outlined faintly by
//! every other cell it reaches, which is what makes the shared-sphere claim
//! something you can see rather than read.
//!
//! Hand-run (WASD + right-drag to look; HOLD a key - a 32 km sector is about
//! 530 s at the base 60 m/s and about 17 s at the 32x ramp):
//! ```text
//! cargo run --example world_features --features debug
//! # fly +X and watch the readout's layer strengths rise and fall
//! ```
//!
//! Harnessed mode, the fleet's run gate:
//! - `NOVA_AUTOPILOT=1`: load, arm, cross one boundary, come back, shoot the
//!   three pictures, exit clean. This is the path `probe run` takes.
//! - `NOVA_CAPTURE=1`: writes `world-features-field.png`,
//!   `world-features-planetoid.png` and `world-features-mooring.png`. The
//!   field shot is the diagnostic rings; the other two are the objects only
//!   this generator makes, which is what a reviewer has to look at.

#[path = "../shared/world_fixture/mod.rs"]
pub mod world_fixture;

use bevy::{color::palettes::tailwind, prelude::*};
use clap::Parser;
use nova_protocol::prelude::*;
use nova_world::prelude::*;
#[cfg(feature = "debug")]
use world_fixture::EXAMPLE_ACTIVE_RADIUS;
use world_fixture::{
    featured_world_config, free_play_scenario, world_observer_plugin, FEATURE_HOME,
};

#[derive(Parser)]
#[command(name = "world_features")]
#[command(version = "1.0.0")]
#[command(
    about = "Fly a free-fly observer through a noise-gated streamed world of rock fields, planetoids and moorings",
    long_about = None
)]
struct Cli;

/// The empty bootstrap this session runs inside.
const SCENARIO_ID: &str = "world_features_observer";

/// Marks the readout line.
#[derive(Component)]
struct FeatureReadout;

/// How bright the observer's key light is.
///
/// The example lights itself instead of authoring `Light` objects into the
/// bootstrap, because the bootstrap has to stay EMPTY - that is the property
/// the range asserts, and a scenario with three lights in it is not it.
const KEY_ILLUMINANCE: f32 = 6_000.0;

/// Where the key light points from.
const KEY_DIRECTION: Vec3 = Vec3::new(-0.4, -1.0, -0.6);

/// How many line segments ring a drawn feature sphere.
///
/// A sphere gizmo at 48-96 km is a horizon-scale ring; the default resolution
/// draws it as a polygon you can count the sides of.
const SPHERE_SEGMENTS: u32 = 64;

/// In-step seconds a harnessed beat gets before the run aborts naming it.
///
/// A hang backstop, not a budget. A 125-cell window is 125 preparations, taken
/// a poolful at a time, and then 125 separate materialization frames - and a
/// featured cell's preparation also meshes any planetoid it owns, which is the
/// most expensive single thing in this example.
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 300.0;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new()
        .with_game_plugins((observer_plugin, world_observer_plugin, NovaWorldPlugin))
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
    app.add_systems(
        Update,
        (
            park_at_home,
            draw_feature_spheres,
            update_readout,
            report_census,
        ),
    );
}

/// Load the empty bootstrap, arm the featured stream, light the scene, and put
/// the readout up.
fn boot_observer(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.trigger(LoadScenario(free_play_scenario(
        &game_assets,
        SCENARIO_ID,
        "World Features Observer",
    )));
    commands.insert_resource(featured_world_config());

    commands.spawn((
        Name::new("Observer Key Light"),
        DirectionalLight {
            illuminance: KEY_ILLUMINANCE,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_translation(Vec3::ZERO).looking_to(KEY_DIRECTION, Vec3::Y),
    ));

    // Top LEFT and several lines, not one centered line: the dev overlay's fps
    // and version bar sits along the top right, and a full-width readout ran
    // straight through it.
    commands
        .spawn((
            Name::new("Feature Readout"),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                left: Val::Px(12.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                FeatureReadout,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

/// Put the free-fly observer down in [`FEATURE_HOME`], once, and leave it
/// flyable.
///
/// The `WASDCamera` is re-inserted WITH the pose rather than the transform
/// being written on its own. The rig keeps its own position and writes the
/// transform from it in `PostUpdate` every frame, so a bare transform write is
/// erased before it can be seen - the observer stayed at the origin and the
/// window never reached this cell. Re-inserting the rig re-reads the transform
/// it is given and adopts it as the new rest position, which is what keeps
/// free flight, unlike the harness's `pose_camera`, which takes the rig off.
///
/// A human opens this example already standing in the one window near the
/// origin that holds all three layers, and flies out of it under their own
/// power.
fn park_at_home(
    mut parked: Local<bool>,
    config: Option<Res<WorldConfig>>,
    observer: Query<(Entity, &WASDCamera), With<ScenarioCameraMarker>>,
    mut commands: Commands,
) {
    if *parked {
        return;
    }
    let Some(config) = config else {
        return;
    };
    // The rig lands a frame after the camera does, so this waits for it
    // rather than taking the one frame the camera is `Added`.
    let Ok((entity, rig)) = observer.single() else {
        return;
    };

    // Engine boundary: a cell centre is in meters, a transform in world units.
    let position = FEATURE_HOME.centre(config.sector_edge).to_engine();
    let look_at = FEATURE_HOME
        .offset(1, 0, 0)
        .centre(config.sector_edge)
        .to_engine();
    commands.entity(entity).insert((
        Transform::from_translation(position).looking_at(look_at, Vec3::Y),
        *rig,
    ));
    *parked = true;
    info!("world features: the observer opens in {FEATURE_HOME}");
}

/// The colour a layer's spheres are rung in.
fn layer_colour(layer: FeatureLayer) -> Srgba {
    match layer {
        FeatureLayer::Asteroid => tailwind::AMBER_400,
        FeatureLayer::Planet => tailwind::CYAN_400,
        FeatureLayer::Anchorage => tailwind::FUCHSIA_400,
    }
}

/// Ring every feature sphere the live window can see.
///
/// The OWNER's ring is drawn at full strength and every other cell that the
/// sphere reaches outlines it faintly, so one sphere seen from six cells
/// reads as one sphere with six faint echoes rather than six spheres. That is
/// the cross-boundary identity claim, made visible.
fn draw_feature_spheres(mut gizmos: Gizmos, roots: Query<(&SectorRoot, &SectorFeatureSpheres)>) {
    for (root, spheres) in &roots {
        for sphere in &spheres.0 {
            let owned = sphere.owner == root.0;
            let colour = layer_colour(sphere.layer).with_alpha(if owned { 0.9 } else { 0.12 });
            gizmos
                .sphere(
                    // Engine boundary: a sphere is authored in meters and drawn
                    // in world units.
                    Isometry3d::from_translation(sphere.centre.to_engine()),
                    sphere.radius.to_engine(),
                    colour,
                )
                .resolution(SPHERE_SEGMENTS);
        }
    }
}

/// Name the cell the observer is in, what the three layers read there, and
/// what the window is holding.
fn update_readout(
    config: Option<Res<WorldConfig>>,
    current: Option<Res<CurrentSector>>,
    ready: Res<ReadySectors>,
    observer: Query<&GlobalTransform, With<WorldObserver>>,
    roots: Query<(&SectorRoot, &SectorStrengths)>,
    jobs: Query<&SectorJob>,
    rocks: Query<&AsteroidMarker>,
    planets: Query<&PlanetMarker>,
    hulls: Query<&SpaceshipRootMarker>,
    mut readout: Query<&mut Text, With<FeatureReadout>>,
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
    let here = roots
        .iter()
        .find(|(root, _)| root.0 == current.0)
        .map(|(_, strengths)| strengths.0);
    let layers = FeatureLayer::ALL
        .map(|layer| {
            let value = here.map_or(0.0, |strengths| strengths[layer.index()]);
            format!("{layer} {value:.2}")
        })
        .join("  ");

    **text = format!(
        "SECTOR {}  {:+.0} {:+.0} {:+.0} m in a {:.0} m cell\n{layers}\n\
         live {} sectors: {} rocks, {} planetoids, {} hulls\npreparing {}/{}, ready {}",
        current.0,
        offset.x().get(),
        offset.y().get(),
        offset.z().get(),
        config.sector_edge.get(),
        roots.iter().count(),
        rocks.iter().count(),
        planets.iter().count(),
        hulls.iter().count(),
        jobs.iter().count(),
        bevy::tasks::AsyncComputeTaskPool::get().thread_num().max(1),
        ready.0.len(),
    );
}

/// Log one census line per crossing: a SNAPSHOT of the live roots at the
/// moment the observer entered a new window, not the finished window.
///
/// `materialize_ready_sector` spawns ONE sector a frame, so a window is still
/// filling when this fires: the live set mixes the cells the new window has
/// reached with the ones the old window has not retired yet. That is why the
/// line carries its own `live of total` count - the count is what makes the
/// snapshot readable as evidence instead of a claim about a whole window.
///
/// A log rather than a readout line because it is the thing a run is READ
/// afterwards for - a headless run of this example leaves behind the census of
/// every window it stood in, which is what the fixed thresholds were tuned
/// against.
fn report_census(
    current: Option<Res<CurrentSector>>,
    roots: Query<(&SectorRoot, &SectorStrengths, &SectorFeatureSpheres)>,
    config: Option<Res<WorldConfig>>,
) {
    let (Some(current), Some(config)) = (current, config) else {
        return;
    };
    if !current.is_changed() {
        return;
    }
    let live = roots.iter().count();
    if live == 0 {
        return;
    }

    let mut owned = [0usize; FeatureLayer::COUNT];
    let mut reached = [0usize; FeatureLayer::COUNT];
    let mut empty = 0usize;
    let mut single = 0usize;
    let mut blended = 0usize;
    for (root, strengths, spheres) in &roots {
        for sphere in &spheres.0 {
            reached[sphere.layer.index()] += 1;
            if sphere.owner == root.0 {
                owned[sphere.layer.index()] += 1;
            }
        }
        match strengths.0.iter().filter(|value| **value > 0.0).count() {
            0 => empty += 1,
            1 => single += 1,
            _ => blended += 1,
        }
    }

    let census = FeatureLayer::ALL
        .map(|layer| {
            format!(
                "{layer} {} owned / {} reaching",
                owned[layer.index()],
                reached[layer.index()]
            )
        })
        .join(", ");
    info!(
        "world features: window at {} ({} of {} cells live): {census}; \
         {empty} empty, {single} single-layer, {blended} blended",
        current.0,
        live,
        desired_cells(config.active_radius),
    );
}

/// How many cells a window of this radius wants.
fn desired_cells(radius: i32) -> usize {
    let side = (2 * radius + 1) as usize;
    side * side * side
}

/// The run gate: cross one boundary and come back, so a harnessed run walks
/// the same lifetime a hand-run flies instead of idling in the first cell.
#[cfg(feature = "debug")]
fn observer_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let home = FEATURE_HOME;
    let across = home.offset(1, 0, 0);
    let edge = featured_world_config().sector_edge;

    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("wait for the streamed world")
        .enter(GameStates::Loading)
        .until(and(scenario_is_built(), sector_set_is(home)))
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
            pose_camera(world, home.centre(edge), across.centre(edge));
        })
        .until(sector_set_is(home))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // The three pictures. The targets are read from the HOME window and
        // kept, because flying to one of them moves the window off the other:
        // the planetoid and the mooring are four cells apart and the window is
        // five cells wide. Last in the script on purpose - `pose_camera` takes
        // the WASD rig off the camera, so nothing flies after this.
        .step("pick the shot targets")
        .on_enter(pick_shot_targets)
        .add()
        .step("frame the feature field")
        .on_enter(move |world: &mut World| {
            hide_dev_overlays(world);
            pose_camera(world, home.centre(edge), across.centre(edge));
        })
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("shoot the feature field")
        .on_enter(|world: &mut World| shoot(world, FIELD_SHOT))
        .until(shot_written(FIELD_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("frame the planetoid")
        .on_enter(|world: &mut World| {
            let targets = *world.resource::<ShotTargets>();
            let distance = Meters(targets.planetoid_radius.get() * PLANETOID_STANDOFF);
            pose_camera(
                world,
                standoff(targets.planetoid, distance),
                targets.planetoid,
            );
        })
        .until(and(window_is_settled(), frames(SETTLE_FRAMES)))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("shoot the planetoid")
        .on_enter(|world: &mut World| {
            assert!(
                planetoid_in_view(world).is_some(),
                "world features: the planetoid must be live in the window the shot frames"
            );
            shoot(world, PLANETOID_SHOT);
        })
        .until(shot_written(PLANETOID_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("frame the mooring")
        .on_enter(|world: &mut World| {
            let targets = *world.resource::<ShotTargets>();
            pose_camera(
                world,
                standoff(targets.mooring, HULL_STANDOFF),
                targets.mooring,
            );
        })
        .until(and(window_is_settled(), frames(SETTLE_FRAMES)))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("shoot the mooring")
        .on_enter(|world: &mut World| {
            assert!(
                mooring_in_view(world).is_some(),
                "world features: a moored hull must be live in the window the shot frames"
            );
            shoot(world, HULL_SHOT);
        })
        .until(shot_written(HULL_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}

/// The picture of the field from the middle of the home window.
#[cfg(feature = "debug")]
const FIELD_SHOT: &str = "world-features-field.png";

/// The picture of the one body a planet sphere made.
#[cfg(feature = "debug")]
const PLANETOID_SHOT: &str = "world-features-planetoid.png";

/// The picture of the hulls an anchorage sphere moored.
#[cfg(feature = "debug")]
const HULL_SHOT: &str = "world-features-mooring.png";

/// How many body radii back the planetoid shot stands.
///
/// A planetoid is 600 to 1200 m, so a fixed distance would fill the frame with
/// one and lose another. Scaling by the body keeps both readable.
#[cfg(feature = "debug")]
const PLANETOID_STANDOFF: f32 = 3.2;

/// How far back the mooring shot stands, in meters.
///
/// A block hauler is about 100 m, so this frames ONE hull. A mooring is one to
/// three hulls spread over kilometers, and a framing that held all of them
/// made every hull a speck - what the picture is for is whether the hull is a
/// real ship, and the readout in the corner carries the count.
#[cfg(feature = "debug")]
const HULL_STANDOFF: Meters = Meters(700.0);

/// Where the two objects the featured generator makes stood while the home
/// window was up.
///
/// Kept rather than looked up at shot time: flying to the planetoid retires
/// the cell the mooring is in, so the second target has to be a remembered
/// PLACE, not a live entity.
#[cfg(feature = "debug")]
#[derive(Resource, Clone, Copy)]
struct ShotTargets {
    /// Where the planetoid is.
    planetoid: Meters3,
    /// How big it is, which is what sizes its shot.
    planetoid_radius: Meters,
    /// Which moored hull to shoot.
    mooring: Meters3,
}

/// Read both shot targets out of the live home window.
///
/// Panics when the window holds neither: the shots would otherwise be pictures
/// of empty space that read as a successful run.
#[cfg(feature = "debug")]
fn pick_shot_targets(world: &mut World) {
    let (planetoid, planetoid_radius) = planetoid_in_view(world)
        .expect("world features: the home window must hold a planetoid to shoot");
    let mooring = mooring_in_view(world)
        .expect("world features: the home window must hold a moored hull to shoot");
    info!(
        "world features: shooting the planetoid in {} and the mooring in {}",
        SectorCoord::containing(planetoid, featured_world_config().sector_edge),
        SectorCoord::containing(mooring, featured_world_config().sector_edge),
    );
    world.insert_resource(ShotTargets {
        planetoid,
        planetoid_radius,
        mooring,
    });
}

/// An eye `distance` from `target`, up and off to one side.
///
/// Off-axis on purpose: a shot taken down an axis of a generated cell hides
/// whether anything has depth.
#[cfg(feature = "debug")]
fn standoff(target: Meters3, distance: Meters) -> Meters3 {
    let eye = target.get() + Vec3::new(0.62, 0.42, 0.66).normalize() * distance.get();
    Meters3::new(eye.x, eye.y, eye.z)
}

/// Where the first live planetoid is, and how big it is.
#[cfg(feature = "debug")]
fn planetoid_in_view(world: &mut World) -> Option<(Meters3, Meters)> {
    let mut query = world.query_filtered::<(&GlobalTransform, &PlanetRadius), With<PlanetMarker>>();
    query.iter(world).next().map(|(transform, radius)| {
        (
            Meters3::from_engine(transform.translation()),
            Meters::from_engine(radius.0),
        )
    })
}

/// The live moored hull nearest the middle of its cluster.
///
/// The middle rather than whichever hull the query hands back first: query
/// order is not the spawn order, and two runs of the same seed have to shoot
/// the same hull to be comparable.
#[cfg(feature = "debug")]
fn mooring_in_view(world: &mut World) -> Option<Meters3> {
    let mut query = world.query_filtered::<&GlobalTransform, With<SpaceshipRootMarker>>();
    let hulls: Vec<Meters3> = query
        .iter(world)
        .map(|transform| Meters3::from_engine(transform.translation()))
        .collect();
    if hulls.is_empty() {
        return None;
    }
    let sum = hulls
        .iter()
        .fold(Vec3::ZERO, |total, hull| total + hull.get());
    let middle = sum / hulls.len() as f32;
    let middle = Meters3::new(middle.x, middle.y, middle.z);
    hulls.into_iter().min_by(|a, b| {
        a.distance(middle)
            .get()
            .total_cmp(&b.distance(middle).get())
    })
}

/// Advance once the window around wherever the observer now stands has
/// finished streaming.
///
/// A shot beat cannot name its cell when the script is written, because the
/// cell comes from an object the run found. This asks the question the other
/// way round: is the live set the set the CURRENT observer cell wants.
#[cfg(feature = "debug")]
fn window_is_settled() -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(|world: &World| {
        let Some(current) = world.get_resource::<CurrentSector>() else {
            return false;
        };
        let Some(mut query) = world.try_query::<&SectorRoot>() else {
            return false;
        };
        let live: std::collections::BTreeSet<SectorCoord> =
            query.iter(world).map(|root| root.0).collect();
        live == desired_sectors(current.0, EXAMPLE_ACTIVE_RADIUS)
    })
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
