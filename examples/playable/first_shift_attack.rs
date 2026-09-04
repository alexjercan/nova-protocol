//! first_shift_attack: the Meridian's last minute on the shipped stage.
//!
//! First Shift's attack beat with nothing but the beat in it. The three ships
//! stand on their authored marks - the cutter on MERIDIAN HOLD, the Meridian in
//! its berth, the stolen warship on its firing mark - and the real salvo runs
//! from there: six torpedo bays walking open a second apart, two railgun lances,
//! real flight times across the real 3.5 and 6.6 km, and a carrier that comes
//! apart when the ordnance actually reaches it. No delay is guessed and no
//! impact is faked, so what a pose looks like here is what it looks like in the
//! chapter.
//!
//! It exists because a camera offset cannot be judged on paper. The shipped
//! death shot is anchored to the CUTTER and aimed at a POINT - the berth, not
//! the ship standing on it, because an object aim falls back to its anchor the
//! moment the target dies and would swing the lens onto the player's own hull on
//! the one frame that matters. Whether that pose holds the Meridian, keeps the
//! cutter at a controlled edge and leaves the torpedo lane readable is a
//! question for a rendered frame. `--offset` asks it of a new pose.
//!
//! This is the complete First Shift stage, not a three-ship void. Both
//! planetoids, the 40-rock salvage plate and all 20 ambient belt rocks surround
//! the three promoted block ships on their authored marks. The attack framing
//! remains clean because the nearest rock plate falls more than 60 degrees off
//! the shot's axis. After the camera hands itself back, fly Cutter through the
//! same map used by both shipped chapters.
//!
//! Watch it, or fly the map and wreck afterwards:
//! ```text
//! cargo run --example first_shift_attack --features debug
//! cargo run --example first_shift_attack --features debug -- --offset 600,20,-360
//! ```
//!
//! Shoot it. `--capture` adds four stills - the torpedo run, the first impacts,
//! the kill and the wreck - which stage under `NOVA_CAPTURE_DIR` when that is
//! set and under `target/shots/` when it is not:
//! ```text
//! DISPLAY=:99 cargo run --example first_shift_attack --features debug -- \
//!     --resolution 1920x1080 --capture --label shipped
//! ```
//!
//! Record the impact-camera railgun hits as a 720p30 VP9 loop. The same
//! harness walk is an unrecorded smoke when `NOVA_CAPTURE` is omitted:
//! ```text
//! NOVA_CAPTURE_DIR=target/loop-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!     cargo run --example first_shift_attack --features debug
//! ```

#[path = "shared/first_shift_stage.rs"]
mod stage;

use bevy::{prelude::*, window::PrimaryWindow};
use clap::Parser;
use nova_protocol::prelude::*;

/// MERIDIAN HOLD: where the cutter is parked when the shooting starts, and so
/// where the player watches this from. Mirrors `HOME_MARK` in
/// `crates/nova_authoring/src/base_content/scenarios/nova_protocol/first_shift/marks.rs`.
const CUTTER_POS: Meters3 = Meters3::new(2_000.0, -600.0, 2_400.0);

/// The warship's firing mark, 6.6 km off the berth. Mirrors
/// `WARSHIP_FIRING_POS` in the same file.
const WARSHIP_POS: Meters3 = Meters3::new(3_700.0, 150.0, -2_200.0);

/// The shipped salvo camera offsets in world axes. These mirror the three
/// poses in First Shift's `salvo` sequence.
const TUBES_OFFSET: Meters3 = Meters3::new(150.0, 90.0, -300.0);
const IMPACT_OFFSET: Meters3 = Meters3::new(-285.0, 175.0, 595.0);
const SHIPPED_DEATH_OFFSET: &str = "440,25,-235";

/// Where relative captures go when `NOVA_CAPTURE_DIR` names nowhere. A relative
/// `Screenshot` path resolves against the process working directory otherwise,
/// which for `cargo run` is the repository root.
const DEFAULT_CAPTURE_DIR: &str = "target/shots";

