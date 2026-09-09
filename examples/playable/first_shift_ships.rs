//! first_shift_ships: eight shipped hulls posed side by side for a free-fly
//! visual review.
//!
//! The front row holds the maintenance cutter, the industrial carrier, and the
//! military warship. The rear row holds five cleanup searchers: two unarmed
//! salvage hulls, two PDC-armed escorts, and a PDC escort carrying one Serpent
//! torpedo bay. All eight are shipped base content, so this row is where a
//! silhouette change is reviewed against the ships it has to read apart from -
//! not a place to iterate before promotion.
//!
//! Every hull is spawned by its CATALOG id, so the row poses the shipped ships
//! themselves: a silhouette that moves in `base_content` moves here. Cladding
//! is still derived by the game from the structure - industrial on the cutter
//! and carrier, armoured on the warship. Nothing here flies or fights.
//!
//! Hand-run with the free WASD camera:
//! ```text
//! cargo run --example first_shift_ships --features debug
//! ```
//!
//! Two harnessed modes, the fleet's capture idiom:
//! - `NOVA_AUTOPILOT=1`: smoke path - load the row, frame it, exit clean.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also shoot the row from the parking
//!   pose as `first-shift-ships.png` (staged under `NOVA_CAPTURE_DIR`); the
//!   v0.13.0 post cuts its fleet figure from it.

use std::collections::HashSet;

use bevy::prelude::*;
use nova_authoring::prelude::*;
use nova_protocol::prelude::*;

const CUTTER_POSITION: Meters3 = Meters3::new(-100.0, 0.0, 0.0);
const CARRIER_POSITION: Meters3 = Meters3::ZERO;
const WARSHIP_POSITION: Meters3 = Meters3::new(200.0, 0.0, 0.0);
const SEARCHER_SKIFF_POSITION: Meters3 = Meters3::new(-200.0, 0.0, 350.0);
const SEARCHER_TUG_POSITION: Meters3 = Meters3::new(-100.0, 0.0, 350.0);
const SEARCHER_PICKET_POSITION: Meters3 = Meters3::new(0.0, 0.0, 350.0);
const SEARCHER_CLAW_POSITION: Meters3 = Meters3::new(100.0, 0.0, 350.0);
const SEARCHER_LEADER_POSITION: Meters3 = Meters3::new(220.0, 0.0, 350.0);

fn main() -> bevy::app::AppExit {
    let mut app = AppBuilder::new().with_game_plugins(showcase_plugin).build();

    #[cfg(feature = "debug")]
    {
        // Probe wiring (each plugin is inert without its NOVA_PROBE_* env):
        // run timeline + engine-bound invariants, so `probe run` grades this
        // example instead of hanging on an app with nothing to end it. No
        // frame-time claim - a posed row holds no steady-state load worth
        // grading.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(fleet_script());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_systems(Update, freeze_bodies);
    }

    app.run()
}

fn showcase_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_showcase);
    app.add_systems(Update, frame_new_camera);
}

fn load_showcase(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
    ships: Res<GameShips>,
) {
    let scenario = showcase(&game_assets);
    refuse_broken(&scenario, &sections, &ships);
    commands.trigger(LoadScenario(scenario));
}

