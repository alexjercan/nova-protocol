//! world_features: fly a streamed world that has PLACES in it.
//!
//! The second hand-driven half of the streamed-world work, and the one that
//! answers a different question from `world_sectors`. That example shows the
//! streaming lifetime with every cell filled the same way, so a missing sector
//! is obvious. This one flies the base game's own world: clusters decided on a
//! global 50 km lattice from three environment fields - asteroid-rich,
//! rock-only, planet-heavy and derelict-field places, each within 8 km of its
//! anchor - and, in a few cells that own no cluster body, a small background
//! scatter of rocks. A wreck is streamed only beside a rock or world of its
//! own cluster in its own cell.
//!
//! What a human is here to judge is the part a headless assert cannot: whether
//! the world reads as PLACES - an empty run of cells, then a rock field, then
//! two to four worlds with rocks around them, then a wreck field - or as noise
//! scattered evenly over a grid. `system_world_sectors` owns the counts and the
//! identities.
//!
//! The streaming loop is `nova_world`'s and the generator is the base game's
//! `NovaLayeredWorld`. A streamed manifest carries bodies only, so the rings
//! and the readout ask `nova_world_base::sector_clusters` for each live root,
//! with the same input the generator was given. The seed and the cell edge are
//! `examples/shared/world_fixture/mod.rs`'s, shared with that range and with
//! `world_sectors`, so what is flown here is what is asserted there.
//!
//! Drawn every frame, so the policy is visible and not only its consequences:
//!
//! | colour | what it rings |
//! | - | - |
//! | amber | an asteroid-rich cluster |
//! | orange | a rock-only cluster |
//! | cyan | a planet-heavy cluster |
//! | magenta | a derelict field |
//!
//! A ring is drawn at the cluster's anchor with its true extent. The cell the
//! anchor stands in draws it bright and every other cell that owns one of its
//! bodies outlines it faintly, which is what makes the one-cluster-across-a-
//! face claim something you can see rather than read.
//!
//! Hand-run (WASD + right-drag to look; HOLD a key - a 32 km sector is about
//! 530 s at the base 60 m/s and about 17 s at the 32x ramp):
//! ```text
//! cargo run --example world_features --features debug
//! # fly +X and watch the readout's fields and clusters change
//! ```
//!
//! Harnessed mode, the fleet's run gate:
//! - `NOVA_AUTOPILOT=1`: load, arm, cross one boundary, come back, shoot the
//!   three pictures, exit clean. This is the path `probe run` takes.
//! - `NOVA_CAPTURE=1`: writes `world-features-cluster.png`,
//!   `world-features-planetoid.png` and `world-features-derelict.png`. The
//!   cluster shot is the rings around the nearest cluster; the other two are
//!   the objects only this generator streams, which is what a reviewer has to
//!   look at.

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
    about = "Fly a free-fly observer through the base game's clustered world of rock fields, planetoids and derelicts",
    long_about = None
)]
struct Cli;

/// The empty bootstrap this session runs inside.
const SCENARIO_ID: &str = "world_features_observer";

/// Marks the readout line.
#[derive(Component)]
struct FeatureReadout;

/// What the cluster policy planned for one live sector, on its root.
///
/// Example-owned: a streamed manifest carries bodies only, so this view asks
/// the base generator for the same plan it streamed, once per root as it
/// comes up.
#[derive(Component)]
struct RootClusters(SectorClusters);

/// How bright the observer's key light is.
///
/// The example lights itself instead of authoring `Light` objects into the
/// bootstrap, because the bootstrap has to stay EMPTY - that is the property
/// the range asserts, and a scenario with three lights in it is not it.
const KEY_ILLUMINANCE: f32 = 6_000.0;

/// Where the key light points from.
const KEY_DIRECTION: Vec3 = Vec3::new(-0.4, -1.0, -0.6);

