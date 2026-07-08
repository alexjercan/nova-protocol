use avian3d::prelude::*;
use bevy::prelude::*;

use crate::prelude::*;

/// Minimum on-screen size (px) of the target reticle. This is its historical fixed
/// size: the reticle grows to match larger targets but never shrinks below this, so
/// small or distant targets still show a clearly visible, clickable marker.
const MIN_RETICLE_PX: f32 = 32.0;

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
                // Starting/minimum size; `update_position_indicator_hud` grows this
                // to match the target's on-screen bounds each frame.
                width: Val::Px(MIN_RETICLE_PX),
                height: Val::Px(MIN_RETICLE_PX),
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
    q_children: Query<&Children>,
    q_aabb: Query<&ColliderAabb>,
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
        let target_entity = *target_entity;

        let Ok(target_transform) = q_transform.get(target_entity) else {
            continue;
        };

        let Ok(center) =
            main_camera.world_to_viewport(main_transform, target_transform.translation())
        else {
            continue;
        };

        // Size the reticle to the target's on-screen extent. Take the target's
        // world bounding-sphere radius (from the union of its collider AABBs),
        // project a point one radius to the camera's right, and measure its pixel
        // distance from the centre - so the reticle grows with apparent size and
        // shrinks with distance. Using the centre plus a side point (rather than
        // projecting all 8 AABB corners) stays robust when the target is close and
        // its far corners fall behind the camera: only the centre must project,
        // which it always does while the target is locked and on-screen. Falls
        // back to the minimum size when the target has no collider AABB yet (e.g.
        // the frame it spawned).
        let mut size = MIN_RETICLE_PX;
        if let Some(aabb) = target_world_aabb(target_entity, &q_children, &q_aabb) {
            let world_radius = aabb.size().length() * 0.5;
            let edge_world = target_transform.translation() + main_transform.right() * world_radius;
            if let Ok(edge) = main_camera.world_to_viewport(main_transform, edge_world) {
                size = (2.0 * center.distance(edge)).max(MIN_RETICLE_PX);
            }
        }

        node.width = Val::Px(size);
        node.height = Val::Px(size);
        // Center the reticle on the target's screen position.
        node.left = Val::Px(center.x - size / 2.0);
        node.top = Val::Px(center.y - size / 2.0);
    }
}

/// Union the world-space [`ColliderAabb`]s of `entity` and all of its descendants
/// into a single bounding box, or `None` if none of them has a collider AABB.
///
/// A lockable body keeps its colliders on child entities (an asteroid's collider
/// node, or a ship's sections), so the whole subtree is walked rather than just the
/// root.
fn target_world_aabb(
    entity: Entity,
    q_children: &Query<&Children>,
    q_aabb: &Query<&ColliderAabb>,
) -> Option<ColliderAabb> {
    let mut acc: Option<ColliderAabb> = None;
    let mut stack = vec![entity];
    while let Some(current) = stack.pop() {
        if let Ok(aabb) = q_aabb.get(current) {
            acc = Some(match acc {
                Some(existing) => existing.merged(*aabb),
                None => *aabb,
            });
        }
        if let Ok(children) = q_children.get(current) {
            stack.extend(children.iter());
        }
    }
    acc
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::SystemState;

    use super::*;

    #[test]
    fn target_world_aabb_unions_child_collider_aabbs() {
        // The reticle sizes to the whole target, whose colliders live on child
        // nodes (asteroid collider node, ship sections). The parent itself has no
        // collider, so the union must come from walking the children.
        let mut world = World::new();
        let child_a = world
            .spawn(ColliderAabb::from_min_max(
                Vec3::new(-1.0, -1.0, -1.0),
                Vec3::ZERO,
            ))
            .id();
        let child_b = world
            .spawn(ColliderAabb::from_min_max(
                Vec3::ZERO,
                Vec3::new(2.0, 3.0, 4.0),
            ))
            .id();
        let parent = world.spawn_empty().add_children(&[child_a, child_b]).id();

        let mut state: SystemState<(Query<&Children>, Query<&ColliderAabb>)> =
            SystemState::new(&mut world);
        let (q_children, q_aabb) = state.get(&world).unwrap();

        let aabb =
            target_world_aabb(parent, &q_children, &q_aabb).expect("subtree has collider AABBs");
        assert_eq!(aabb.min, Vec3::new(-1.0, -1.0, -1.0));
        assert_eq!(aabb.max, Vec3::new(2.0, 3.0, 4.0));
    }

    #[test]
    fn target_world_aabb_is_none_without_colliders() {
        // A target whose colliders are not ready yet (e.g. spawn frame) yields no
        // AABB, so the caller falls back to the minimum reticle size.
        let mut world = World::new();
        let entity = world.spawn_empty().id();

        let mut state: SystemState<(Query<&Children>, Query<&ColliderAabb>)> =
            SystemState::new(&mut world);
        let (q_children, q_aabb) = state.get(&world).unwrap();

        assert!(target_world_aabb(entity, &q_children, &q_aabb).is_none());
    }
}
