//! first_shift_ships: the fixed ship candidates for the first two replacement
//! Nova Protocol scenarios, posed for a free-fly visual review.
//!
//! The front row holds the engineer's maintenance cutter, the industrial
//! carrier, and the stolen military warship. The rear row holds five cleanup
//! searchers for the wreck-field sequel: two unarmed salvage hulls, two
//! PDC-armed escorts, and a PDC escort carrying one Serpent torpedo bay. All
//! eight are shipped base content and the campaign flies them, so this row is
//! where a silhouette change is reviewed against the ships it has to read
//! apart from - not a place to iterate before promotion.
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
        app.add_plugins(nova_protocol::nova_debug::harness::nova_screenshot(
            nova_protocol::nova_debug::harness::nova_autopilot(),
        ));
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
        description: "The eight ships of the first two Nova Protocol chapters".to_string(),
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

/// One posed catalog ship. `ship` is the CATALOG id, so the row shows the hull
/// the campaign flies rather than a copy of it.
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
    );
    let errors: Vec<_> = issues
        .iter()
        .filter(|issue| issue.severity == LintSeverity::Error)
        .map(|issue| issue.message.as_str())
        .collect();
    assert!(
        errors.is_empty(),
        "first_shift_ships: candidate fleet failed content lint:\n  {}",
        errors.join("\n  "),
    );
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
