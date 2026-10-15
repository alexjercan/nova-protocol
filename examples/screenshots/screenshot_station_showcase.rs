//! screenshot_station_showcase: every catalog station design and one
//! generated station per archetype, side by side.
//!
//! Reads the merged station catalog after load, generates a ring, a spire and
//! a yard at advancement 0.5 from the loaded parts, and parks one living
//! Neutral copy of each in a row along X, each in a slot as wide as its own
//! hull, under the three-point rig. The catalog roster is RESOLVED, never
//! listed, so a mod's station joins the row with no code change.
//!
//! Every shot is framed by the hull it shows: the camera stands back by a
//! multiple of the design's longest extent, so a 30 m block and a 300 m yard
//! fill the frame alike.
//!
//! Two harnessed modes, the fleet's capture idiom:
//! - `NOVA_AUTOPILOT=1`: smoke path - load the row, walk every framing, exit
//!   clean.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also shoot `station-showcase.png`
//!   (the whole row) and one `station-showcase-<design>.png` per design
//!   (staged under `NOVA_CAPTURE_DIR`).
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=1800 \
//!   cargo run --example screenshot_station_showcase --features debug
//! # look for: `station showcase: parked <n> design(s)`,
//! #           `station showcase: walk complete`,
//! #           `autopilot: cycle complete, no panic`
//! ```

use bevy::prelude::*;
use clap::Parser;
use generated_station::generated_station_design;
// Direct, not through `nova_protocol::nova_debug`: that path only exists under
// the `debug` feature, and `capturing()` gates the body freeze in EVERY build.
#[cfg(feature = "debug")]
use nova_debug::prelude::capturing;
use nova_protocol::prelude::*;

#[path = "../shared/generated_station.rs"]
mod generated_station;

#[derive(Parser)]
#[command(name = "screenshot_station_showcase")]
#[command(version = "1.0.0")]
#[command(about = "Every station design parked side by side, each shot framed by its own hull. Autopilot-only: a posed row behind a scripted camera", long_about = None)]
struct Cli;

/// The scenario id the row loads under.
const SCENARIO_ID: &str = "station_showcase";

/// The gap between two neighbouring hulls' bounding boxes.
const SLOT_GAP: Meters = Meters(120.0);

/// The advancement the generated stations are built at.
const GENERATED_ADVANCEMENT: f32 = 0.5;

/// The seed the generated stations are built from.
const GENERATED_SEED: u32 = 3;

/// One build-grid cell, in meters.
const CELL_M: f32 = 10.0;

/// How far the camera stands back from a design, as a multiple of its
/// longest extent, plus a margin so a small hull is not shot from inside.
const FRAME_DISTANCE_FACTOR: f32 = 1.35;
const FRAME_DISTANCE_MARGIN: Meters = Meters(40.0);

/// The direction the camera looks from: high, front and to the right of the
/// row, so every hull reads two faces and its top at once.
const FRAME_DIRECTION: Vec3 = Vec3::new(0.7, 0.55, 1.0);

/// Real seconds the load and a shot may take on a software rasterizer, and
/// the frames a framing settles for before its shot.
#[cfg(feature = "debug")]
const LOAD_SECS: f32 = 900.0;
#[cfg(feature = "debug")]
const SHOT_SECS: f32 = 600.0;
#[cfg(feature = "debug")]
const WALK_SECS: f32 = 3_600.0;
#[cfg(feature = "debug")]
const SETTLE: u32 = 6;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_systems(
            Startup,
            (force_capture_resolution, hide_dev_overlays, hide_hud),
        );
        // The row is posed, so it must not drift under the framing - but only
        // on a capture run, so a plain run keeps its physics.
        app.add_systems(Update, freeze_bodies.run_if(capturing));
        app.init_resource::<Walk>();
        app.add_systems(Update, drive_walk);
        app.add_plugins(showcase_script());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_row);
}

/// Where one design stands and how far its hull reaches.
#[derive(Clone, Debug)]
struct Slot {
    /// The design id, which names its shot.
    design: String,
    /// The centre of the hull's bounding box.
    centre: Meters3,
    /// The hull's longest bounding-box extent.
    length: Meters,
}

/// The row as laid out at load.
#[derive(Resource, Clone, Debug, Default)]
struct Row {
    slots: Vec<Slot>,
}

