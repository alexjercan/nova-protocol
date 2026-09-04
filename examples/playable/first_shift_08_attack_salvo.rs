//! first_shift_08_attack_salvo: First Shift's production salvo on its production
//! map, with preview-only ship poses and capture controls.
//!
//! The map, ships, dialogue, weapon actions, lighting, camera cuts, and timing
//! come from `nova_authoring::first_shift_scene`, the same scene fragment used
//! by the shipped chapter. This file owns only the firing-line fixture and the
//! optional capture instrumentation.
//!
//! ```text
//! cargo run --example first_shift_08_attack_salvo --features debug
//! cargo run --example first_shift_08_attack_salvo --features debug -- \
//!     --offset 600,20,-360
//! ```
//!
//! Record the paired railgun impacts:
//! ```text
//! NOVA_CAPTURE_DIR=target/loop-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!     cargo run --example first_shift_08_attack_salvo --features debug
//! ```

use bevy::{prelude::*, window::PrimaryWindow};
use clap::Parser;
use nova_authoring::prelude::*;
use nova_protocol::prelude::*;

/// Preview-only firing-line poses. Mainline reaches them through real player
/// GOTO and scripted warship helm orders.
const CUTTER_PREVIEW_POS: Meters3 = Meters3::new(2_000.0, -600.0, 2_400.0);
const CARRIER_POS: Meters3 = Meters3::new(-1_000.0, 0.0, 2_500.0);
const WARSHIP_PREVIEW_POS: Meters3 = Meters3::new(3_700.0, 150.0, -2_200.0);
const SHIPPED_DEATH_OFFSET: &str = "440,25,-235";
const DEFAULT_CAPTURE_DIR: &str = "target/shots";
const LOOP_WINDOW: &str = "railgun_loop_window";
#[cfg(feature = "debug")]
const LOOP_NAME: &str = "first-shift-railgun-hits";
const STILL_BEATS: [(f64, &str); 4] = [
    (14.0, "run"),
    (17.0, "impact"),
    (21.0, "kill"),
    (29.0, "wreck"),
];

#[derive(Parser, Resource)]
#[command(name = "first_shift_08_attack_salvo")]
struct Cli {
    /// Preview override for the production death-shot offset, in meters.
    #[arg(long, value_name = "X,Y,Z", default_value = SHIPPED_DEATH_OFFSET, value_parser = parse_offset)]
    offset: Meters3,

    /// Window size for interactive review and still capture.
    #[arg(long, value_name = "WxH", default_value = "1280x720", value_parser = parse_resolution)]
    resolution: UVec2,

    /// Capture four review stills under `target/shots` or `NOVA_CAPTURE_DIR`.
    #[arg(long)]
    capture: bool,

    /// Prefix for review still filenames.
    #[arg(long, default_value = "attack")]
    label: String,
}

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
                .step("load the attack salvo")
                .enter(GameStates::Loading)
                .until(player_ship_present())
                .deadline(60.0)
                .add()
                .step("wait for the railgun window")
                .until(scenario_variable_is(LOOP_WINDOW, 1.0))
                .deadline(30.0)
                .add()
                .step("open the railgun loop")
                .on_enter(|world| loop_start(world, LOOP_NAME))
                .until(scenario_variable_is(LOOP_WINDOW, 0.0))
                .deadline(10.0)
                .add()
                .step("close the railgun loop")
                .on_enter(|world| loop_end(world, LOOP_NAME))
                .until(loop_written(LOOP_NAME))
                .deadline(60.0)
                .add(),
        );
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    }

    app.run()
}

fn attack_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load);
    app.add_systems(Update, size_window);
}

fn load(mut commands: Commands, assets: Res<GameAssets>, cli: Res<Cli>) {
    let mut scene = first_shift_scene(
        FirstShiftScene::AttackSalvo,
        assets.cubemap.clone().into(),
        assets.asteroid_texture.clone().into(),
    );
    place_ship(
        &mut scene,
        "cutter",
        CUTTER_PREVIEW_POS,
        facing(CUTTER_PREVIEW_POS, CARRIER_POS),
    );
    place_ship(
        &mut scene,
        "warship",
        WARSHIP_PREVIEW_POS,
        facing(WARSHIP_PREVIEW_POS, CARRIER_POS),
    );
    instrument_salvo(&mut scene, &cli);
    commands.trigger(LoadScenario(scene));
}

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

fn place_ship(scenario: &mut ScenarioConfig, id: &str, position: Meters3, rotation: Quat) {
    for action in scenario
        .events
        .iter_mut()
        .flat_map(|event| event.actions.iter_mut())
    {
        let EventActionConfig::SpawnScenarioObject(object) = action else {
            continue;
        };
        if object.base.id == id {
            object.base.position = position;
            object.base.rotation = rotation;
            return;
        }
    }
    panic!("first_shift_08_attack_salvo: production scene did not spawn '{id}'");
}

fn facing(from: Meters3, to: Meters3) -> Quat {
    Quat::from_rotation_arc(Vec3::NEG_Z, (to.to_engine() - from.to_engine()).normalize())
}

fn set_number(key: &str, value: f64) -> EventActionConfig {
    EventActionConfig::VariableSet(VariableSetActionConfig {
        key: key.to_string(),
        expression: VariableExpressionNode::new_term(VariableTermNode::new_factor(
            VariableFactorNode::new_literal(VariableLiteral::Number(value)),
        )),
    })
}

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

fn instrument_salvo(scenario: &mut ScenarioConfig, cli: &Cli) {
    let sequence = scenario
        .events
        .iter_mut()
        .flat_map(|event| event.actions.iter_mut())
        .find_map(|action| match action {
            EventActionConfig::Sequence(sequence) if sequence.key == "salvo" => Some(sequence),
            _ => None,
        })
        .expect("production AttackSalvo scene has no salvo sequence");

    for step in &mut sequence.steps {
        for action in &mut step.actions {
            if let EventActionConfig::SetCameraAnchor(shot) = action {
                if shot.anchor == "cutter" && matches!(shot.look_at, CameraLookAtConfig::Point(_)) {
                    shot.offset = cli.offset;
                }
            }
        }
    }

    let mut at = 0.0;
    let mut timeline: Vec<(f64, usize, SequenceStepConfig)> = sequence
        .steps
        .drain(..)
        .enumerate()
        .map(|(order, mut step)| {
            at += step.after.unwrap_or(0.0);
            step.after = None;
            (at, order, step)
        })
        .collect();
    let mut order = timeline.len();
    let mut add = |at: f64, actions: Vec<EventActionConfig>| {
        timeline.push((
            at,
            order,
            SequenceStepConfig {
                actions,
                ..default()
            },
        ));
        order += 1;
    };
    add(7.5, vec![set_number(LOOP_WINDOW, 1.0)]);
    add(11.5, vec![set_number(LOOP_WINDOW, 0.0)]);
    if cli.capture {
        for (at, beat) in STILL_BEATS {
            add(at, vec![capture(cli, beat)]);
        }
    }
    timeline.sort_by(|left, right| {
        left.0
            .total_cmp(&right.0)
            .then_with(|| left.1.cmp(&right.1))
    });
    let mut previous = 0.0;
    sequence.steps = timeline
        .into_iter()
        .map(|(at, _, mut step)| {
            step.after = Some(at - previous);
            previous = at;
            step
        })
        .collect();

    scenario.events[0]
        .actions
        .insert(0, set_number(LOOP_WINDOW, 0.0));
}
