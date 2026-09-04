//! first_shift_setpiece: the Meridian's last minute, staged for the lens.
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
//! The belt is not staged. At these three marks the nearest rock plate falls
//! more than 60 degrees off the shot's axis, so the frame is ships and sky.
//!
//! Watch it, or fly the wreck afterwards - the camera hands itself back at the
//! end:
//! ```text
//! cargo run --example first_shift_setpiece --features debug
//! cargo run --example first_shift_setpiece --features debug -- --offset 600,20,-360
//! ```
//!
//! Shoot it. `--capture` adds four stills - the torpedo run, the first impacts,
//! the kill and the wreck - which stage under `NOVA_CAPTURE_DIR` when that is
//! set and under `target/shots/` when it is not:
//! ```text
//! DISPLAY=:99 cargo run --example first_shift_setpiece --features debug -- \
//!     --resolution 1920x1080 --capture --label shipped
//! ```

#[expect(
    dead_code,
    reason = "the shared stage module also carries both map benches' asteroid layout"
)]
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

/// The shipped death-shot offset from the cutter, in world axes. Mirrors
/// `CINEMA_DEATH_OFFSET`.
const SHIPPED_OFFSET: &str = "440,25,-235";

/// Where relative captures go when `NOVA_CAPTURE_DIR` names nowhere. A relative
/// `Screenshot` path resolves against the process working directory otherwise,
/// which for `cargo run` is the repository root.
const DEFAULT_CAPTURE_DIR: &str = "target/shots";

const BAYS: [&str; 6] = [
    "bay_port_forward",
    "bay_port_midships",
    "bay_port_aft",
    "bay_starboard_forward",
    "bay_starboard_midships",
    "bay_starboard_aft",
];
const LANCES: [&str; 2] = ["railgun_port", "railgun_starboard"];

/// Seconds from the first tube. The gaps between the shots are the authored
/// cadence; the times after them were read off this bench's own trace, which
/// prints the live ship and torpedo count once a second.
///
/// A slug crosses the 6.6 km lane in under half a second and a torpedo needs
/// about sixteen, so the lances land first and alone, the camera poses at 10 s
/// with six torpedoes still in the air, and the carrier comes apart when they
/// arrive. The exact second of a hit moves a little with the frame rate; the
/// trace is how a beat gets re-timed rather than guessed.
const BAY_GAP: f64 = 1.0;
const FIRST_LANCE_AT: f64 = 8.0;
const SECOND_LANCE_AT: f64 = 9.5;
const POSE_AT: f64 = 10.0;
const BEATS: [(f64, &str); 4] = [
    (14.0, "run"),
    (17.0, "impact"),
    (21.0, "kill"),
    (28.0, "wreck"),
];
const RELEASE_AT: f64 = 31.0;

#[derive(Parser, Resource)]
#[command(name = "first_shift_setpiece")]
#[command(version = "1.0.0")]
#[command(about = "The First Shift destruction set piece, staged for camera review", long_about = None)]
struct Cli {
    /// Death-shot offset from the cutter, in meters on world axes: `X,Y,Z`.
    #[arg(long, value_name = "X,Y,Z", default_value = SHIPPED_OFFSET, value_parser = parse_offset)]
    offset: Meters3,

    /// Window size, `WIDTHxHEIGHT`. Framing is a function of aspect ratio.
    #[arg(long, value_name = "WxH", default_value = "1280x720", value_parser = parse_resolution)]
    resolution: UVec2,

    /// Also shoot the four beat stills.
    #[arg(long)]
    capture: bool,

    /// Leading name for the captured files, so two poses do not overwrite.
    #[arg(long, default_value = "setpiece")]
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
    let mut app = AppBuilder::new().with_game_plugins(setpiece_plugin).build();
    app.insert_resource(cli);
    app.run()
}

fn setpiece_plugin(app: &mut App) {
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

fn pose(offset: Meters3) -> EventActionConfig {
    EventActionConfig::SetCameraAnchor(SetCameraAnchorActionConfig {
        anchor: "cutter".to_string(),
        offset,
        frame: CameraOffsetFrame::World,
        // A POINT, not the carrier object: the aim has to outlive the target.
        look_at: CameraLookAtConfig::Point(stage::CARRIER_POS),
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
    beats.push((FIRST_LANCE_AT, vec![lance(LANCES[0])]));
    beats.push((SECOND_LANCE_AT, vec![lance(LANCES[1])]));
    beats.push((POSE_AT, vec![pose(cli.offset)]));
    if cli.capture {
        beats.extend(BEATS.map(|(at, beat)| (at, vec![capture(cli, beat)])));
    }
    beats.push((
        RELEASE_AT,
        vec![EventActionConfig::ReleaseCamera(ReleaseCameraActionConfig)],
    ));

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
    let objects = vec![
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
    ];

    ScenarioConfig {
        description: "First Shift's destruction set piece, staged for the camera".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: objects
                .into_iter()
                .map(EventActionConfig::SpawnScenarioObject)
                .chain([EventActionConfig::Sequence(SequenceActionConfig {
                    key: "salvo".to_string(),
                    steps: sequence(cli),
                })])
                .collect(),
        }],
        ..ScenarioConfig::new(
            "setpiece_frame".to_string(),
            "Set Piece Framing".to_string(),
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
        "setpiece t={now:6.1}s ships={alive:?} torpedoes={}",
        torpedoes.iter().count()
    );
}
