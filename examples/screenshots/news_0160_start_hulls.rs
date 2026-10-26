//! news_0160_start_hulls: the v0.16.0 news still of every start role's hull,
//! at one scale under one light (`news-0160-start-hulls.png`).
//!
//! The roster is RESOLVED from the loaded start roles, in the order the New
//! Game role list shows them, never listed here. Each hull is the catalog
//! design its role flies, posted Neutral and inert, bow to the left and
//! rolled toward the camera so the dorsal intake door on every start hull
//! reads. The hulls fill a grid of [`COLUMNS`] in a plane square to the
//! camera, each centred in a cell as large as the largest hull, so no hull
//! stands nearer the lens than another and one meter is one length across
//! the frame. One three-point rig lights them all.
//!
//! `NOVA_AUTOPILOT=1` alone walks the smoke path and captures nothing;
//! `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1` also writes the PNG, staged under
//! `NOVA_CAPTURE_DIR`:
//! ```text
//! NOVA_CAPTURE_DIR=target/news-0160-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example news_0160_start_hulls --features debug
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "news_0160_start_hulls")]
#[command(version = "1.0.0")]
#[command(
    about = "Capture every start role's hull at one scale under one light. Autopilot-only: a posed grid behind a scripted camera",
    long_about = None
)]
struct Cli;

/// The still this producer writes.
const SHOT_FILE: &str = "news-0160-start-hulls.png";

/// The scenario id the grid loads under.
const SCENARIO_ID: &str = "news_0160_start_hulls";

/// Hulls per grid row; the base roster of six fills two rows.
const COLUMNS: usize = 3;

/// The roll toward the camera that shows each hull's top deck, where every
/// start hull carries its intake door.
const ROLL_TOWARD_CAMERA: f32 = 0.55;

/// Clear space between two neighbouring grid cells; half of it is the
/// frame's margin round the grid.
const CELL_GAP: Meters = Meters(20.0);

/// The camera's vertical field of view: Bevy's perspective default, which
/// the scenario camera keeps.
const VERTICAL_FOV: f32 = std::f32::consts::FRAC_PI_4;

/// The frame's aspect: the capture's 16:9.
const ASPECT: f32 = 16.0 / 9.0;

/// One build-grid cell, in meters.
const CELL_M: f32 = 10.0;

/// Real seconds the load gets on a software rasterizer.
const LOAD_SECS: f32 = 600.0;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();
    app.add_systems(
        Startup,
        (force_capture_resolution, hide_dev_overlays, hide_hud),
    );
    app.add_systems(Update, freeze_bodies);
    app.add_plugins(hulls_script());
    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_grid);
}

/// The pose every hull wears: the bow (-Z) turned to the frame's left (-X),
/// then rolled about it so the top deck tilts toward the camera on +Z.
fn hull_rotation() -> Quat {
    Quat::from_rotation_x(ROLL_TOWARD_CAMERA) * Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)
}