/// How many line segments ring a drawn cluster.
///
/// A cluster reaches up to 8 km, which fills a frame from inside it; the
/// default resolution draws that as a polygon you can count the sides of.
const RING_SEGMENTS: u32 = 64;

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
        .with_game_plugins((
            observer_plugin,
            world_observer_plugin,
            NovaWorldPlugin::<NovaLayeredWorld>::default(),
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
    app.add_systems(
        Update,
        (
            park_at_home,
            describe_root_clusters.after(NovaWorldSystems::Retire),
            (draw_clusters, update_readout, report_census).after(describe_root_clusters),
        ),
    );
}

/// Load the empty bootstrap, arm the featured stream, light the scene, and put
/// the readout up.
///
/// `OnEnter` rather than `Update`: nova_world requires a `WorldConfig` writer
/// to land ahead of `NovaWorldSystems::Cleanup`, and a state transition runs
/// before `Update` at all.
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
/// A human opens this example already standing in a window that holds every
/// cluster type, and flies out of it under their own power.
fn park_at_home(
    mut parked: Local<bool>,
    config: Option<Res<WorldConfig<NovaLayeredWorld>>>,
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

/// The colour a cluster type is rung in.
fn cluster_colour(cluster_type: ClusterType) -> Srgba {
    match cluster_type {
        ClusterType::AsteroidRich => tailwind::AMBER_400,
        ClusterType::RockOnly => tailwind::ORANGE_600,
        ClusterType::PlanetHeavy => tailwind::CYAN_400,
        ClusterType::DerelictField => tailwind::FUCHSIA_400,
    }
}

/// Ask the policy what each root that came up this frame holds.
///
/// After `Retire`, so a root is described in the frame it is spawned and
/// never after it is taken back.
fn describe_root_clusters(
    mut commands: Commands,
    config: Option<Res<WorldConfig<NovaLayeredWorld>>>,
    roots: Query<(Entity, &SectorRoot), Added<SectorRoot>>,
) {
    let Some(config) = config else {
        return;
    };
    for (entity, root) in &roots {
        let clusters = sector_clusters(config.input(root.0))
            .unwrap_or_else(|fault| panic!("world features: {}: {fault}", root.0));
        // A scenario swap can despawn the root on this frame.
        commands.entity(entity).try_insert(RootClusters(clusters));
    }
}

/// Ring every cluster the live window owns a body of.
///
/// The cell the anchor stands in draws the ring bright and every other cell
/// that owns one of its bodies outlines it faintly, so one cluster seen from
/// two cells reads as one ring with one faint echo rather than two clusters.
/// That is the cross-face identity claim, made visible.
fn draw_clusters(mut gizmos: Gizmos, roots: Query<(&SectorRoot, &RootClusters)>) {
    for (root, clusters) in &roots {
        for cluster in &clusters.0.clusters {
            let home = cluster.home == root.0;
            let colour =
                cluster_colour(cluster.cluster_type).with_alpha(if home { 0.9 } else { 0.15 });
            gizmos
                .sphere(
                    // Engine boundary: a cluster is planned in meters and
                    // drawn in world units.
                    Isometry3d::from_translation(cluster.anchor.to_engine()),
                    cluster.extent.to_engine(),
                    colour,
                )
                .resolution(RING_SEGMENTS);
        }
    }
}

/// Name the cell the observer is in, what the three fields read there, what
/// it owns, and what the window is holding.
fn update_readout(
    config: Option<Res<WorldConfig<NovaLayeredWorld>>>,
    current: Option<Res<CurrentSector>>,
    ready: Res<ReadySectors>,
    observer: Query<&GlobalTransform, With<WorldObserver>>,
    roots: Query<(&SectorRoot, &RootClusters)>,
    jobs: Query<&SectorJob>,
    rocks: Query<&AsteroidMarker>,
    planets: Query<&PlanetMarker>,
    ships: Query<&SpaceshipRootMarker>,
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
        .map(|(_, clusters)| &clusters.0);
    let (fields, owned) = here.map_or_else(
        || ("fields pending".to_string(), String::new()),
        |plan| {
            let fields = EnvironmentFieldType::ALL
                .map(|field| format!("{} {:.2}", field.label(), plan.environment.get(field)))
                .join("  ");
            let clusters = plan
                .clusters
                .iter()
                .map(|cluster| format!("{} {}", cluster.cluster_type.label(), cluster.id))
                .collect::<Vec<_>>()
                .join(", ");
            let owned = format!(
                "owns {}{}; placed {} ({} escorts), skipped {} face / {} clearance / {} companion",
                if clusters.is_empty() {
                    "no cluster body".to_string()
                } else {
                    clusters
                },
                if plan.background_rocks > 0 {
                    format!(", a {}-rock scatter", plan.background_rocks)
                } else {
                    String::new()
                },
                plan.placed,
                plan.escorts,
                plan.skipped_face,
                plan.skipped_clearance,
                plan.skipped_companion,
            );
            (fields, owned)
        },
    );

    **text = format!(
        "SECTOR {}  {:+.0} {:+.0} {:+.0} m in a {:.0} m cell\n{fields}\n{owned}\n\
         live {} sectors: {} rocks, {} planetoids, {} ships\npreparing {}/{}, ready {}",
        current.0,
        offset.x().get(),
        offset.y().get(),
        offset.z().get(),
        config.sector_edge.get(),
        roots.iter().count(),
        rocks.iter().count(),
        planets.iter().count(),
        ships.iter().count(),
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
/// every window it stood in.
fn report_census(
    current: Option<Res<CurrentSector>>,
    roots: Query<(&SectorRoot, &RootClusters)>,
    config: Option<Res<WorldConfig<NovaLayeredWorld>>>,
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

    let mut clusters = std::collections::BTreeMap::new();
    let (mut empty, mut owning, mut scattered) = (0usize, 0usize, 0usize);
    let (mut face, mut clearance, mut companion, mut escorts) = (0usize, 0usize, 0usize, 0usize);
    for (_, plan) in &roots {
        let plan = &plan.0;
        for cluster in &plan.clusters {
            clusters.insert(cluster.id.clone(), cluster.cluster_type);
        }
        if !plan.clusters.is_empty() {
            owning += 1;
        } else if plan.background_rocks > 0 {
            scattered += 1;
        } else if plan.placed == 0 {
            empty += 1;
        }
        face += plan.skipped_face;
        clearance += plan.skipped_clearance;
        companion += plan.skipped_companion;
        escorts += plan.escorts;
    }

    let census = ClusterType::ALL
        .map(|kind| {
            let count = clusters.values().filter(|each| **each == kind).count();
            format!("{count} {}", kind.label())
        })
        .join(", ");
    info!(
        "world features: window at {} ({} of {} cells live): clusters {census}; \
         {owning} cells own cluster bodies, {scattered} hold a scatter, {empty} empty; \
         placed {escorts} escorts; skipped {face} at a face, {clearance} for clearance, \
         {companion} hulls for want of a companion",
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
        // kept, because flying to one of them can move the window off the
        // other. Last in the script on purpose - `pose_camera` takes the WASD
        // rig off the camera, so nothing flies after this.
        .step("pick the shot targets")
        .on_enter(pick_shot_targets)
        .add()
        .step("frame the nearest cluster")
        .on_enter(|world: &mut World| {
            hide_dev_overlays(world);
            let targets = *world.resource::<ShotTargets>();
            let distance = targets.cluster_extent + CLUSTER_STANDOFF;
            pose_camera(world, standoff(targets.cluster, distance), targets.cluster);
        })
        .until(and(window_is_settled(), frames(SETTLE_FRAMES)))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("shoot the nearest cluster")
        .on_enter(|world: &mut World| shoot(world, CLUSTER_SHOT))
        .until(shot_written(CLUSTER_SHOT))
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
            let targets = *world.resource::<ShotTargets>();
            assert!(
                planetoid_nearest(world, targets.planetoid)
                    .is_some_and(|(at, _)| at.distance(targets.planetoid).get() < 1.0),
                "world features: the planetoid must be live in the window the shot frames"
            );
            shoot(world, PLANETOID_SHOT);
        })
        .until(shot_written(PLANETOID_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("frame the derelict")
        .on_enter(|world: &mut World| {
            let targets = *world.resource::<ShotTargets>();
            pose_camera(
                world,
                standoff(targets.derelict, DERELICT_STANDOFF),
                targets.derelict,
            );
        })
        .until(and(window_is_settled(), frames(SETTLE_FRAMES)))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("shoot the derelict")
        .on_enter(|world: &mut World| {
            let targets = *world.resource::<ShotTargets>();
            assert!(
                derelict_nearest(world, targets.derelict)
                    .is_some_and(|at| at.distance(targets.derelict).get() < 1.0),
                "world features: the derelict must be live in the window the shot frames"
            );
            shoot(world, DERELICT_SHOT);
        })
        .until(shot_written(DERELICT_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}

/// The picture of the cluster anchored nearest the home cell.
#[cfg(feature = "debug")]
const CLUSTER_SHOT: &str = "world-features-cluster.png";

/// The picture of the planetoid nearest the home cell.
#[cfg(feature = "debug")]
const PLANETOID_SHOT: &str = "world-features-planetoid.png";

/// The picture of the derelict nearest the home cell.
#[cfg(feature = "debug")]
const DERELICT_SHOT: &str = "world-features-derelict.png";

/// How many body radii back the planetoid shot stands.
///
/// A planetoid is 600 to 1200 m, so a fixed distance would fill the frame with
/// one and lose another. Scaling by the body keeps both readable.
#[cfg(feature = "debug")]
const PLANETOID_STANDOFF: f32 = 3.2;

/// How far back the derelict shot stands, in meters.
///
/// Close enough to frame ONE derelict: what the picture is for is whether the
/// hull is a real ship, and the readout in the corner carries the count.
#[cfg(feature = "debug")]
const DERELICT_STANDOFF: Meters = Meters(700.0);

/// How far outside a cluster's extent the cluster shot stands.
///
/// Close to the ring, so the near half of the cluster is inside the camera's
/// 10 km far plane and the far half fades out behind it.
#[cfg(feature = "debug")]
const CLUSTER_STANDOFF: Meters = Meters(1_500.0);

/// Where the three shot targets stood while the home window was up.
///
/// Kept rather than looked up at shot time: flying to the planetoid can
/// retire the cell the derelict is in, so the second target has to be a
/// remembered PLACE, not a live entity.
#[cfg(feature = "debug")]
#[derive(Resource, Clone, Copy)]
struct ShotTargets {
    /// Where the planetoid is.
    planetoid: Meters3,
    /// How big it is, which is what sizes its shot.
    planetoid_radius: Meters,
    /// Which derelict to shoot.
    derelict: Meters3,
    /// The anchor of the cluster the cluster shot frames.
    cluster: Meters3,
    /// How far that cluster reaches, which is where its ring is.
    cluster_extent: Meters,
}

/// Read the three shot targets out of the live home window: the cluster
/// anchored nearest the home centre, and the planetoid and the derelict
/// nearest it.
///
/// Nearest rather than first: query order is not the spawn order, and two
/// runs of the same seed have to shoot the same bodies to be comparable.
/// Panics when the window lacks any of them: the shots would otherwise be
/// pictures of empty space that read as a successful run.
#[cfg(feature = "debug")]
fn pick_shot_targets(world: &mut World) {
    let edge = featured_world_config().sector_edge;
    let home = FEATURE_HOME.centre(edge);
    let mut roots = world.query::<&RootClusters>();
    // Every cell that owns a body of a cluster hands back the same anchor and
    // id, so the pick does not depend on query order.
    let cluster = roots
        .iter(world)
        .flat_map(|plan| &plan.0.clusters)
        .min_by(|a, b| {
            let near = |cluster: &ClusterSummary| cluster.anchor.distance(home).get();
            near(a).total_cmp(&near(b)).then_with(|| a.id.cmp(&b.id))
        })
        .cloned()
        .expect("world features: the home window must own a cluster body to shoot");
    let (planetoid, planetoid_radius) = planetoid_nearest(world, home)
        .expect("world features: the home window must hold a planetoid to shoot");
    let derelict = derelict_nearest(world, home)
        .expect("world features: the home window must hold a derelict to shoot");
    info!(
        "world features: shooting {} ({}, extent {:.0} m, anchor {:?}), the planetoid in {} \
         and the derelict in {}",
        cluster.id,
        cluster.cluster_type.label(),
        cluster.extent.get(),
        cluster.anchor.get(),
        SectorCoord::containing(planetoid, edge),
        SectorCoord::containing(derelict, edge),
    );
    world.insert_resource(ShotTargets {
        planetoid,
        planetoid_radius,
        derelict,
        cluster: cluster.anchor,
        cluster_extent: cluster.extent,
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

/// The live planetoid nearest `point`, and how big it is.
#[cfg(feature = "debug")]
fn planetoid_nearest(world: &mut World, point: Meters3) -> Option<(Meters3, Meters)> {
    let mut query = world.query_filtered::<(&GlobalTransform, &PlanetRadius), With<PlanetMarker>>();
    query
        .iter(world)
        .map(|(transform, radius)| {
            (
                Meters3::from_engine(transform.translation()),
                Meters::from_engine(radius.0),
            )
        })
        .min_by(|a, b| {
            a.0.distance(point)
                .get()
                .total_cmp(&b.0.distance(point).get())
        })
}

/// The live derelict nearest `point`.
#[cfg(feature = "debug")]
fn derelict_nearest(world: &mut World, point: Meters3) -> Option<Meters3> {
    let mut query = world.query_filtered::<&GlobalTransform, With<SpaceshipRootMarker>>();
    query
        .iter(world)
        .map(|transform| Meters3::from_engine(transform.translation()))
        .min_by(|a, b| a.distance(point).get().total_cmp(&b.distance(point).get()))
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
