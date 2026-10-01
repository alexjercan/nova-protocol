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
//!   pictures, exit clean. This is the path `probe run` takes.
//! - `NOVA_CAPTURE=1`: writes `world-features-cluster.png`,
//!   `world-features-planetoid.png` and one
//!   `world-features-ship-<role>-<condition>.png` for each generated ship role
//!   and each of intact and derelict. The cluster shot is the rings around the
//!   nearest cluster; the others are the objects only this generator streams,
//!   which is what a reviewer has to look at. A ship shot frames the manifest
//!   ship of that role and condition nearest the home centre within
//!   `SHIP_SEARCH_RADIUS` cells, streams the window around it and checks the
//!   live ship by its `EntityId`: its `Name`, role style and
//!   `DerelictShipMarker`. A caption in the frame prints what was read off
//!   the live ship beside its manifest entry, and the run logs each pick. A
//!   role and condition with no ship in reach is logged as missing and shot
//!   as nothing.

#[path = "../shared/world_fixture/mod.rs"]
pub mod world_fixture;

use bevy::{color::palettes::tailwind, prelude::*};
use clap::Parser;
use nova_protocol::prelude::*;
use nova_world::prelude::*;
use world_fixture::{
    featured_world_config, free_play_scenario, world_observer_plugin, FEATURE_HOME,
};
#[cfg(feature = "debug")]
use world_fixture::{EXAMPLE_ACTIVE_RADIUS, EXAMPLE_SECTOR_EDGE};

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
fn boot_observer(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    loaded: Res<LoadedSectionPacks>,
    styles: Res<GameStyles>,
) {
    commands.trigger(LoadScenario(free_play_scenario(
        &game_assets,
        SCENARIO_ID,
        "World Features Observer",
    )));
    commands.insert_resource(featured_world_config(&loaded, &styles));

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
        let clusters = sector_clusters(&config.generator, config.input(root.0))
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
    let edge = EXAMPLE_SECTOR_EDGE;

    let mut script = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
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
        // The pictures. The targets are read from the HOME window and kept,
        // because flying to one of them can move the window off the others.
        // Last in the script on purpose - `pose_camera` takes the WASD rig
        // off the camera, so nothing flies after this.
        .step("pick the shot targets")
        .on_enter(pick_shot_targets)
        .add()
        .step("search the ship shots")
        .on_enter(search_ship_shots)
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
        .add();

    for (r, role) in ShipRoleType::ALL.into_iter().enumerate() {
        for (c, condition) in SHIP_CONDITIONS.into_iter().enumerate() {
            let name = ship_shot_name(role, condition);
            let missing = resource_where::<ShipShots>(move |shots| shots.0[r][c].is_none());
            script = script
                .step(format!(
                    "frame the {} {} ship",
                    role.label(),
                    condition.label()
                ))
                .on_enter(move |world: &mut World| {
                    let Some(shot) = world.resource::<ShipShots>().0[r][c].clone() else {
                        return;
                    };
                    let distance = Meters(shot.clearance.get() * SHIP_STANDOFF_CLEARANCES);
                    pose_camera(world, standoff(shot.position, distance), shot.position);
                })
                .until(or(
                    missing.clone(),
                    and(window_is_settled(), frames(SETTLE_FRAMES)),
                ))
                .deadline(STEP_DEADLINE_SECS)
                .add()
                .step(format!(
                    "caption the {} {} ship",
                    role.label(),
                    condition.label()
                ))
                .on_enter(move |world: &mut World| {
                    let Some(shot) = world.resource::<ShipShots>().0[r][c].clone() else {
                        return;
                    };
                    caption_live_ship(world, &shot);
                })
                .until(or(missing.clone(), frames(CAPTION_FRAMES)))
                .deadline(STEP_DEADLINE_SECS)
                .add()
                .step(format!(
                    "shoot the {} {} ship",
                    role.label(),
                    condition.label()
                ))
                .on_enter({
                    let name = name.clone();
                    let present =
                        move |world: &World| world.resource::<ShipShots>().0[r][c].is_some();
                    move |world: &mut World| {
                        if present(world) {
                            shoot(world, &name);
                        }
                    }
                })
                .until(or(missing, shot_written(name)))
                .deadline(SHOT_DEADLINE_SECS)
                .add();
        }
    }
    script
}

