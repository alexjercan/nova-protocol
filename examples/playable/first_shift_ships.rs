//! first_shift_ships: the three fixed ship candidates for Nova Protocol's
//! replacement opening, posed side by side for a free-fly visual review.
//!
//! From left to right: the engineer's small industrial maintenance cutter just
//! clear of its empty port berth, the much larger unarmed industrial carrier
//! with a second cutter built vertically into its starboard side, and the stolen
//! armoured warship that destroys the carrier. These are candidate structures,
//! not promoted base-content ships. Iterate here before the campaign depends on
//! their silhouettes.
//!
//! The ships are hand-authored and reproducible. Their cladding is still
//! derived by the game from the structure: industrial on the cutter and
//! carrier, armoured on the warship. Nothing in the example flies or fights.
//!
//! Hand-run with the free WASD camera:
//! ```text
//! cargo run --example first_shift_ships --features debug
//! ```

#[path = "shared/first_shift.rs"]
mod first_shift;

use std::collections::HashSet;

use bevy::prelude::*;
use nova_protocol::prelude::*;

const CUTTER_POSITION: Meters3 = Meters3::new(-100.0, 0.0, 0.0);
const CARRIER_POSITION: Meters3 = Meters3::ZERO;
const WARSHIP_POSITION: Meters3 = Meters3::new(200.0, 0.0, 0.0);

fn main() -> bevy::app::AppExit {
    let mut app = AppBuilder::new().with_game_plugins(showcase_plugin).build();

    #[cfg(feature = "debug")]
    app.add_systems(Update, freeze_bodies);

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
) {
    let scenario = showcase(&game_assets);
    refuse_broken(&scenario, &sections);
    commands.trigger(LoadScenario(scenario));
}

fn showcase(game_assets: &GameAssets) -> ScenarioConfig {
    let ships = [
        ship_object(
            "maintenance_cutter",
            "Maintenance Cutter",
            CUTTER_POSITION,
            first_shift::maintenance_cutter(),
        ),
        ship_object(
            "industrial_carrier",
            "Industrial Carrier",
            CARRIER_POSITION,
            first_shift::industrial_carrier(),
        ),
        ship_object(
            "stolen_warship",
            "Stolen Military Warship",
            WARSHIP_POSITION,
            first_shift::stolen_warship(),
        ),
    ];

    ScenarioConfig {
        description: "Three candidate ships for the first Nova Protocol scenario".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: ships
                .into_iter()
                .map(EventActionConfig::SpawnScenarioObject)
                .chain(ThreePointRig::around("showcase", Meters3::ZERO, 18.0).actions())
                .collect(),
        }],
        ..ScenarioConfig::new(
            "first_shift_ships".to_string(),
            "First Shift Ships".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

fn ship_object(id: &str, name: &str, position: Meters3, hull: ShipHull) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::None,
            hull: ShipSource::Inline(hull),
            ..default()
        }),
    }
}

fn refuse_broken(scenario: &ScenarioConfig, sections: &GameSections) {
    let known = KnownSections::from_configs(sections.iter());
    let issues = lint_scenario(
        scenario,
        &known,
        &KnownShips::default(),
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

const CAMERA_TARGET: Vec3 = Vec3::new(0.0, 0.0, 1.0);
const CAMERA_POSITION: Vec3 = Vec3::new(0.0, 28.0, -50.0);

fn frame_new_camera(
    mut cameras: Query<&mut Transform, (With<ScenarioCameraMarker>, Added<ScenarioCameraMarker>)>,
) {
    for mut transform in &mut cameras {
        *transform =
            Transform::from_translation(CAMERA_POSITION).looking_at(CAMERA_TARGET, Vec3::Y);
    }
}
