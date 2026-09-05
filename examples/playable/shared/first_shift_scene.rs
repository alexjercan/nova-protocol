//! Thin launcher for examples that select one production First Shift scene.

use bevy::prelude::*;
use nova_authoring::prelude::*;
use nova_protocol::prelude::*;

/// One preview-only ship pose supplied by an individual example.
#[derive(Clone, Copy)]
pub struct ShipPose {
    pub id: &'static str,
    pub position: Meters3,
    pub rotation: Quat,
}

#[derive(Resource)]
struct Preview {
    scene: FirstShiftScene,
    poses: &'static [ShipPose],
}

/// Run one production scene with example-owned starting poses.
pub fn run(scene: FirstShiftScene, poses: &'static [ShipPose]) -> bevy::app::AppExit {
    let mut app = AppBuilder::new().with_game_plugins(preview_plugin).build();
    app.insert_resource(Preview { scene, poses });
    // Wrapped in the ONE-capture beat rather than the bare preset: an unarmed
    // run walks the identical steps and writes nothing, so this stays the smoke
    // path while `NOVA_CAPTURE` turns any First Shift scene into a still.
    #[cfg(feature = "debug")]
    app.add_plugins(nova_protocol::nova_debug::harness::nova_screenshot(
        nova_protocol::nova_debug::harness::nova_autopilot(),
    ));
    app.run()
}

fn preview_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load);
}

fn load(mut commands: Commands, assets: Res<GameAssets>, preview: Res<Preview>) {
    let mut scenario = first_shift_scene(
        preview.scene,
        assets.cubemap.clone().into(),
        assets.asteroid_texture.clone().into(),
        &CampaignPortraits::from_game_assets(&assets),
    );
    for pose in preview.poses {
        place_ship(&mut scenario, *pose);
    }
    commands.trigger(LoadScenario(scenario));
}

fn place_ship(scenario: &mut ScenarioConfig, pose: ShipPose) {
    for action in scenario
        .events
        .iter_mut()
        .flat_map(|event| event.actions.iter_mut())
    {
        let EventActionConfig::SpawnScenarioObject(object) = action else {
            continue;
        };
        if object.base.id == pose.id {
            object.base.position = pose.position;
            object.base.rotation = pose.rotation;
            return;
        }
    }
    panic!(
        "First Shift scene '{}' did not spawn preview ship '{}'",
        scenario.id, pose.id
    );
}
