//! news_0160_station_archetypes: the v0.16.0 news still of the three
//! generated station archetypes side by side at one scale under one light
//! (`news-0160-station-archetypes.png`).
//!
//! Each station is the open world's station generator run on the loaded
//! parts at its own advancement, as a civilization at that advancement builds
//! it: the ring below the diagonal spokes' advancement, so it stands four
//! spokes; the spire at the solar panels' advancement or above, so its wings
//! are panels; the yard below it, so its arms are struts. The load refuses a
//! station that does not show its feature. `screenshot_station_showcase`
//! keeps its own row of every design at one advancement.
//!
//! The stations stand in a row along X, each in a slot as wide as its own
//! hull, posed Neutral and frozen. The yard is turned so its spine runs
//! across the frame. The camera looks down on the row from +Z so the ring's
//! plane reads, back far enough that the row fits the frame. One three-point
//! rig lights them all.
//!
//! `NOVA_AUTOPILOT=1` alone walks the smoke path and captures nothing;
//! `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1` also writes the PNG, staged under
//! `NOVA_CAPTURE_DIR`:
//! ```text
//! NOVA_CAPTURE_DIR=target/news-0160-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example news_0160_station_archetypes --features debug
//! # look for: `station archetypes: posed 3 station(s)`
//! ```

use std::{collections::BTreeSet, f32::consts::FRAC_PI_4};

use bevy::prelude::*;
use clap::Parser;
use generated_station::generated_station_design;
use nova_protocol::prelude::*;

#[path = "../shared/generated_station.rs"]
mod generated_station;

#[derive(Parser)]
#[command(name = "news_0160_station_archetypes")]
#[command(version = "1.0.0")]
#[command(
    about = "Capture the three generated station archetypes at one scale under one light. Autopilot-only: a posed row behind a scripted camera",
    long_about = None
)]
struct Cli;

/// The still this producer writes.
const SHOT_FILE: &str = "news-0160-station-archetypes.png";

/// The scenario id the row loads under.
const SCENARIO_ID: &str = "news_0160_station_archetypes";

/// The seed every station is built from, and its civilization's world seed:
/// the first seed from 1 whose civilization knows both the truss and the
/// solar panel, so the ring's spokes and the yard's arms are struts and the
/// spire's wings are panels.
const GENERATED_SEED: u32 = 9;

/// Each archetype and the advancement it is built at. The ring stands below
/// the diagonal spokes' 0.5, and the yard below the solar panels' 0.5, both
/// above the truss's 0.2 so their lines are struts; the spire stands above
/// the solar panels' 0.5.
const ARCHETYPES: [(StationArchetypeType, f32); 3] = [
    (StationArchetypeType::Ring, 0.25),
    (StationArchetypeType::Spire, 0.75),
    (StationArchetypeType::Yard, 0.25),
];

/// The section id prefixes the station generator gives its frame parts.
const STRUT_PREFIX: &str = "strut_";
const PANEL_PREFIX: &str = "panel_";
const BLOCK_PREFIX: &str = "block_";

/// The spoke directions a ring below advancement 0.5 stands, as eighths of
/// a turn in its plane: the four axis spokes and no diagonal one.
const RING_SPOKE_EIGHTHS: [i32; 4] = [0, 2, 4, 6];

/// Clear space between two neighbouring hulls' boxes; half of it is the
/// frame's margin round the row.
const SLOT_GAP: Meters = Meters(60.0);

/// How far above the row the camera looks down, in radians.
const CAMERA_PITCH: f32 = 0.6;

/// The camera's vertical field of view: Bevy's perspective default, which
/// the scenario camera keeps.
const VERTICAL_FOV: f32 = FRAC_PI_4;

/// The frame's aspect: the capture's 16:9.
const ASPECT: f32 = 16.0 / 9.0;

/// One build-grid cell, in meters.
const CELL_M: f32 = 10.0;

/// Real seconds the load gets on a software rasterizer.
const LOAD_SECS: f32 = 900.0;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();
    app.add_systems(
        Startup,
        (force_capture_resolution, hide_dev_overlays, hide_hud),
    );
    app.add_systems(Update, freeze_bodies);
    app.add_plugins(archetypes_script());
    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_row);
}

/// The pose each archetype stands in: the yard turned a quarter so its
/// spine (Z) runs across the frame; the ring and the spire level and
/// unturned, the spire's wings already across it.
fn station_rotation(archetype: StationArchetypeType) -> Quat {
    match archetype {
        StationArchetypeType::Yard => Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
        StationArchetypeType::Ring | StationArchetypeType::Spire => Quat::IDENTITY,
    }
}

