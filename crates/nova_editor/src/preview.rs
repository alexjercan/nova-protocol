//! The one place that knows how a [`SectionConfig`] becomes preview entities,
//! so the build ship, the gallery tiles and the placement ghost cannot drift
//! apart.
//!
//! Preview entities render and pick but never simulate: see
//! [`preview_section`] for what a preview deliberately leaves out.

use bevy::{ecs::system::EntityCommands, prelude::*};
use bevy_enhanced_input::prelude::Binding;
use nova_ship::prelude::*;

/// Turn `entity` into a preview of `section`: the shared preview bundle plus
/// the kind-specific one that renders it.
///
/// `binds` are the input bindings a BUILT section carries; a display-only
/// preview (a gallery tile) passes none and gets an empty binding component,
/// which nothing reads outside the scenario hand-off.
pub(crate) fn insert_preview_section(
    entity: &mut EntityCommands,
    section: &SectionConfig,
    binds: Vec<Binding>,
) {
    entity.insert(preview_section(section.base.clone()));
    match &section.kind {
        SectionKind::Hull(hull) => {
            entity.insert(hull_section(hull.clone()));
        }
        SectionKind::Controller(controller) => {
            entity.insert(preview_controller_section(controller.clone()));
        }
        SectionKind::Thruster(thruster) => {
            entity.insert((
                thruster_section(thruster.clone()),
                SpaceshipThrusterInputBinding(binds),
            ));
        }
        SectionKind::Turret(turret) => {
            entity.insert((
                turret_section(turret.clone()),
                SpaceshipTurretInputBinding(binds),
            ));
        }
        SectionKind::Torpedo(torpedo) => {
            entity.insert((
                torpedo_section(torpedo.clone()),
                SpaceshipTorpedoInputBinding(binds),
            ));
        }
    }
}
