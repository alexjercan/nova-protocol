//! news_0160_generated_scale: the v0.16.0 news still of three generated
//! armored hulls from three distances, at one scale under one light
//! (`news-0160-generated-scale.png`).
//!
//! Each hull is the open world's ship generator run for a real civilization
//! of the figure's world seed: one from the first ring, one near 2,000 km and
//! one past 3,000 km, each at its civilization's own advancement. A band
//! takes the first node along its axes whose civilization is living, fields
//! armored hulls and builds one, then the first hull seed whose intake opens
//! on the top deck, where the pose shows it; the far band also needs a bow
//! lance. A band with no such hull stops the load, naming every node and why.
//!
//! The hulls stand in a row along X, bow (-Z) to the frame's left, rolled
//! toward the camera on +Z so the top deck reads, each in a slot as wide as
//! its own hull. The camera stands square to the row, back far enough that
//! it fits the frame, so one meter is one length across it. One three-point
//! rig lights them all.
//!
//! `NOVA_AUTOPILOT=1` alone walks the smoke path and captures nothing;
//! `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1` also writes the PNG, staged under
//! `NOVA_CAPTURE_DIR`:
//! ```text
//! NOVA_CAPTURE_DIR=target/news-0160-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example news_0160_generated_scale --features debug
//! # look for: `generated scale: posed 3 hull(s)`
//! ```

use std::collections::HashMap;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "news_0160_generated_scale")]
#[command(version = "1.0.0")]
#[command(
    about = "Capture three generated armored hulls from three distances at one scale under one light. Autopilot-only: a posed row behind a scripted camera",
    long_about = None
)]
struct Cli;

/// The still this producer writes.
const SHOT_FILE: &str = "news-0160-generated-scale.png";

/// The scenario id the row loads under.
const SCENARIO_ID: &str = "news_0160_generated_scale";

/// The world seed the other v0.16.0 figures play.
const WORLD_SEED: u32 = 10;

/// One distance band of the row.
struct Band {
    /// What the band is called in a log line or a refusal.
    label: &'static str,
    /// The lattice ring its nodes stand on: the civilization lattice is
    /// 240 km, and each multiple of it from the origin is one ring.
    ring: i32,
    /// The centroid distances it accepts, low included.
    distance: (Meters, Meters),
    /// Whether its hull must carry a bow lance.
    lance: bool,
}

/// The three bands, near to far.
const BANDS: [Band; 3] = [
    Band {
        label: "the first ring",
        ring: 1,
        distance: (Meters(0.0), Meters(400_000.0)),
        lance: false,
    },
    Band {
        label: "near 2,000 km",
        ring: 8,
        distance: (Meters(1_850_000.0), Meters(2_150_000.0)),
        lance: false,
    },
    Band {
        label: "past 3,000 km",
        ring: 13,
        distance: (Meters(3_000_000.0), Meters(3_400_000.0)),
        lance: true,
    },
];

/// The lattice directions a band's nodes are tried along, in order.
const NODE_DIRECTIONS: [[i32; 3]; 6] = [
    [1, 0, 0],
    [-1, 0, 0],
    [0, 0, 1],
    [0, 0, -1],
    [0, 1, 0],
    [0, -1, 0],
];

/// The first hull seed a civilization tries.
const FIRST_HULL_SEED: u32 = 1;

/// How many hull seeds a civilization tries.
const HULL_SEED_SEARCH: u32 = 64;

/// The roll toward the camera that shows each hull's top deck.
const ROLL_TOWARD_CAMERA: f32 = 0.55;

/// Clear space between two neighbouring hulls' boxes; half of it is the
/// frame's margin round the row.
const SLOT_GAP: Meters = Meters(40.0);

/// The camera's vertical field of view: Bevy's perspective default, which
/// the scenario camera keeps.
const VERTICAL_FOV: f32 = std::f32::consts::FRAC_PI_4;

/// The frame's aspect: the capture's 16:9.
const ASPECT: f32 = 16.0 / 9.0;

/// One build-grid cell, in meters.
const CELL_M: f32 = 10.0;

