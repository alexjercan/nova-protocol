//! The one place that knows how a [`SectionConfig`] becomes preview entities,
//! so the build ship, the gallery tiles and the placement ghost cannot drift
//! apart.
//!
//! Preview entities render and pick but never simulate: see [`preview_section`]
//! for what a preview deliberately leaves out.

use avian3d::prelude::Collider;
use bevy::{ecs::system::EntityCommands, prelude::*};
use bevy_enhanced_input::prelude::Binding;
use nova_gameplay::markers::prelude::SectionMarker;
use nova_ship::prelude::*;

/// What a preview entity IS to the rest of the editor.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PreviewRole {
    /// A section of the ship being built: picked, counted, placed against and
    /// handed to the scenario.
    Section,
    /// Scenery that only has to render - a gallery tile, the placement ghost.
    /// It must NOT read as a section: a display copy in a section query would
    /// be one more part on a ship nobody built, and one more collider in the
    /// pointer's way.
    Display,
}

/// Turn `entity` into a preview of `section`: the shared preview bundle plus
/// the kind-specific one that renders it.
///
/// `binds` are the input bindings a BUILT section carries; a [`Display`]
/// preview passes none and gets an empty binding component, which nothing
/// reads outside the scenario hand-off.
///
/// [`Display`]: PreviewRole::Display
pub(crate) fn insert_preview_section(
    entity: &mut EntityCommands,
    section: &SectionConfig,
    role: PreviewRole,
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
    if role == PreviewRole::Display {
        // Dropped rather than never inserted: the preview bundle is one shared
        // recipe, and a display copy is that recipe minus its identity.
        entity.remove::<(SectionMarker, Collider)>();
    }
}