/// The picture of the cluster anchored nearest the home cell.
#[cfg(feature = "debug")]
const CLUSTER_SHOT: &str = "world-features-cluster.png";

/// The picture of the planetoid nearest the home cell.
#[cfg(feature = "debug")]
const PLANETOID_SHOT: &str = "world-features-planetoid.png";

/// How many body radii back the planetoid shot stands.
///
/// A planetoid is 600 to 1200 m, so a fixed distance would fill the frame with
/// one and lose another. Scaling by the body keeps both readable.
#[cfg(feature = "debug")]
const PLANETOID_STANDOFF: f32 = 3.2;

/// How many hull clearance radii back a ship shot stands.
///
/// A generated hull's clearance runs up to 400 m, so a fixed distance would
/// lose a small hull in the frame. Scaling by the hull keeps each one close.
#[cfg(feature = "debug")]
const SHIP_STANDOFF_CLEARANCES: f32 = 2.5;

/// The conditions a ship shot is taken in, in [`ShipShots`] column order.
#[cfg(feature = "debug")]
const SHIP_CONDITIONS: [SectorShipConditionType; 2] = [
    SectorShipConditionType::Intact,
    SectorShipConditionType::Derelict,
];

/// How far outside a cluster's extent the cluster shot stands.
///
/// Close to the ring, so the near half of the cluster is inside the camera's
/// 10 km far plane and the far half fades out behind it.
#[cfg(feature = "debug")]
const CLUSTER_STANDOFF: Meters = Meters(1_500.0);

/// Where the cluster and planetoid shot targets stood while the home window
/// was up.
///
/// Kept rather than looked up at shot time: flying to the cluster can retire
/// the cell the planetoid is in, so the second target has to be a remembered
/// PLACE, not a live entity.
#[cfg(feature = "debug")]
#[derive(Resource, Clone, Copy)]
struct ShotTargets {
    /// Where the planetoid is.
    planetoid: Meters3,
    /// How big it is, which is what sizes its shot.
    planetoid_radius: Meters,
    /// The anchor of the cluster the cluster shot frames.
    cluster: Meters3,
    /// How far that cluster reaches, which is where its ring is.
    cluster_extent: Meters,
}