/// Real seconds the load gets on a software rasterizer.
const LOAD_SECS: f32 = 900.0;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let packs = nova_authoring::lint_walk::repo_ship_part_packs(&["base"]);
    let snapshot = ShipPartSnapshot::build(&packs).unwrap_or_else(|faults| {
        panic!("generated scale: the base snapshot is refused: {faults:?}")
    });
    let field = CivilizationField::new(WORLD_SEED);
    let hulls: Vec<(String, String, ShipDesign)> = BANDS
        .iter()
        .map(|band| band_hull(&snapshot, &field, band))
        .collect();
    let rotation = hull_rotation();
    let stands: Vec<Stand> = hulls
        .into_iter()
        .map(|(name, pick, design)| Stand {
            bounds: turned_box(&design, rotation),
            name,
            pick,
            design,
        })
        .collect();
    for pair in stands.windows(2) {
        let [near, far] = pair else { unreachable!() };
        assert!(
            near.length() < far.length(),
            "generated scale: '{}' is {:.0} m long, not shorter than '{}' at {:.0} m",
            near.name,
            near.length(),
            far.name,
            far.length()
        );
    }

    let mut app = AppBuilder::new()
        .with_game_plugins(move |app: &mut App| {
            app.insert_resource(Stands(stands.clone()));
            app.add_systems(OnEnter(GameAssetsStates::Loaded), load_row);
        })
        .build();
    app.add_systems(
        Startup,
        (force_capture_resolution, hide_dev_overlays, hide_hud),
    );
    app.add_systems(Update, freeze_bodies);
    app.add_plugins(scale_script());
    app.run()
}

/// The hull `band` shows: the first node along [`NODE_DIRECTIONS`] whose
/// civilization is in the band, living, fields armored hulls and builds one
/// whose every intake opens up (and, for a lance band, with a bow lance),
/// and the first such hull seed. Named after the band, with a line naming
/// the pick for the load to log: the logger is not up yet.
///
/// # Panics
///
/// When no node of the band gives one, naming each node and why.
fn band_hull(
    snapshot: &ShipPartSnapshot,
    field: &CivilizationField,
    band: &Band,
) -> (String, String, ShipDesign) {
    let parts: HashMap<&str, &ShipPart> = snapshot
        .parts()
        .iter()
        .map(|part| (part.id(), part))
        .collect();
    let armored = ShipRoleType::ALL
        .iter()
        .position(|role| *role == ShipRoleType::Armored)
        .expect("ShipRoleType::ALL carries Armored");
    let mut refusals = Vec::new();
    for direction in NODE_DIRECTIONS {
        let node = direction.map(|axis| axis * band.ring);
        let civilization = field.civilization(node);
        let distance = civilization.centroid.length();
        let refuse =
            |why: String| format!("node {node:?} at {:.0} km: {why}", distance.0 / 1_000.0);
        if !(band.distance.0 .0..band.distance.1 .0).contains(&distance.0) {
            refusals.push(refuse("outside the band".to_string()));
            continue;
        }
        if civilization.status != CivilizationStatusType::Living {
            refusals.push(refuse("extinct".to_string()));
            continue;
        }
        if civilization.role_preference[armored] <= 0.0
            || !snapshot
                .eligible_roles(civilization.id, civilization.advancement)
                .contains(&ShipRoleType::Armored)
        {
            refusals.push(refuse("fields no armored hull".to_string()));
            continue;
        }
        let found = (FIRST_HULL_SEED..FIRST_HULL_SEED + HULL_SEED_SEARCH).find_map(|seed| {
            let request = ShipLayoutRequest {
                seed,
                civilization: civilization.id,
                role: ShipRoleType::Armored,
                advancement: civilization.advancement,
            };
            let layout = generate_ship(snapshot, request).ok()?;
            let fittings = Fittings::of(&layout.design, &parts);
            (fittings.intakes > 0
                && fittings.intakes == fittings.dorsal_intakes
                && (!band.lance || fittings.bow_lances > 0))
                .then_some((seed, layout, fittings))
        });
        let Some((seed, layout, fittings)) = found else {
            refusals.push(refuse(format!(
                "no hull seed in {FIRST_HULL_SEED}..{} has a dorsal intake{}",
                FIRST_HULL_SEED + HULL_SEED_SEARCH,
                if band.lance { " and a bow lance" } else { "" }
            )));
            continue;
        };
        let pick = format!(
            "generated scale: {}: node {node:?} at {:.0} km, advancement {:.3}, hull seed {seed}: \
             {} section(s), clearance {:.0} m, {} dorsal intake(s), {} bow lance(s)",
            band.label,
            distance.0 / 1_000.0,
            civilization.advancement,
            layout.design.sections.len(),
            layout.clearance.0,
            fittings.dorsal_intakes,
            fittings.bow_lances,
        );
        return (band.label.to_string(), pick, layout.design);
    }
    panic!(
        "generated scale: world seed {WORLD_SEED} gives {} no hull:\n  {}",
        band.label,
        refusals.join("\n  ")
    );
}

/// The fittings of a generated hull the pose must show.
struct Fittings {
    intakes: usize,
    /// Intakes whose door opens up (+Y).
    dorsal_intakes: usize,
    /// Railguns whose bore fires forward (-Z): a lance only fires along it.
    bow_lances: usize,
}