/// How many of `design`'s sections carry an id starting with `prefix`.
fn count_prefixed(design: &ShipDesign, prefix: &str) -> usize {
    design
        .sections
        .iter()
        .filter(|section| section.id.starts_with(prefix))
        .count()
}

/// The directions, in eighths of a turn, of the frame cells in the ring's
/// plane outside its hub and inside half its outer radius: only spokes stand
/// there. The hub is the one frame part above and below the plane, so its
/// half height is its half width; below advancement 0.5 the ring has at most
/// two rings, and the inner one stands past half the outer radius.
fn ring_spoke_eighths(design: &ShipDesign) -> BTreeSet<i32> {
    let frame: Vec<Vec3> = design
        .sections
        .iter()
        .filter(|section| {
            section.id.starts_with(STRUT_PREFIX) || section.id.starts_with(BLOCK_PREFIX)
        })
        .map(|section| section.position.round())
        .collect();
    let hub = frame.iter().map(|cell| cell.y.abs()).fold(0.0, f32::max);
    let plane: Vec<Vec2> = frame
        .iter()
        .filter(|cell| cell.y == 0.0)
        .map(|cell| cell.xz())
        .collect();
    let outer = plane.iter().map(|cell| cell.length()).fold(0.0, f32::max);
    plane
        .iter()
        .filter(|cell| cell.abs().max_element() > hub && cell.length() < outer * 0.5)
        .map(|cell| (cell.y.atan2(cell.x) / FRAC_PI_4).round() as i32)
        .map(|eighth| eighth.rem_euclid(8))
        .collect()
}

/// Refuse a station that does not show its archetype's feature at its
/// advancement.
fn assert_feature(archetype: StationArchetypeType, advancement: f32, design: &ShipDesign) {
    let struts = count_prefixed(design, STRUT_PREFIX);
    let panels = count_prefixed(design, PANEL_PREFIX);
    info!(
        "station archetypes: {archetype} at advancement {advancement:.2}: {} section(s), \
         {struts} strut(s), {panels} panel(s)",
        design.sections.len()
    );
    match archetype {
        StationArchetypeType::Ring => {
            assert!(
                advancement < 0.5,
                "station archetypes: the ring's spoke check holds below advancement 0.5 only"
            );
            let spokes = ring_spoke_eighths(design);
            info!("station archetypes: ring spokes stand at eighths {spokes:?}");
            assert_eq!(
                spokes,
                BTreeSet::from(RING_SPOKE_EIGHTHS),
                "station archetypes: the ring at advancement {advancement:.2} stands spokes at \
                 eighths {spokes:?}, not the four axis spokes"
            );
            assert!(
                struts > 0 && panels == 0,
                "station archetypes: the ring stands {struts} strut(s) and {panels} panel(s); \
                 seed {GENERATED_SEED} must know the truss"
            );
        }
        StationArchetypeType::Spire => assert!(
            panels > 0,
            "station archetypes: the spire at advancement {advancement:.2} stands no solar \
             panel; seed {GENERATED_SEED} must know the solar panel"
        ),
        StationArchetypeType::Yard => assert!(
            struts > 0 && panels == 0,
            "station archetypes: the yard at advancement {advancement:.2} stands {struts} \
             strut(s) and {panels} panel(s); seed {GENERATED_SEED} must know the truss"
        ),
    }
}

