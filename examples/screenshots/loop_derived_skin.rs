//! loop_derived_skin: derive cladding over an authored section ship, turn the
//! result under a fixed photo rig, then strip a visible patch.

#[path = "shared/showcase.rs"]
mod showcase;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "loop_derived_skin")]
#[command(version = "1.0.0")]
#[command(about = "Capture derived ship skin outside the editor")]
struct Cli;

#[cfg(feature = "debug")]
const LOOP_NAME: &str = "news-0110-derived-skin";
#[cfg(feature = "debug")]
const CAMERA_EYE: Vec3 = Vec3::new(4.5, 2.8, 5.8);

#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct Turntable(bool);

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
        app.init_resource::<Turntable>();
        app.add_plugins(skin_script());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_systems(Update, turn_showcase);
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(showcase::section_ship(
        &game_assets,
        &sections,
    )));
}

#[cfg(feature = "debug")]
fn skin_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the bare showcase ship")
        .enter(GameStates::Loading)
        .until(any_entity::<With<SpaceshipRootMarker>>())
        .deadline(30.0)
        .add()
        .step("frame the bare structure")
        .on_enter(frame_showcase)
        .until(elapsed(0.8))
        .add()
        .step("open the derived-skin loop")
        .on_enter(|world| loop_start(world, LOOP_NAME))
        .add()
        .step("hold the bare structure")
        .until(elapsed(1.0))
        .add()
        .step("derive the skin")
        .on_enter(derive_showcase_skin)
        .until(frames(30))
        .add()
        .step("turn the clad ship")
        .on_enter(|world: &mut World| world.resource_mut::<Turntable>().0 = true)
        .until(elapsed(3.0))
        .add()
        .step("stop the turntable")
        .on_enter(|world: &mut World| world.resource_mut::<Turntable>().0 = false)
        .add()
        .step("strip a visible skin patch")
        .on_enter(strip_visible_patch)
        .until(elapsed(1.5))
        .add()
        .step("close the derived-skin loop")
        .on_enter(|world| loop_end(world, LOOP_NAME))
        .until(loop_written(LOOP_NAME))
        .deadline(60.0)
        .add()
}

#[cfg(feature = "debug")]
fn frame_showcase(world: &mut World) {
    hide_hud(world);
    let camera = world
        .query_filtered::<Entity, With<Camera3d>>()
        .iter(world)
        .next()
        .expect("the showcase has a camera");
    world.entity_mut(camera).insert(ScriptedCameraPose {
        position: CAMERA_EYE,
        look_at: Vec3::ZERO,
    });
}

#[cfg(feature = "debug")]
fn derive_showcase_skin(world: &mut World) {
    let root = world
        .query_filtered::<Entity, With<SpaceshipRootMarker>>()
        .iter(world)
        .next()
        .expect("the showcase ship is loaded");
    world
        .entity_mut(root)
        .insert((ShipSkin(true), ShipStyle(Some("industrial".to_string()))));

    // Skin derivation reacts when section geometry becomes available. Reinsert
    // the unchanged authored link points to run that production spawn path over
    // the already-visible bare structure.
    let sections: Vec<(Entity, SectionLinkPoints)> = world
        .query_filtered::<(Entity, &SectionLinkPoints), With<SectionMarker>>()
        .iter(world)
        .map(|(entity, links)| (entity, links.clone()))
        .collect();
    for (entity, links) in sections {
        world.entity_mut(entity).remove::<SectionLinkPoints>();
        world.entity_mut(entity).insert(links);
    }
}

#[cfg(feature = "debug")]
fn turn_showcase(
    time: Res<Time>,
    turntable: Res<Turntable>,
    mut ships: Query<&mut Transform, With<SpaceshipRootMarker>>,
) {
    if !turntable.0 {
        return;
    }
    for mut transform in &mut ships {
        transform.rotate_y(0.42 * time.delta_secs());
    }
}

#[cfg(feature = "debug")]
fn strip_visible_patch(world: &mut World) {
    let eye = CAMERA_EYE.normalize();
    let mut plates: Vec<(Entity, f32)> = world
        .query_filtered::<(Entity, &GlobalTransform), With<ShipSkinMarker>>()
        .iter(world)
        .map(|(entity, transform)| (entity, transform.translation().dot(eye)))
        .collect();
    plates.sort_by(|left, right| right.1.total_cmp(&left.1));
    assert!(
        plates.len() >= 3,
        "the clad showcase must expose a skin patch"
    );
    for (entity, _) in plates.into_iter().take(3) {
        world.despawn(entity);
    }
}