impl Fittings {
    fn of(design: &ShipDesign, parts: &HashMap<&str, &ShipPart>) -> Self {
        let mut fittings = Self {
            intakes: 0,
            dorsal_intakes: 0,
            bow_lances: 0,
        };
        for section in &design.sections {
            let part = parts.get(section.source.prototype_id()).unwrap_or_else(|| {
                panic!(
                    "generated scale: no part '{}'",
                    section.source.prototype_id()
                )
            });
            let Some(exit) = exit_normal(&part.config.kind).map(|normal| section.rotation * normal)
            else {
                continue;
            };
            match part.config.kind {
                SectionKind::CargoIntake(_) => {
                    fittings.intakes += 1;
                    fittings.dorsal_intakes += usize::from(exit.y > 0.5);
                }
                SectionKind::Railgun(_) => fittings.bow_lances += usize::from(exit.z < -0.5),
                _ => {}
            }
        }
        fittings
    }
}

/// The pose every hull wears: the bow (-Z) turned to the frame's left (-X),
/// then rolled about it so the top deck tilts toward the camera on +Z.
fn hull_rotation() -> Quat {
    Quat::from_rotation_x(ROLL_TOWARD_CAMERA) * Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)
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

/// One hull of the row and the box it fills in its pose.
#[derive(Clone)]
struct Stand {
    name: String,
    /// The line naming the civilization, request and fittings it was picked by.
    pick: String,
    design: ShipDesign,
    bounds: (Vec3, Vec3),
}

impl Stand {
    /// Its length across the frame: bow to stern in the pose.
    fn length(&self) -> f32 {
        self.bounds.1.x - self.bounds.0.x
    }
}

/// The row, near band first, computed once at boot.
#[derive(Resource, Clone)]
struct Stands(Vec<Stand>);

/// The row's size, for the framing.
#[derive(Resource, Clone, Copy, Debug)]
struct Row {
    size: Vec3,
    hulls: usize,
}

/// Lay the hulls along X and load the row with the rig.
fn load_row(mut commands: Commands, game_assets: Res<GameAssets>, stands: Res<Stands>) {
    let stands = &stands.0;
    let width =
        stands.iter().map(Stand::length).sum::<f32>() + SLOT_GAP.get() * stands.len() as f32;
    let size = stands
        .iter()
        .fold(Vec3::new(width, 0.0, 0.0), |size, stand| {
            let (min, max) = stand.bounds;
            size.max(Vec3::new(0.0, max.y - min.y, max.z - min.z))
        });
    let rotation = hull_rotation();
    let mut cursor = (SLOT_GAP.get() - width) / 2.0;
    let mut actions = Vec::new();
    for (index, stand) in stands.iter().enumerate() {
        let (min, max) = stand.bounds;
        let root = Vec3::new(
            cursor - min.x,
            -(min.y + max.y) / 2.0,
            -(min.z + max.z) / 2.0,
        );
        cursor += stand.length() + SLOT_GAP.get();
        info!("{}", stand.pick);
        info!(
            "generated scale: '{}' stands {:.0} x {:.0} m in frame",
            stand.name,
            max.x - min.x,
            max.y - min.y
        );
        actions.push(EventActionConfig::SpawnScenarioObject(
            ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: format!("generated_scale_{index}"),
                    name: stand.name.clone(),
                    position: Meters3(root),
                    rotation,
                },
                kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
                    controller: SpaceshipController::None,
                    allegiance: Some(Allegiance::Neutral),
                    design: ShipDesignSource::Inline(stand.design.clone()),
                    ..default()
                }),
            },
        ));
    }
    // One rig: its lights are directional, so a rig per hull would stack.
    actions.extend(ThreePointRig::around("photo", Meters3::ZERO, 1.0).actions());
    info!(
        "generated scale: posed {} hull(s) in a {size:.0} m row",
        stands.len()
    );
    commands.insert_resource(Row {
        size,
        hulls: stands.len(),
    });
    commands.trigger(LoadScenario(ScenarioConfig {
        description: "Three generated armored hulls from three distances at one scale.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions,
        }],
        ..ScenarioConfig::new(
            SCENARIO_ID.to_string(),
            "Generated Scale".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }));
}

/// Stand the camera square to the row on +Z, back far enough that the row
/// fits the frame both ways in front of its depth.
fn frame_row(world: &mut World) {
    let row = *world.resource::<Row>();
    let half_tan = (VERTICAL_FOV / 2.0).tan();
    let fit_height = row.size.y / 2.0 / half_tan;
    let fit_width = row.size.x / 2.0 / (half_tan * ASPECT);
    let distance = fit_height.max(fit_width) + row.size.z / 2.0;
    pose_camera(world, Meters3::new(0.0, 0.0, distance), Meters3::ZERO);
}

/// Every hull of the row is up as a ship.
fn row_is_up() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let Some(row) = world.get_resource::<Row>() else {
            return false;
        };
        world
            .try_query_filtered::<(), With<SpaceshipRootMarker>>()
            .is_some_and(|mut ships| ships.iter(world).count() == row.hulls)
    })
}

/// Load the row, frame it and shoot it.
fn scale_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
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