/// The world box a design's mount cells fill once turned by `rotation`, in
/// meters relative to its root, as `(min, max)`, each cell taken whole.
fn turned_box(design: &ShipDesign, rotation: Quat) -> (Vec3, Vec3) {
    // A turned cell's half extent on each axis: the absolute rotation matrix
    // times the cell's half size.
    let turn = Mat3::from_quat(rotation);
    let half = (turn.x_axis.abs() + turn.y_axis.abs() + turn.z_axis.abs()) * 0.5 * CELL_M;
    design.sections.iter().fold(
        (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
        |(min, max), section| {
            let turned = rotation * section.position * CELL_M;
            (min.min(turned - half), max.max(turned + half))
        },
    )
}

/// The row's size, for the framing.
#[derive(Resource, Clone, Copy, Debug)]
struct Row {
    size: Vec3,
    stations: usize,
}

/// Build each archetype at its advancement, refuse one that does not show
/// its feature, and load the row with the rig.
fn load_row(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    loaded: Res<LoadedSectionPacks>,
    items: Res<GameItems>,
) {
    let roster: Vec<(StationArchetypeType, StationDesign, Quat, (Vec3, Vec3))> = ARCHETYPES
        .into_iter()
        .map(|(archetype, advancement)| {
            let (_, design) =
                generated_station_design(&loaded, &items, archetype, advancement, GENERATED_SEED);
            assert_feature(archetype, advancement, &design.hull);
            let rotation = station_rotation(archetype);
            let bounds = turned_box(&design.hull, rotation);
            (archetype, design, rotation, bounds)
        })
        .collect();
    let width = roster
        .iter()
        .map(|(_, _, _, (min, max))| max.x - min.x)
        .sum::<f32>()
        + SLOT_GAP.get() * roster.len() as f32;
    let size = roster
        .iter()
        .fold(Vec3::new(width, 0.0, 0.0), |size, (_, _, _, (min, max))| {
            size.max(Vec3::new(0.0, max.y - min.y, max.z - min.z))
        });
    let mut cursor = (SLOT_GAP.get() - width) / 2.0;
    let mut actions = Vec::new();
    for (archetype, design, rotation, (min, max)) in &roster {
        let root = Vec3::new(
            cursor - min.x,
            -(min.y + max.y) / 2.0,
            -(min.z + max.z) / 2.0,
        );
        cursor += max.x - min.x + SLOT_GAP.get();
        let extent = *max - *min;
        info!(
            "station archetypes: {archetype} stands {:.0} x {:.0} x {:.0} m",
            extent.x, extent.y, extent.z
        );
        actions.push(EventActionConfig::SpawnScenarioObject(
            ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: format!("archetype_{archetype}"),
                    name: format!("Generated {archetype}").to_uppercase(),
                    position: Meters3(root),
                    rotation: *rotation,
                },
                kind: ScenarioObjectKind::Station(StationConfig {
                    design: StationDesignSource::Inline(design.clone()),
                    allegiance: Allegiance::Neutral,
                    derelict: false,
                    initial_velocity: MetersPerSecond3::ZERO,
                }),
            },
        ));
    }
    // One rig: its lights are directional, so a rig per station would stack.
    actions.extend(ThreePointRig::around("photo", Meters3::ZERO, 4.0).actions());
    info!(
        "station archetypes: posed {} station(s) in a {size:.0} m row",
        roster.len()
    );
    commands.insert_resource(Row {
        size,
        stations: roster.len(),
    });
    commands.trigger(LoadScenario(ScenarioConfig {
        description: "The three generated station archetypes at one scale.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions,
        }],
        ..ScenarioConfig::new(
            SCENARIO_ID.to_string(),
            "Station Archetypes".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }));
}

/// Stand the camera on +Z, [`CAMERA_PITCH`] above the row, back far enough
/// that the row's front face fits the frame both ways with its depth
/// beyond.
fn frame_row(world: &mut World) {
    let row = *world.resource::<Row>();
    let half_tan = (VERTICAL_FOV / 2.0).tan();
    let height = row.size.y * CAMERA_PITCH.cos() + row.size.z * CAMERA_PITCH.sin();
    let fit_height = height / 2.0 / half_tan;
    let fit_width = row.size.x / 2.0 / (half_tan * ASPECT);
    let distance = fit_height.max(fit_width) + row.size.z / 2.0;
    let eye = Vec3::new(0.0, CAMERA_PITCH.sin(), CAMERA_PITCH.cos()) * distance;
    pose_camera(world, Meters3(eye), Meters3::ZERO);
}

/// Every station of the row is up.
fn row_is_up() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let Some(row) = world.get_resource::<Row>() else {
            return false;
        };
        world
            .try_query_filtered::<(), With<StationMarker>>()
            .is_some_and(|mut stations| stations.iter(world).count() == row.stations)
    })
}

/// Load the row, frame it and shoot it.
fn archetypes_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("wait for the row")
        .enter(GameStates::Loading)
        .until(and(
            and(state_is(GameStates::Playing), scenario_camera_present()),
            and(scenario_is_built(), row_is_up()),
        ))
        .deadline(LOAD_SECS)
        .add()
        .step("frame the row")
        .on_enter(frame_row)
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("shoot the row")
        .on_enter(|world: &mut World| shoot(world, SHOT_FILE))
        .until(shot_written(SHOT_FILE))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
