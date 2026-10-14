//! screenshot_base_fleet: every shipped catalog ship, one framed still each.
//!
//! Spawns the base fleet in a row, inert and frozen, under the photo rig, and
//! frames each hull three-quarters from above its port bow, the flank every
//! crewed hull carries its docking collar on. One still per ship:
//! `fleet-<ship id>.png`. The shots are for reviewing a hull, so no manifest
//! image ships from here.
//!
//! - `NOVA_AUTOPILOT=1` alone: walk the framings and exit clean, capturing
//!   nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also capture each still (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! ```text
//! NOVA_CAPTURE_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_base_fleet --features debug
//! ```

#[path = "shared/kit.rs"]
mod kit;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "screenshot_base_fleet")]
#[command(version = "1.0.0")]
#[command(about = "Every shipped catalog ship, framed one at a time for review. Autopilot-only: a posed row behind a scripted camera", long_about = None)]
struct Cli;

/// The fleet, in catalog order, with the eye distance each hull needs: the
/// start hulls are about a hundred meters long, the line warship twice that.
const FLEET: [(&str, f32); 9] = [
    ("mining_tug", 1.0),
    ("salvage_hauler", 1.0),
    ("light_freighter", 1.0),
    ("interceptor", 0.9),
    ("raider", 0.9),
    ("patrol_picket", 0.8),
    ("line_warship", 2.0),
    ("freighter_derelict", 1.0),
    ("debris_plate", 0.6),
];

/// Space between two hulls in the row: wider than the longest hull, so no
/// framing catches a neighbour.
const SPACING: Meters = Meters(600.0);

/// The eye's offset from a hull at distance factor 1: above the port bow.
const EYE: Meters3 = Meters3::new(-110.0, 55.0, -95.0);

fn slot(index: usize) -> Meters3 {
    Meters3::new(SPACING.0 * index as f32, 0.0, 0.0)
}

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_systems(
            Startup,
            (force_capture_resolution, hide_dev_overlays, hide_hud),
        );
        app.add_systems(Update, freeze_bodies);
        let mut script = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
            .step("wait for the fleet row")
            .enter(GameStates::Loading)
            .until(and(
                state_is(GameStates::Playing),
                scenario_camera_present(),
            ))
            .deadline(STEP_DEADLINE_SECS)
            .add();
        for (index, (id, scale)) in FLEET.into_iter().enumerate() {
            let shot = format!("fleet-{id}.png");
            let centre = slot(index);
            script = script
                .step(format!("frame {shot}"))
                .on_enter(move |world: &mut World| {
                    pose_camera(world, centre + EYE * scale, centre);
                })
                .until(frames(SETTLE_FRAMES))
                .add();
            let name = shot.clone();
            script = script
                .step(format!("shoot {shot}"))
                .on_enter(move |world: &mut World| shoot(world, &name))
                .until(shot_written(&shot))
                .deadline(SHOT_DEADLINE_SECS)
                .add();
        }
        app.add_plugins(script);
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShipDesigns>) {
    let hulls = FLEET.iter().enumerate().map(|(index, (id, _))| {
        EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: format!("fleet_{id}"),
                name: id.to_string(),
                position: slot(index),
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
                controller: SpaceshipController::None,
                allegiance: Some(Allegiance::Neutral),
                design: ShipDesignSource::Inline(kit::catalog_ship(&ships, id)),
                ..default()
            }),
        })
    });
    // One rig: its lights are directional, so a rig per hull would stack.
    let rigs = ThreePointRig::around("photo", Meters3::ZERO, 1.0).actions();
    commands.trigger(LoadScenario(ScenarioConfig {
        description: "The base fleet in a row, for review.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: hulls.chain(rigs).collect(),
        }],
        ..ScenarioConfig::new(
            "base_fleet".to_string(),
            "Base Fleet".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }));
}
