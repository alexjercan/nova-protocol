//! screenshot_solar_wing: the derived skin at the open end of a solar wing
//! and a truss spur.
//!
//! One ship, built from the `block_cutter` design with its sections replaced:
//! a 2x2x2 block of hull cubes holding the controller, a solar wing of
//! [`WING`] panels off each X flank, and a [`SPUR`]-cell truss off +Z. The
//! wing tips and the spur end are open sockets nothing mates, so this hull is
//! the visual proof of what the skin does with a socket at the end of a run.
//!
//! Every shot is framed by the hull: the camera stands back by a multiple of
//! the hull's longest extent, and the close shots aim at the cell they show.
//!
//! Two harnessed modes, the fleet's capture idiom:
//! - `NOVA_AUTOPILOT=1`: smoke path - load the ship, walk every framing, exit
//!   clean.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also shoot one
//!   `solar-wing-<shot>.png` per framing (staged under `NOVA_CAPTURE_DIR`).
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=1800 \
//!   cargo run --example screenshot_solar_wing --features debug
//! # look for: `solar wing: walk complete`,
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
#[command(name = "screenshot_solar_wing")]
#[command(version = "1.0.0")]
#[command(about = "The derived skin at the open end of a solar wing and a truss spur. Autopilot-only: a posed ship behind a scripted camera", long_about = None)]
struct Cli;

/// The scenario id the ship loads under.
const SCENARIO_ID: &str = "solar_wing";

/// The catalog design the test hull borrows its style and presentation from.
const BASE_DESIGN: &str = "block_cutter";

/// One build-grid cell, in meters.
const CELL_M: f32 = 10.0;

/// How many solar panels each wing runs out from the block.
const WING: i32 = 3;
/// How many truss cells the spur runs out from the block.
const SPUR: i32 = 2;

/// What a shot looks at.
#[derive(Clone, Copy, Debug)]
enum Subject {
    Hull,
    WingTip,
    SpurEnd,
}

/// One framing: its name, the direction the camera looks from, how far back
/// it stands as a multiple of the hull's longest extent, and what it aims at.
type Shot = (&'static str, Vec3, f32, Subject);

/// The whole hull, then the +X wing tip and the spur end up close.
const SHOTS: [Shot; 3] = [
    ("wide", Vec3::new(0.7, 0.55, 1.0), 1.1, Subject::Hull),
    ("wing-tip", Vec3::new(1.0, 0.6, 0.7), 0.35, Subject::WingTip),
    ("spur-end", Vec3::new(0.6, 0.5, 1.0), 0.35, Subject::SpurEnd),
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
        app.add_plugins(solar_script());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_ship);
}

/// Where the hull, the +X wing tip and the spur end stand, in meters.
#[derive(Resource, Clone, Debug)]
struct Frame {
    hull: Meters3,
    wing_tip: Meters3,
    spur_end: Meters3,
    length: Meters,
}

impl Frame {
    fn aim(&self, subject: Subject) -> Meters3 {
        match subject {
            Subject::Hull => self.hull,
            Subject::WingTip => self.wing_tip,
            Subject::SpurEnd => self.spur_end,
        }
    }
}

/// The hull's sections: the block and its controller, both wings and the
/// spur.
fn sections() -> Vec<SpaceshipSectionConfig> {
    let section = |id: String, x: i32, y: i32, z: i32, prototype: &str| SpaceshipSectionConfig {
        id,
        position: Vec3::new(x as f32, y as f32, z as f32),
        rotation: Quat::IDENTITY,
        source: SectionSource::prototype(prototype),
    };
    let mut sections = Vec::new();
    for x in 0..2 {
        for y in 0..2 {
            for z in 0..2 {
                sections.push(match (x, y, z) == (0, 1, 0) {
                    true => section("bridge".to_string(), x, y, z, "basic_controller_section"),
                    false => section(
                        format!("hull_{x}_{y}_{z}"),
                        x,
                        y,
                        z,
                        "reinforced_hull_section",
                    ),
                });
            }
        }
    }
    for step in 1..=WING {
        sections.push(section(
            format!("wing_east_{step}"),
            1 + step,
            0,
            0,
            "solar_panel_section",
        ));
        sections.push(section(
            format!("wing_west_{step}"),
            -step,
            0,
            0,
            "solar_panel_section",
        ));
    }
    for step in 1..=SPUR {
        sections.push(section(
            format!("spur_{step}"),
            0,
            0,
            1 + step,
            "truss_hull_section",
        ));
    }
    sections
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
        wing_tip: Meters3(Vec3::new((1 + WING) as f32, 0.0, 0.0) * CELL_M),
        spur_end: Meters3(Vec3::new(0.0, 0.0, (1 + SPUR) as f32) * CELL_M),
        length: Meters((max - min).max_element() * CELL_M),
    };
    info!(
        "solar wing: {} section(s), {:.0} m long",
        design.sections.len(),
        frame.length.get()
    );

    let mut actions = vec![EventActionConfig::SpawnScenarioObject(
        ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "solar_wing".to_string(),
                name: "SOLAR WING".to_string(),
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
        description: "The derived skin at the open end of a solar wing.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions,
        }],
        ..ScenarioConfig::new(
            SCENARIO_ID.to_string(),
            "Solar Wing".to_string(),
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
        info!("solar wing: walk complete ({} shot(s))", SHOTS.len());
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
            let name = format!("solar-wing-{suffix}.png");
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
fn solar_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
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