/// Scenario latch around the railgun impact window.
const RAILGUN_LOOP_WINDOW: &str = "railgun_loop_window";
/// The webm stem written by the loop recorder.
#[cfg(feature = "debug")]
const RAILGUN_LOOP_NAME: &str = "first-shift-railgun-hits";

const BAYS: [&str; 6] = [
    "bay_port_forward",
    "bay_port_midships",
    "bay_port_aft",
    "bay_starboard_forward",
    "bay_starboard_midships",
    "bay_starboard_aft",
];
const LANCES: [&str; 2] = ["railgun_port", "railgun_starboard"];

/// Absolute seconds from the first tube. These are the cumulative timings of
/// First Shift's shipped sequence, including its dialogue-only gaps. Keeping
/// those silent gaps here makes every camera cut and weapon action happen on
/// the same beat as mainline.
const BAY_GAP: f64 = 1.0;
const RAILGUN_LOOP_START_AT: f64 = 7.0;
const IMPACT_CAMERA_AT: f64 = 7.5;
const LANCES_AT: f64 = 8.0;
const RAILGUN_LOOP_END_AT: f64 = 12.0;
const DEATH_CAMERA_AT: f64 = 20.0;
const RELEASE_AT: f64 = 28.0;
const BEATS: [(f64, &str); 4] = [
    (14.0, "run"),
    (17.0, "impact"),
    (21.0, "kill"),
    (29.0, "wreck"),
];

#[derive(Parser, Resource)]
#[command(name = "first_shift_attack")]
#[command(version = "1.0.0")]
#[command(about = "The First Shift attack scene on its shipped map", long_about = None)]
struct Cli {
    /// Death-shot offset from the cutter, in meters on world axes: `X,Y,Z`.
    #[arg(long, value_name = "X,Y,Z", default_value = SHIPPED_DEATH_OFFSET, value_parser = parse_offset)]
    offset: Meters3,

    /// Window size, `WIDTHxHEIGHT`. Framing is a function of aspect ratio.
    #[arg(long, value_name = "WxH", default_value = "1280x720", value_parser = parse_resolution)]
    resolution: UVec2,

    /// Also shoot the four beat stills.
    #[arg(long)]
    capture: bool,

    /// Leading name for the captured files, so two poses do not overwrite.
    #[arg(long, default_value = "attack")]
    label: String,
}

/// `X,Y,Z` in meters.
fn parse_offset(raw: &str) -> Result<Meters3, String> {
    let axes: Vec<&str> = raw.split(',').collect();
    let [x, y, z] = axes[..] else {
        return Err(format!("expected X,Y,Z in meters, got '{raw}'"));
    };
    let axis = |value: &str| {
        value
            .trim()
            .parse::<f32>()
            .map_err(|error| error.to_string())
    };
    Ok(Meters3::new(axis(x)?, axis(y)?, axis(z)?))
}

/// `WIDTHxHEIGHT` in pixels.
fn parse_resolution(raw: &str) -> Result<UVec2, String> {
    let Some((width, height)) = raw.split_once('x') else {
        return Err(format!("expected WIDTHxHEIGHT, got '{raw}'"));
    };
    let side = |value: &str| {
        value
            .trim()
            .parse::<u32>()
            .map_err(|error| error.to_string())
    };
    Ok(UVec2::new(side(width)?, side(height)?))
}

fn main() -> bevy::app::AppExit {
    let cli = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(attack_plugin).build();
    app.insert_resource(cli);

    #[cfg(feature = "debug")]
    {
        app.add_plugins(LoopCapturePlugin::default());
        app.add_plugins(
            nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
                .step("load the attack scene")
                .enter(GameStates::Loading)
                .until(player_ship_present())
                .deadline(60.0)
                .add()
                .step("wait for the railgun window")
                .until(scenario_variable_is(RAILGUN_LOOP_WINDOW, 1.0))
                .deadline(30.0)
                .add()
                .step("open the railgun loop")
                .on_enter(|world| loop_start(world, RAILGUN_LOOP_NAME))
                .until(scenario_variable_is(RAILGUN_LOOP_WINDOW, 0.0))
                .deadline(10.0)
                .add()
                .step("close the railgun loop")
                .on_enter(|world| loop_end(world, RAILGUN_LOOP_NAME))
                .until(loop_written(RAILGUN_LOOP_NAME))
                .deadline(60.0)
                .add(),
        );
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    }

    app.run()
}

