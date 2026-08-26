//! Where a section of the edited ship is on screen.

use bevy::prelude::*;
use nova_protocol::prelude::*;

/// The viewport point the first visible section projects to - where a click
/// reaches the part itself, and the world anchor a label is placed against.
///
/// VISIBLE is what scopes this to the ship being built: the document opens
/// seeded with a stock range whose hulks are ship nodes with sections of their
/// own, and entering a ship takes those off the stage.
pub fn a_section_on_screen(world: &mut World) -> Option<Vec2> {
    let mut q_sections =
        world.query_filtered::<(&GlobalTransform, &InheritedVisibility), With<SectionMarker>>();
    let at = q_sections
        .iter(world)
        .filter(|(_, visible)| visible.get())
        .map(|(pose, _)| pose.translation())
        .next()?;
    let camera_entity = world
        .query_filtered::<Entity, With<Camera3d>>()
        .iter(world)
        .next()?;
    let camera = world.get::<Camera>(camera_entity)?;
    let camera_pose = world.get::<GlobalTransform>(camera_entity)?;
    camera.world_to_viewport(camera_pose, at).ok()
}
