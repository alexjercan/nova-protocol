use bevy::prelude::*;

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
        // Full-screen, click-through UI layer. The reticle is an absolutely-positioned
        // child moved to the target's screen position each frame. This uses the UI pass
        // (like the health/objectives HUDs) instead of a second Camera2d: a second
        // window-targeting camera on Bevy 0.19 blacks out the 3D scene camera, so the
        // crosshair must be plain UI.
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        Pickable::IGNORE,
        children![(
            Name::new("TorpedoTargetUI"),
            TorpedoTargetUIMarker,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(32.0),
                height: Val::Px(32.0),
                // TODO(20260525-133022): Size to fit the object being targeted.
                ..default()
            },
            ImageNode::new(config.target_sprite.clone()),
            Pickable::IGNORE,
            Visibility::Hidden,
        )],
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
    q_target: Query<&TorpedoTargetHudEntity, With<TorpedoTargetHudMarker>>,
    mut q_ui: Query<(&mut Node, &ChildOf), With<TorpedoTargetUIMarker>>,
    q_transform: Query<&GlobalTransform>,
    main_camera: Single<(&GlobalTransform, &Camera), With<SpaceshipCameraController>>,
) {
    let (main_transform, main_camera) = main_camera.into_inner();

    for (mut node, ChildOf(parent)) in &mut q_ui {
        let Ok(target) = q_target.get(*parent) else {
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

        // Center the 32x32 reticle on the target's screen position.
        node.left = Val::Px(coords.x - 16.0);
        node.top = Val::Px(coords.y - 16.0);
    }
}
