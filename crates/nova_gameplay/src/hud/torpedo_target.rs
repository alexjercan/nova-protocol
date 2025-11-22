use bevy::{camera::visibility::RenderLayers, prelude::*};

use crate::prelude::*;

pub mod prelude {
    pub use super::{
        torpedo_target_hud, TorpedoTargetHudConfig, TorpedoTargetHudEntity, TorpedoTargetHudMarker,
        TorpedoTargetHudPlugin,
    };
}

#[derive(Component, Debug, Clone, Reflect)]
pub struct TorpedoTargetHudMarker;

#[derive(Component, Debug, Clone, Reflect)]
struct TorpedoTargetCameraHudMarker;

#[derive(Component, Debug, Clone, Reflect)]
struct TorpedoTargetUIMarker;

#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct TorpedoTargetHudEntity(pub Option<Entity>);

#[derive(Clone, Debug, Default)]
pub struct TorpedoTargetHudConfig {
    pub target_sprite: Handle<Image>,
}

pub fn torpedo_target_hud(config: TorpedoTargetHudConfig) -> impl Bundle {
    debug!("torpedo_target_hud: config {:?}", config);

    (
        Name::new("TorpedoTargetHUD"),
        TorpedoTargetHudMarker,
        TorpedoTargetHudEntity(None),
        TorpedoTargetCameraHudMarker,
        Camera2d,
        Camera {
            order: 1,
            // Don't draw anything in the background, to see the previous camera.
            clear_color: ClearColorConfig::Custom(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            ..default()
        },
        // This camera will only render entities which are on the same render layer.
        RenderLayers::layer(1),
        Visibility::Visible,
        children![(
            Name::new("TorpedoTargetUI"),
            TorpedoTargetUIMarker,
            Sprite {
                image: config.target_sprite.clone(),
                // TODO: Size to fit the object being targeted.
                custom_size: Some(Vec2::new(32.0, 32.0)),
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, 0.0),
            RenderLayers::layer(1),
            Visibility::Hidden,
        ),],
    )
}

#[derive(Default)]
pub struct TorpedoTargetHudPlugin;

impl Plugin for TorpedoTargetHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                update_target_system,
                update_ui_target_system,
                update_position_indicator_hud,
            )
                .in_set(super::NovaHudSystems),
        );
    }
}

fn update_target_system(
    mut q_target: Query<&mut TorpedoTargetHudEntity, With<TorpedoTargetHudMarker>>,
    res_target: Res<SpaceshipPlayerTorpedoTargetEntity>,
) {
    for mut target in &mut q_target {
        **target = **res_target;
    }
}

fn update_ui_target_system(
    q_target: Query<&TorpedoTargetHudEntity, With<TorpedoTargetHudMarker>>,
    mut q_ui: Query<(&mut Visibility, &ChildOf), With<TorpedoTargetUIMarker>>,
) {
    for (mut visibility, ChildOf(parent)) in &mut q_ui {
        let Ok(target) = q_target.get(*parent) else {
            continue;
        };

        match &**target {
            Some(_target) => {
                *visibility = Visibility::Visible;
            }
            None => {
                *visibility = Visibility::Hidden;
            }
        }
    }
}

fn update_position_indicator_hud(
    q_target: Query<
        (&Camera, &GlobalTransform, &TorpedoTargetHudEntity),
        With<TorpedoTargetHudMarker>,
    >,
    mut q_ui: Query<(&mut Transform, &ChildOf), With<TorpedoTargetUIMarker>>,
    q_transform: Query<&GlobalTransform>,
    main_camera: Single<(&GlobalTransform, &Camera), With<SpaceshipCameraController>>,
) {
    let (main_transform, main_camera) = main_camera.into_inner();

    for (mut ui_transform, ChildOf(parent)) in &mut q_ui {
        let Ok((camera, camera_transform, target)) = q_target.get(*parent) else {
            continue;
        };

        let Some(target_entity) = &**target else {
            continue;
        };

        let Ok(target_transform) = q_transform.get(*target_entity) else {
            continue;
        };

        let Ok(coords) =
            main_camera.world_to_viewport(main_transform, target_transform.translation())
        else {
            continue;
        };

        let Ok(hud_pos) = camera.viewport_to_world_2d(camera_transform, coords) else {
            continue;
        };

        ui_transform.translation = hud_pos.extend(0.0);
    }
}
