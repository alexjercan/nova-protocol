//! screenshot_diagonal_skin: the derived skin over hull cubes that touch
//! along an edge only.
//!
//! One ship, built from the `block_cutter` design with its sections replaced:
//! a one-cell STAIRCASE and a 2x2 STEP BLOCK, each step touching the next
//! along an edge, run four cells deep and tied together through a back layer
//! that also holds the controller. A diagonal is where a corner-sampled skin
//! drew spikes and notches, so this hull is the visual proof that a stair is
//! clad as one continuous ramp and a step block as clean slabs.
//!
//! Every shot is framed by the hull: the camera stands back by a multiple of
//! the hull's longest extent, and the close shots aim at the centre of the
//! cells they show.
//!
//! Two harnessed modes, the fleet's capture idiom:
//! - `NOVA_AUTOPILOT=1`: smoke path - load the ship, walk every framing, exit
//!   clean.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also shoot one
//!   `diagonal-skin-<shot>.png` per framing (staged under `NOVA_CAPTURE_DIR`).
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=1800 \
//!   cargo run --example screenshot_diagonal_skin --features debug
//! # look for: `diagonal skin: walk complete`,
//! #           `autopilot: cycle complete, no panic`
//! ```

use bevy::prelude::*;
use clap::Parser;
// Direct, not through `nova_protocol::nova_debug`: that path only exists under
// the `debug` feature, and `capturing()` gates the body freeze in EVERY build.
#[cfg(feature = "debug")]
use nova_debug::prelude::capturing;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "screenshot_diagonal_skin")]
#[command(version = "1.0.0")]
#[command(about = "The derived skin over a staircase and a step block of hull cubes. Autopilot-only: a posed ship behind a scripted camera", long_about = None)]
struct Cli;

/// The scenario id the ship loads under.
const SCENARIO_ID: &str = "diagonal_skin";

/// The catalog design the test hull borrows its style and presentation from.
const BASE_DESIGN: &str = "block_cutter";

/// One build-grid cell, in meters.
const CELL_M: f32 = 10.0;

/// The front of each step, as `(x, y)` cells, run four cells deep in z.
const STAIR: [(i32, i32); 3] = [(0, 0), (1, 1), (2, 2)];
const BLOCK: [(i32, i32); 8] = [
    (5, 0),
    (6, 0),
    (5, 1),
    (6, 1),
    (7, 2),
    (8, 2),
    (7, 3),
    (8, 3),
];
/// How deep the steps run, in cells.
const DEPTH: i32 = 4;
/// The back layer at `z = -1`, which ties the stair and the block into one
/// hull, and the cell in it that holds the controller.
const BACK: [(i32, i32); 17] = [
    (0, 0),
    (1, 0),
    (1, 1),
    (2, 1),
    (2, 2),
    (3, 1),
    (4, 1),
    (5, 0),
    (6, 0),
    (5, 1),
    (6, 1),
    (7, 1),
    (7, 2),
    (8, 2),
    (7, 3),
    (8, 3),
    (4, 0),
];
const BRIDGE: (i32, i32) = (4, 0);

/// What a shot looks at.
#[derive(Clone, Copy, Debug)]
enum Subject {
    Hull,
    Stair,
    Block,
}

/// One framing: its name, the direction the camera looks from, how far back
/// it stands as a multiple of the hull's longest extent, and what it aims at.
type Shot = (&'static str, Vec3, f32, Subject);

/// The whole hull, then each diagonal from the +X/-Y side (the valley under
/// the steps), end on, and from the -X/+Y side (over the treads).
const SHOTS: [Shot; 7] = [
    ("wide", Vec3::new(0.7, 0.55, 1.0), 1.5, Subject::Hull),
    (
        "stair-valley",
        Vec3::new(1.0, -1.0, 0.8),
        0.6,
        Subject::Stair,
    ),
    (
        "stair-front",
        Vec3::new(0.25, 0.2, 1.0),
        0.65,
        Subject::Stair,
    ),
    (
        "stair-upper",
        Vec3::new(-1.0, 1.0, 0.8),
        0.6,
        Subject::Stair,
    ),
    (
        "block-valley",
        Vec3::new(1.0, -1.0, 0.8),
        0.75,
        Subject::Block,
    ),
    (
        "block-front",
        Vec3::new(0.25, 0.2, 1.0),
        0.8,
        Subject::Block,
    ),
    (
        "block-upper",
        Vec3::new(-1.0, 1.0, 0.8),
        0.75,
        Subject::Block,
    ),
];

/// Real seconds the load and the walk may take on a software rasterizer, and
/// the frames a framing settles for before its shot.
#[cfg(feature = "debug")]
const LOAD_SECS: f32 = 900.0;
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
        // The ship is posed, so it must not drift under the framing - but
        // only on a capture run, so a plain run keeps its physics.
        app.add_systems(Update, freeze_bodies.run_if(capturing));
        app.init_resource::<Walk>();
        app.add_systems(Update, drive_walk);
        app.add_plugins(diagonal_script());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_ship);
}

/// Where the hull and its two diagonals stand, in meters.
#[derive(Resource, Clone, Debug)]
struct Frame {
    hull: Meters3,
    stair: Meters3,
    block: Meters3,
    length: Meters,
}

impl Frame {
    fn aim(&self, subject: Subject) -> Meters3 {
        match subject {
            Subject::Hull => self.hull,
            Subject::Stair => self.stair,
            Subject::Block => self.block,
        }
    }
}