fn attack_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load);
    app.add_systems(Update, (size_window, trace));
}

fn load(mut commands: Commands, game_assets: Res<GameAssets>, cli: Res<Cli>) {
    commands.trigger(LoadScenario(scenario(&game_assets, &cli)));
}

/// The window is sized once, after the primary window exists.
fn size_window(
    mut done: Local<bool>,
    cli: Res<Cli>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if *done {
        return;
    }
    #[cfg(feature = "debug")]
    if capturing() {
        *done = true;
        return;
    }
    let Ok(mut window) = windows.single_mut() else {
        return;
    };
    window
        .resolution
        .set(cli.resolution.x as f32, cli.resolution.y as f32);
    *done = true;
}

fn ship(
    id: &str,
    name: &str,
    position: Meters3,
    prototype: &str,
    rotation: Quat,
) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
            rotation,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: if id == "cutter" {
                SpaceshipController::Player(PlayerControllerConfig::default())
            } else {
                SpaceshipController::None
            },
            hull: ShipSource::Prototype(prototype.to_string()),
            ..default()
        }),
    }
}

/// Face `from` at `to`: hulls are built nose along -Z.
fn facing(from: Meters3, to: Meters3) -> Quat {
    let direction = (to.to_engine() - from.to_engine()).normalize();
    Quat::from_rotation_arc(Vec3::NEG_Z, direction)
}

fn torpedo(bay: &str) -> EventActionConfig {
    EventActionConfig::ForceTorpedoFire(ForceTorpedoFireActionConfig {
        ship: "warship".to_string(),
        section: bay.to_string(),
        target: "carrier".to_string(),
    })
}

fn lance(id: &str) -> EventActionConfig {
    EventActionConfig::ForceRailgunFire(ForceRailgunFireActionConfig {
        ship: "warship".to_string(),
        section: id.to_string(),
    })
}

fn set_number(key: &str, value: f64) -> EventActionConfig {
    EventActionConfig::VariableSet(VariableSetActionConfig {
        key: key.to_string(),
        expression: VariableExpressionNode::new_term(VariableTermNode::new_factor(
            VariableFactorNode::new_literal(VariableLiteral::Number(value)),
        )),
    })
}

fn film(anchor: &str, offset: Meters3, look_at: CameraLookAtConfig) -> EventActionConfig {
    EventActionConfig::SetCameraAnchor(SetCameraAnchorActionConfig {
        anchor: anchor.to_string(),
        offset,
        frame: CameraOffsetFrame::World,
        look_at,
    })
}

/// Relative captures stage under `NOVA_CAPTURE_DIR`, and under
/// [`DEFAULT_CAPTURE_DIR`] when nothing sets it.
fn capture(cli: &Cli, beat: &str) -> EventActionConfig {
    let name = format!(
        "{}_{}x{}_{beat}.png",
        cli.label, cli.resolution.x, cli.resolution.y
    );
    let path = if std::env::var_os(CAPTURE_DIR_ENV).is_some() {
        name
    } else {
        format!("{DEFAULT_CAPTURE_DIR}/{name}")
    };
    EventActionConfig::Screenshot(ScreenshotActionConfig::new(&path))
}