/// The bounding box of a design's sections, in meters relative to its root:
/// `(min, max)`, each cell taken whole.
fn hull_box(design: &StationDesign) -> (Vec3, Vec3) {
    let (min, max) = design.hull.sections.iter().fold(
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

/// Lay every design out along X, each root placed so its bounding box sits
/// in its slot, and load the row with the rig over it.
fn load_row(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    stations: Res<GameStationDesigns>,
    loaded: Res<LoadedSectionPacks>,
    items: Res<GameItems>,
) {
    let mut catalog: Vec<&StationDesignPrototype> = stations.0.iter().collect();
    catalog.sort_by(|a, b| a.id.cmp(&b.id));
    // (id, name, design, source) for every station in the row.
    let mut roster: Vec<(String, String, StationDesign, StationDesignSource)> = catalog
        .into_iter()
        .map(|prototype| {
            (
                prototype.id.to_string(),
                prototype.name.clone(),
                prototype.design.clone(),
                StationDesignSource::Prototype(prototype.id.clone()),
            )
        })
        .collect();
    for archetype in StationArchetypeType::ALL {
        let (_, design) = generated_station_design(
            &loaded,
            &items,
            archetype,
            GENERATED_ADVANCEMENT,
            GENERATED_SEED,
        );
        roster.push((
            format!("generated_{archetype}"),
            format!("Generated {archetype}"),
            design.clone(),
            StationDesignSource::Inline(design),
        ));
    }
    let mut cursor = 0.0;
    let mut slots = Vec::new();
    let mut objects = Vec::new();
    for (id, name, design, source) in roster {
        let (min, max) = hull_box(&design);
        let size = max - min;
        let root = Vec3::new(
            cursor - min.x,
            -(min.y + max.y) / 2.0,
            -(min.z + max.z) / 2.0,
        );
        let centre = root + (min + max) / 2.0;
        cursor += size.x + SLOT_GAP.get();
        info!(
            "station showcase: '{}' has {} section(s), {:.0} x {:.0} x {:.0} m",
            id,
            design.hull.sections.len(),
            size.x,
            size.y,
            size.z
        );
        slots.push(Slot {
            design: id.clone(),
            centre: Meters3(centre),
            length: Meters(size.max_element()),
        });
        objects.push(EventActionConfig::SpawnScenarioObject(
            ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: format!("showcase_{id}"),
                    name: name.to_uppercase(),
                    position: Meters3(root),
                    rotation: Quat::IDENTITY,
                },
                kind: ScenarioObjectKind::Station(StationConfig {
                    design: source,
                    allegiance: Allegiance::Neutral,
                    derelict: false,
                    initial_velocity: MetersPerSecond3::ZERO,
                }),
            },
        ));
    }
    info!("station showcase: parked {} design(s)", slots.len());
    let middle = Meters3::new((cursor - SLOT_GAP.get()) / 2.0, 0.0, 0.0);
    objects.extend(ThreePointRig::around("rig", middle, 4.0).actions());
    commands.insert_resource(Row { slots });
    commands.trigger(LoadScenario(ScenarioConfig {
        description: "Every station design parked side by side.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: objects,
        }],
        ..ScenarioConfig::new(
            SCENARIO_ID.to_string(),
            "Station Showcase".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }));
}

/// The camera pose that frames a box of `length` around `centre`.
fn frame(centre: Meters3, length: Meters) -> (Meters3, Meters3) {
    let distance = length.get() * FRAME_DISTANCE_FACTOR + FRAME_DISTANCE_MARGIN.get();
    let eye = centre.get() + FRAME_DIRECTION.normalize() * distance;
    (Meters3(eye), centre)
}

/// The pose that frames the whole row.
fn frame_row(row: &Row) -> (Meters3, Meters3) {
    let first = row.slots.first().expect("the row holds a design");
    let last = row.slots.last().expect("the row holds a design");
    let left = first.centre.get().x - first.length.get() / 2.0;
    let right = last.centre.get().x + last.length.get() / 2.0;
    let centre = Meters3::new((left + right) / 2.0, 0.0, 0.0);
    let eye = centre.get() + Vec3::new(0.0, 0.35, 1.0).normalize() * (right - left) * 0.85;
    (Meters3(eye), centre)
}

/// The walk over the slots: which one is posed, the settle frames left, and
/// the shot awaiting its ack.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct Walk {
    active: bool,
    done: bool,
    slot: usize,
    settle: Option<u32>,
    awaiting: Option<String>,
}

/// Every design in the row is up as a station.
#[cfg(feature = "debug")]
fn row_is_up() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let Some(row) = world.get_resource::<Row>() else {
            return false;
        };
        let Some(mut stations) = world.try_query_filtered::<(), With<StationMarker>>() else {
            return false;
        };
        stations.iter(world).count() == row.slots.len()
    })
}

/// Pose each slot in turn, let it settle, shoot it and wait for the file.
#[cfg(feature = "debug")]
fn drive_walk(world: &mut World) {
    {
        let walk = world.resource::<Walk>();
        if !walk.active || walk.done {
            return;
        }
    }
    let slots = world.resource::<Row>().slots.clone();
    if let Some(name) = world.resource::<Walk>().awaiting.clone() {
        if world
            .get_resource::<CaptureLog>()
            .is_some_and(|log| log.wrote(&name))
        {
            let mut walk = world.resource_mut::<Walk>();
            walk.awaiting = None;
            walk.settle = None;
            walk.slot += 1;
        }
        return;
    }
    let (index, settle) = {
        let walk = world.resource::<Walk>();
        (walk.slot, walk.settle)
    };
    let Some(slot) = slots.get(index) else {
        world.resource_mut::<Walk>().done = true;
        info!(
            "station showcase: walk complete ({} design(s))",
            slots.len()
        );
        return;
    };
    match settle {
        None => {
            let (eye, target) = frame(slot.centre, slot.length);
            pose_camera(world, eye, target);
            world.resource_mut::<Walk>().settle = Some(SETTLE);
        }
        Some(0) => {
            let name = format!("station-showcase-{}.png", slot.design);
            shoot(world, &name);
            let mut walk = world.resource_mut::<Walk>();
            if capturing() {
                walk.awaiting = Some(name);
            } else {
                walk.settle = None;
                walk.slot += 1;
            }
        }
        Some(left) => world.resource_mut::<Walk>().settle = Some(left - 1),
    }
}

/// Load the row, shoot it whole, then walk every design.
#[cfg(feature = "debug")]
fn showcase_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
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
        .on_enter(|world: &mut World| {
            let (eye, target) = frame_row(world.resource::<Row>());
            pose_camera(world, eye, target);
        })
        .until(frames(SETTLE))
        .add()
        .step("shoot the row")
        .on_enter(|world: &mut World| shoot(world, "station-showcase.png"))
        .until(shot_written("station-showcase.png"))
        .deadline(SHOT_SECS)
        .add()
        .step("walk every design")
        .on_enter(|world: &mut World| world.resource_mut::<Walk>().active = true)
        .until(resource_where::<Walk>(|walk| walk.done))
        .deadline(WALK_SECS)
        .add()
}