fn showcase(game_assets: &GameAssets) -> ScenarioConfig {
    let ships = [
        ship_object(
            "maintenance_cutter",
            "Maintenance Cutter",
            CUTTER_POSITION,
            BLOCK_CUTTER_SHIP_ID,
        ),
        ship_object(
            "industrial_carrier",
            "Industrial Carrier",
            CARRIER_POSITION,
            BLOCK_CARRIER_SHIP_ID,
        ),
        ship_object(
            "stolen_warship",
            "Stolen Military Warship",
            WARSHIP_POSITION,
            BLOCK_WARSHIP_SHIP_ID,
        ),
        ship_object(
            "searcher_skiff",
            "Searcher 1 - Unarmed Skiff",
            SEARCHER_SKIFF_POSITION,
            BLOCK_SKIFF_SHIP_ID,
        ),
        ship_object(
            "searcher_tug",
            "Searcher 2 - Unarmed Tug",
            SEARCHER_TUG_POSITION,
            BLOCK_TUG_SHIP_ID,
        ),
        ship_object(
            "searcher_picket",
            "Searcher 3 - PDC Picket",
            SEARCHER_PICKET_POSITION,
            BLOCK_PICKET_SHIP_ID,
        ),
        ship_object(
            "searcher_claw",
            "Searcher 4 - PDC Claw",
            SEARCHER_CLAW_POSITION,
            BLOCK_CLAW_SHIP_ID,
        ),
        ship_object(
            "searcher_leader",
            "Searcher 5 - PDC and Torpedo Leader",
            SEARCHER_LEADER_POSITION,
            BLOCK_CLEANUP_LEADER_SHIP_ID,
        ),
    ];

    ScenarioConfig {
        description: "Eight shipped hulls, posed side by side".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: ships
                .into_iter()
                .map(EventActionConfig::SpawnScenarioObject)
                .chain(
                    ThreePointRig::around("showcase", Meters3::new(0.0, 0.0, 150.0), 32.0)
                        .actions(),
                )
                .collect(),
        }],
        ..ScenarioConfig::new(
            "first_shift_ships".to_string(),
            "First Shift Ships".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// One posed catalog ship. `ship` is the CATALOG id, so the row shows the
/// shipped hull rather than a copy of it.
fn ship_object(id: &str, name: &str, position: Meters3, ship: &str) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::None,
            hull: hull(ship),
            ..default()
        }),
    }
}

fn refuse_broken(scenario: &ScenarioConfig, sections: &GameSections, ships: &GameShips) {
    let known = KnownSections::from_configs(sections.iter());
    let issues = lint_scenario(
        scenario,
        &known,
        &KnownShips::from_configs(ships.iter()),
        &HashSet::from([scenario.id.clone()]),
        &build_channels()
            .into_iter()
            .map(|channel| channel.id)
            .collect(),
    );
    let errors: Vec<_> = issues
        .iter()
        .filter(|issue| issue.severity == LintSeverity::Error)
        .map(|issue| issue.message.as_str())
        .collect();
    assert!(
        errors.is_empty(),
        "first_shift_ships: posed fleet failed content lint:\n  {}",
        errors.join("\n  "),
    );
}

/// The still the capture run writes: the whole row from [`FLEET_EYE`].
#[cfg(feature = "debug")]
const FLEET_SHOT: &str = "first-shift-ships.png";

/// Where the lens stands for the still, in meters: high and back of the front
/// row, so both rows sit in one frame at a common scale. The row is 500 m
/// across and the lens spans about 1.5 times its distance at 16:9, so 650 m
/// off the front row holds the whole set, carrier stern included.
#[cfg(feature = "debug")]
const FLEET_EYE: Meters3 = Meters3::new(0.0, 340.0, -470.0);

/// What the still looks at, in meters: just behind the front row, so the
/// back row sits in the upper third instead of the middle of the frame.
#[cfg(feature = "debug")]
const FLEET_LOOK: Meters3 = Meters3::new(0.0, 0.0, 80.0);

/// Put the lens on the row and take the HUD (and its status bar) down.
#[cfg(feature = "debug")]
fn frame_the_fleet(world: &mut World) {
    hide_hud(world);
    pose_camera(world, FLEET_EYE, FLEET_LOOK);
}

/// The driven walk: wait for the row and its camera, settle, shoot.
#[cfg(feature = "debug")]
fn fleet_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("wait for the fleet")
        .enter(GameStates::Loading)
        .until(and(
            state_is(GameStates::Playing),
            scenario_camera_present(),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("frame the row")
        .on_enter(frame_the_fleet)
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("shoot the row")
        .on_enter(|world: &mut World| shoot(world, FLEET_SHOT))
        .until(shot_written(FLEET_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}

const CAMERA_TARGET: Vec3 = Vec3::new(0.0, 0.0, 15.0);
const CAMERA_POSITION: Vec3 = Vec3::new(0.0, 55.0, -70.0);

fn frame_new_camera(
    mut cameras: Query<&mut Transform, (With<ScenarioCameraMarker>, Added<ScenarioCameraMarker>)>,
) {
    for mut transform in &mut cameras {
        *transform =
            Transform::from_translation(CAMERA_POSITION).looking_at(CAMERA_TARGET, Vec3::Y);
    }
}