/// The salvo as absolute seconds, turned into the gaps a sequence wants. Every
/// beat keeps its own time whether or not the run is capturing.
fn sequence(cli: &Cli) -> Vec<SequenceStepConfig> {
    let mut beats: Vec<(f64, Vec<EventActionConfig>)> = BAYS
        .iter()
        .enumerate()
        .map(|(index, bay)| (index as f64 * BAY_GAP, vec![torpedo(bay)]))
        .collect();
    beats[0].1.splice(
        0..0,
        [
            EventActionConfig::SuspendPlayerControl(SuspendPlayerControlActionConfig),
            film(
                "warship",
                TUBES_OFFSET,
                CameraLookAtConfig::Object("carrier".to_string()),
            ),
        ],
    );
    beats.extend([
        (
            RAILGUN_LOOP_START_AT,
            vec![set_number(RAILGUN_LOOP_WINDOW, 1.0)],
        ),
        (
            IMPACT_CAMERA_AT,
            vec![film(
                "carrier",
                IMPACT_OFFSET,
                CameraLookAtConfig::Object("warship".to_string()),
            )],
        ),
        (
            LANCES_AT,
            LANCES.iter().map(|railgun| lance(railgun)).collect(),
        ),
        (
            RAILGUN_LOOP_END_AT,
            vec![set_number(RAILGUN_LOOP_WINDOW, 0.0)],
        ),
        (
            DEATH_CAMERA_AT,
            vec![film(
                "cutter",
                cli.offset,
                CameraLookAtConfig::Point(stage::CARRIER_POS),
            )],
        ),
        (
            RELEASE_AT,
            vec![
                EventActionConfig::ReleaseCamera(ReleaseCameraActionConfig),
                EventActionConfig::ResumePlayerControl(ResumePlayerControlActionConfig),
            ],
        ),
    ]);
    if cli.capture {
        beats.extend(BEATS.map(|(at, beat)| (at, vec![capture(cli, beat)])));
    }
    beats.sort_by(|left, right| left.0.total_cmp(&right.0));

    let mut previous = 0.0;
    beats
        .into_iter()
        .map(|(at, actions)| {
            let after = at - previous;
            previous = at;
            SequenceStepConfig {
                after: Some(after),
                actions,
                ..default()
            }
        })
        .collect()
}

fn scenario(assets: &GameAssets, cli: &Cli) -> ScenarioConfig {
    let mut objects = stage::belt(&assets.asteroid_texture);
    objects.extend(
        ThreePointRig::around("first_shift", Meters3::new(0.0, 0.0, -2_000.0), 25.0).objects(),
    );
    objects.extend([
        ship(
            "cutter",
            "Cutter",
            CUTTER_POS,
            "block_cutter",
            facing(CUTTER_POS, stage::CARRIER_POS),
        ),
        ship(
            "carrier",
            "ICV Meridian",
            stage::CARRIER_POS,
            "block_carrier",
            Quat::IDENTITY,
        ),
        ship(
            "warship",
            "Warship",
            WARSHIP_POS,
            "block_warship",
            facing(WARSHIP_POS, stage::CARRIER_POS),
        ),
    ]);

    ScenarioConfig {
        description: "First Shift's attack scene on its complete shipped stage".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: objects
                .into_iter()
                .map(EventActionConfig::SpawnScenarioObject)
                .chain([
                    set_number(RAILGUN_LOOP_WINDOW, 0.0),
                    EventActionConfig::Sequence(SequenceActionConfig {
                        key: "salvo".to_string(),
                        steps: sequence(cli),
                    }),
                ])
                .collect(),
        }],
        ..ScenarioConfig::new(
            "first_shift_attack".to_string(),
            "First Shift Attack".to_string(),
            assets.cubemap.clone().into(),
        )
    }
}

#[derive(Default)]
struct Trace {
    next: f32,
}

/// One line a second: which ships are alive and how much ordnance is still in
/// the air. This is where the beat times above came from, and it is how a
/// retimed capture is re-derived rather than guessed.
fn trace(
    time: Res<Time>,
    mut trace: Local<Trace>,
    ships: Query<&EntityId, With<SpaceshipRootMarker>>,
    torpedoes: Query<Entity, With<TorpedoProjectileMarker>>,
) {
    let now = time.elapsed_secs();
    if now < trace.next {
        return;
    }
    trace.next = now + 1.0;
    let alive: Vec<&str> = ships.iter().map(|id| id.0.as_str()).collect();
    info!(
        "attack t={now:6.1}s ships={alive:?} torpedoes={}",
        torpedoes.iter().count()
    );
}