/// The hull's sections: the steps, then the back layer and its controller.
fn sections() -> Vec<SpaceshipSectionConfig> {
    let section = |id: String, x: i32, y: i32, z: i32, prototype: &str| SpaceshipSectionConfig {
        id,
        position: Vec3::new(x as f32, y as f32, z as f32),
        rotation: Quat::IDENTITY,
        source: SectionSource::prototype(prototype),
    };
    let mut sections = Vec::new();
    for z in 0..DEPTH {
        for (x, y) in STAIR.iter().chain(&BLOCK) {
            sections.push(section(
                format!("hull_{x}_{y}_{z}"),
                *x,
                *y,
                z,
                "reinforced_hull_section",
            ));
        }
    }
    for (x, y) in BACK {
        sections.push(match (x, y) == BRIDGE {
            true => section("bridge".to_string(), x, y, -1, "basic_controller_section"),
            false => section(
                format!("hull_{x}_{y}_back"),
                x,
                y,
                -1,
                "reinforced_hull_section",
            ),
        });
    }
    sections
}

/// The centre of the cells of one run of steps, in meters.
fn centre_of(steps: &[(i32, i32)]) -> Vec3 {
    let sum = steps
        .iter()
        .map(|(x, y)| Vec3::new(*x as f32, *y as f32, (DEPTH - 1) as f32 / 2.0))
        .sum::<Vec3>();
    sum / steps.len() as f32 * CELL_M
}

/// Spawn the test hull at the origin and load it with the rig over it.
fn load_ship(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShipDesigns>) {
    let base = ships
        .0
        .iter()
        .find(|design| design.id.to_string() == BASE_DESIGN)
        .unwrap_or_else(|| panic!("the `{BASE_DESIGN}` design is in the catalog"));
    let mut design = base.design.clone();
    design.sections = sections();

    let (min, max) = design.sections.iter().fold(
        (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
        |(min, max), section| {
            (
                min.min(section.position - Vec3::splat(0.5)),
                max.max(section.position + Vec3::splat(0.5)),
            )
        },
    );
    let frame = Frame {
        hull: Meters3((min + max) / 2.0 * CELL_M),
        stair: Meters3(centre_of(&STAIR)),
        block: Meters3(centre_of(&BLOCK)),
        length: Meters((max - min).max_element() * CELL_M),
    };
    info!(
        "diagonal skin: {} section(s), {:.0} m long",
        design.sections.len(),
        frame.length.get()
    );

    let mut actions = vec![EventActionConfig::SpawnScenarioObject(
        ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "diagonal".to_string(),
                name: "DIAGONAL".to_string(),
                position: Meters3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
                design: ShipDesignSource::Inline(design),
                allegiance: Some(Allegiance::Neutral),
                ..default()
            }),
        },
    )];
    actions.extend(ThreePointRig::around("rig", frame.hull, 4.0).actions());
    commands.insert_resource(frame);
    commands.trigger(LoadScenario(ScenarioConfig {
        description: "The derived skin over a staircase and a step block.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions,
        }],
        ..ScenarioConfig::new(
            SCENARIO_ID.to_string(),
            "Diagonal Skin".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }));
}

/// The walk over the shots: which one is posed, the settle frames left, and
/// the shot awaiting its ack.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct Walk {
    active: bool,
    done: bool,
    shot: usize,
    settle: Option<u32>,
    awaiting: Option<String>,
}

/// The test hull is up as a ship.
#[cfg(feature = "debug")]
fn ship_is_up() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let Some(mut roots) = world.try_query_filtered::<(), With<SpaceshipRootMarker>>() else {
            return false;
        };
        roots.iter(world).count() >= 1
    })
}

/// Pose each shot in turn, let it settle, shoot it and wait for the file.
#[cfg(feature = "debug")]
fn drive_walk(world: &mut World) {
    {
        let walk = world.resource::<Walk>();
        if !walk.active || walk.done {
            return;
        }
    }
    let frame = world.resource::<Frame>().clone();
    if let Some(name) = world.resource::<Walk>().awaiting.clone() {
        if world
            .get_resource::<CaptureLog>()
            .is_some_and(|log| log.wrote(&name))
        {
            let mut walk = world.resource_mut::<Walk>();
            walk.awaiting = None;
            walk.settle = None;
            walk.shot += 1;
        }
        return;
    }
    let (index, settle) = {
        let walk = world.resource::<Walk>();
        (walk.shot, walk.settle)
    };
    let Some((suffix, direction, factor, subject)) = SHOTS.get(index).copied() else {
        world.resource_mut::<Walk>().done = true;
        info!("diagonal skin: walk complete ({} shot(s))", SHOTS.len());
        return;
    };
    match settle {
        None => {
            let target = frame.aim(subject);
            let eye = target.get() + direction.normalize() * frame.length.get() * factor;
            pose_camera(world, Meters3(eye), target);
            world.resource_mut::<Walk>().settle = Some(SETTLE);
        }
        Some(0) => {
            let name = format!("diagonal-skin-{suffix}.png");
            shoot(world, &name);
            let mut walk = world.resource_mut::<Walk>();
            if capturing() {
                walk.awaiting = Some(name);
            } else {
                walk.settle = None;
                walk.shot += 1;
            }
        }
        Some(left) => world.resource_mut::<Walk>().settle = Some(left - 1),
    }
}

/// Load the ship, then walk every framing.
#[cfg(feature = "debug")]
fn diagonal_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("wait for the ship")
        .enter(GameStates::Loading)
        .until(and(
            and(state_is(GameStates::Playing), scenario_camera_present()),
            and(scenario_is_built(), ship_is_up()),
        ))
        .deadline(LOAD_SECS)
        .add()
        .step("walk every framing")
        .on_enter(|world: &mut World| world.resource_mut::<Walk>().active = true)
        .until(resource_where::<Walk>(|walk| walk.done))
        .deadline(WALK_SECS)
        .add()
}