/// The box a design's mount cells fill, in meters relative to its root, as
/// `(min, max)`, each cell taken whole.
fn hull_box(design: &ShipDesign) -> (Vec3, Vec3) {
    let (min, max) = design.sections.iter().fold(
        (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
        |(min, max), section| {
            (
                min.min(section.position - Vec3::splat(0.5)),
                max.max(section.position + Vec3::splat(0.5)),
            )
        },
    );
    (min * CELL_M, max * CELL_M)
}

/// The world extent of `(min, max)` turned by `rotation`, and its centre
/// turned the same way.
fn turned_box(rotation: Quat, (min, max): (Vec3, Vec3)) -> (Vec3, Vec3) {
    let mut low = Vec3::splat(f32::MAX);
    let mut high = Vec3::splat(f32::MIN);
    for corner in 0..8 {
        let at = Vec3::new(
            if corner & 1 == 0 { min.x } else { max.x },
            if corner & 2 == 0 { min.y } else { max.y },
            if corner & 4 == 0 { min.z } else { max.z },
        );
        let turned = rotation * at;
        low = low.min(turned);
        high = high.max(turned);
    }
    (high - low, rotation * ((min + max) / 2.0))
}

/// The grid's size, for the framing.
#[derive(Resource, Clone, Copy, Debug)]
struct Grid {
    size: Vec2,
    hulls: usize,
}

/// Lay every start role's hull into the grid and load it with the rig.
///
/// # Panics
///
/// On a role whose hull the catalog lacks: lint refuses that first.
fn load_grid(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    roles: Res<GameStartRoles>,
    ships: Res<GameShipDesigns>,
) {
    let rotation = hull_rotation();
    let roster: Vec<(String, ShipDesign, Vec3, Vec3)> = roles
        .0
        .iter()
        .map(|role| {
            let design = ships
                .get_design(&role.hull)
                .unwrap_or_else(|| {
                    panic!(
                        "start hulls: role '{}' flies no catalog hull '{}'",
                        role.id, role.hull
                    )
                })
                .design
                .clone();
            let (extent, centre) = turned_box(rotation, hull_box(&design));
            (role.id.to_string(), design, extent, centre)
        })
        .collect();
    let cell = roster.iter().fold(Vec2::ZERO, |cell, (_, _, extent, _)| {
        cell.max(extent.truncate())
    }) + Vec2::splat(CELL_GAP.get());
    let rows = roster.len().div_ceil(COLUMNS);
    let size = Vec2::new(
        cell.x * COLUMNS.min(roster.len()) as f32,
        cell.y * rows as f32,
    );
    let hulls = roster
        .iter()
        .enumerate()
        .map(|(index, (id, design, extent, centre))| {
            let (column, row) = (index % COLUMNS, index / COLUMNS);
            let slot = Vec3::new(
                (column as f32 + 0.5) * cell.x - size.x / 2.0,
                size.y / 2.0 - (row as f32 + 0.5) * cell.y,
                0.0,
            );
            info!(
                "start hulls: '{id}' in cell ({column}, {row}), {:.0} x {:.0} m in frame",
                extent.x, extent.y
            );
            EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: format!("start_hull_{id}"),
                    name: id.clone(),
                    position: Meters3(slot - *centre),
                    rotation,
                },
                kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
                    controller: SpaceshipController::None,
                    allegiance: Some(Allegiance::Neutral),
                    design: ShipDesignSource::Inline(design.clone()),
                    ..default()
                }),
            })
        });
    // One rig: its lights are directional, so a rig per hull would stack.
    let rigs = ThreePointRig::around("photo", Meters3::ZERO, 1.0).actions();
    let actions: Vec<EventActionConfig> = hulls.chain(rigs).collect();
    info!(
        "start hulls: posed {} hull(s) in a {size:.0} m grid",
        roster.len()
    );
    commands.insert_resource(Grid {
        size,
        hulls: roster.len(),
    });
    commands.trigger(LoadScenario(ScenarioConfig {
        description: "Every start role's hull at one scale under one light.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions,
        }],
        ..ScenarioConfig::new(
            SCENARIO_ID.to_string(),
            "Start Hulls".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }));
}

/// Stand the camera square to the grid on +Z, back far enough that the
/// grid fits the frame both ways.
fn frame_grid(world: &mut World) {
    let grid = *world.resource::<Grid>();
    let half_tan = (VERTICAL_FOV / 2.0).tan();
    let fit_height = grid.size.y / 2.0 / half_tan;
    let fit_width = grid.size.x / 2.0 / (half_tan * ASPECT);
    let distance = fit_height.max(fit_width);
    pose_camera(world, Meters3::new(0.0, 0.0, distance), Meters3::ZERO);
}

/// Every hull of the grid is up as a ship.
fn grid_is_up() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let Some(grid) = world.get_resource::<Grid>() else {
            return false;
        };
        world
            .try_query_filtered::<(), With<SpaceshipRootMarker>>()
            .is_some_and(|mut ships| ships.iter(world).count() == grid.hulls)
    })
}

/// Load the grid, frame it and shoot it.
fn hulls_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("wait for the grid")
        .enter(GameStates::Loading)
        .until(and(
            and(state_is(GameStates::Playing), scenario_camera_present()),
            and(scenario_is_built(), grid_is_up()),
        ))
        .deadline(LOAD_SECS)
        .add()
        .step("frame the grid")
        .on_enter(frame_grid)
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("shoot the grid")
        .on_enter(|world: &mut World| shoot(world, SHOT_FILE))
        .until(shot_written(SHOT_FILE))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