/// Read the cluster and planetoid shot targets out of the live home window:
/// the cluster anchored nearest the home centre, and the planetoid nearest
/// it.
///
/// Nearest rather than first: query order is not the spawn order, and two
/// runs of the same seed have to shoot the same bodies to be comparable.
/// Panics when the window lacks any of them: the shots would otherwise be
/// pictures of empty space that read as a successful run.
#[cfg(feature = "debug")]
fn pick_shot_targets(world: &mut World) {
    let edge = EXAMPLE_SECTOR_EDGE;
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
    info!(
        "world features: shooting {} ({}, extent {:.0} m, anchor {:?}) and the planetoid in {}",
        cluster.id,
        cluster.cluster_type.label(),
        cluster.extent.get(),
        cluster.anchor.get(),
        SectorCoord::containing(planetoid, edge),
    );
    world.insert_resource(ShotTargets {
        planetoid,
        planetoid_radius,
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

/// How many cells out from home, on each axis, the ship shots search the
/// generator's manifests.
///
/// Wider than the live window: at the fixture seed the home window holds no
/// intact ship. The search is pure, so it streams nothing; only the cell a
/// shot frames is streamed, by the live window around the camera. Bounded, so
/// a role the world does not field nearby is reported missing rather than
/// hunted for.
#[cfg(feature = "debug")]
const SHIP_SEARCH_RADIUS: i32 = 3;

/// How many frames a ship caption stands before its shot, so the text is laid
/// out and drawn in the picture.
#[cfg(feature = "debug")]
const CAPTION_FRAMES: u32 = 3;

/// One generated ship a ship shot frames, as its cell's manifest plans it.
#[cfg(feature = "debug")]
#[derive(Clone)]
struct ShipShot {
    /// Its stable streamed id, which finds it again at shot time.
    id: String,
    /// The cell whose manifest plans it.
    cell: SectorCoord,
    /// The civilization it belongs to.
    civilization: CivilizationId,
    /// Its role.
    role: ShipRoleType,
    /// Whether it is intact or a derelict.
    condition: SectorShipConditionType,
    /// Where its root stands.
    position: Meters3,
    /// Its manifest clearance radius, which sizes its shot.
    clearance: Meters,
}

#[cfg(feature = "debug")]
impl ShipShot {
    /// The `Name` the stream gives it.
    fn name(&self) -> String {
        match self.condition {
            SectorShipConditionType::Intact => {
                format!("{} {}", self.civilization.name(), self.role.label())
            }
            SectorShipConditionType::Derelict => format!(
                "{} derelict, former {}",
                self.civilization.name(),
                self.role.label()
            ),
        }
    }
}

/// The ship each ship shot frames: one row per [`ShipRoleType::ALL`] role and
/// one column per [`SHIP_CONDITIONS`] condition. `None` when no manifest in
/// reach plans a ship of that role and condition.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct ShipShots([[Option<ShipShot>; 2]; 4]);

/// What a ship caption reads off a live ship root.
#[cfg(feature = "debug")]
type ShipRow = (
    &'static EntityId,
    &'static Name,
    &'static ShipStyle,
    Has<DerelictShipMarker>,
    &'static GlobalTransform,
);

/// The text a ship shot prints over its picture.
#[cfg(feature = "debug")]
#[derive(Component)]
struct ShipShotCaption;

/// The picture of the `role` ship in `condition`.
#[cfg(feature = "debug")]
fn ship_shot_name(role: ShipRoleType, condition: SectorShipConditionType) -> String {
    format!(
        "world-features-ship-{}-{}.png",
        role.label(),
        condition.label()
    )
}

/// Pick one ship of each role and condition out of the manifests of every
/// cell within [`SHIP_SEARCH_RADIUS`] of home: the one nearest the home
/// centre, ties broken by id. Logs every pick and every missing one.
///
/// # Panics
///
/// When a cell fails to generate, or a planned ship does not wear its role's
/// style.
#[cfg(feature = "debug")]
fn search_ship_shots(world: &mut World) {
    let config = world.resource::<WorldConfig<NovaLayeredWorld>>();
    let home = FEATURE_HOME.centre(config.sector_edge);
    let reach = -SHIP_SEARCH_RADIUS..=SHIP_SEARCH_RADIUS;
    let mut shots: [[Option<ShipShot>; 2]; 4] = Default::default();
    let mut planned = 0;
    for x in reach.clone() {
        for y in reach.clone() {
            for z in reach.clone() {
                let cell = FEATURE_HOME.offset(x, y, z);
                let manifest = config
                    .generator
                    .generate(config.input(cell))
                    .unwrap_or_else(|fault| panic!("world features: {cell}: {fault}"));
                for ship in manifest.ships {
                    planned += 1;
                    assert_eq!(
                        ship.design.presentation.style.as_deref(),
                        Some(role_style_id(ship.role)),
                        "world features: ship {} is planned in another style",
                        ship.id
                    );
                    let r = ShipRoleType::ALL
                        .iter()
                        .position(|role| *role == ship.role)
                        .expect("every role is in ShipRoleType::ALL");
                    let c = usize::from(ship.condition == SectorShipConditionType::Derelict);
                    let nearer = shots[r][c].as_ref().is_none_or(|best| {
                        ship.position
                            .distance(home)
                            .get()
                            .total_cmp(&best.position.distance(home).get())
                            .then_with(|| ship.id.cmp(&best.id))
                            .is_lt()
                    });
                    if nearer {
                        shots[r][c] = Some(ShipShot {
                            id: ship.id,
                            cell,
                            civilization: ship.civilization,
                            role: ship.role,
                            condition: ship.condition,
                            position: ship.position,
                            clearance: ship.clearance,
                        });
                    }
                }
            }
        }
    }
    info!(
        "world features: {planned} ships planned within {SHIP_SEARCH_RADIUS} cells of \
         {FEATURE_HOME}"
    );
    for (r, role) in ShipRoleType::ALL.into_iter().enumerate() {
        for (c, condition) in SHIP_CONDITIONS.into_iter().enumerate() {
            match &shots[r][c] {
                Some(shot) => info!(
                    "world features: picked {} {} ship {} in {}: {} ({}), {:.0} m from home, \
                     clearance {:.0} m",
                    role.label(),
                    condition.label(),
                    shot.id,
                    shot.cell,
                    shot.name(),
                    shot.civilization,
                    shot.position.distance(home).get(),
                    shot.clearance.get(),
                ),
                None => info!(
                    "world features: missing {} {} ship: no manifest within \
                     {SHIP_SEARCH_RADIUS} cells of {FEATURE_HOME} plans one",
                    role.label(),
                    condition.label()
                ),
            }
        }
    }
    world.insert_resource(ShipShots(shots));
}

/// Find `shot`'s ship live by its `EntityId`, check that it is the ship its
/// manifest planned, and print what it carries over the picture.
///
/// # Panics
///
/// When the ship is not live, or its `Name`, style or derelict marker is not
/// what its manifest entry gives it.
#[cfg(feature = "debug")]
fn caption_live_ship(world: &mut World, shot: &ShipShot) {
    let mut ships = world.query_filtered::<ShipRow, With<SpaceshipRootMarker>>();
    let (name, style, derelict, at) = ships
        .iter(world)
        .find(|(id, ..)| id.0 == shot.id)
        .map(|(_, name, style, derelict, transform)| {
            (
                name.to_string(),
                style.0.clone(),
                derelict,
                Meters3::from_engine(transform.translation()),
            )
        })
        .unwrap_or_else(|| {
            panic!(
                "world features: ship {} must be live in the window its shot frames",
                shot.id
            )
        });
    assert_eq!(name, shot.name(), "world features: ship {} name", shot.id);
    assert_eq!(
        style.as_deref(),
        Some(role_style_id(shot.role)),
        "world features: ship {} style",
        shot.id
    );
    assert_eq!(
        derelict,
        shot.condition == SectorShipConditionType::Derelict,
        "world features: ship {} derelict marker",
        shot.id
    );
    let caption = format!(
        "live EntityId {}\nName \"{name}\"\nShipStyle {}  DerelictShipMarker {}\n\
         manifest {} {} {} {} clearance {:.0} m\nlive {:.0} m from planned",
        shot.id,
        style.as_deref().unwrap_or("none"),
        if derelict { "yes" } else { "no" },
        shot.cell,
        shot.civilization,
        shot.role.label(),
        shot.condition.label(),
        shot.clearance.get(),
        at.distance(shot.position).get(),
    );
    info!(
        "world features: caption {}: {}",
        shot.id,
        caption.replace('\n', " | ")
    );
    let mut captions = world.query_filtered::<&mut Text, With<ShipShotCaption>>();
    if let Some(mut text) = captions.iter_mut(world).next() {
        text.0 = caption;
        return;
    }
    world
        .spawn((
            Name::new("Ship Shot Caption"),
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(12.0),
                left: Val::Px(12.0),
                padding: UiRect::all(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
        ))
        .with_children(|parent| {
            parent.spawn((
                ShipShotCaption,
                Text::new(caption),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
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
