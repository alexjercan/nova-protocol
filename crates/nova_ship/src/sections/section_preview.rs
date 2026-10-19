//! How a [`SectionConfig`] becomes preview entities, so the editor's build
//! ship, the parts gallery's tiles and the placement ghost cannot drift apart.
//!
//! Preview entities render and pick but never simulate, and they are inert by
//! CONSTRUCTION: every section kind is built from its `preview_*_section` half,
//! which carries the render mesh and the config the render observers read and
//! leaves out the live state - thrust input and magnitude, turret aim and
//! trigger, torpedo fire input, the controller's `PDController`. The simulation
//! systems all demand one of those, so they match a preview against no query at
//! all.

use avian3d::prelude::Collider;
use bevy::{
    ecs::system::EntityCommands,
    prelude::{ChildOf, Commands, Entity, Name, Transform, Visibility},
};
use nova_gameplay::markers::prelude::SectionMarker;

use crate::prelude::*;

/// Glob-import surface.
pub mod prelude {
    pub use super::{insert_preview_section, spawn_display_skin, PreviewRole};
}

/// What a preview entity IS to the rest of the game.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PreviewRole {
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
/// No input bindings. A section's binds belong to the ship or the document
/// that holds it, and a preview is a picture of a section: a second copy of
/// the binds out here would have to be kept in step across every despawn of
/// the view that held it.
pub fn insert_preview_section(
    entity: &mut EntityCommands,
    section: &SectionConfig,
    role: PreviewRole,
) {
    entity.insert(preview_section(section.base.clone()));
    match &section.kind {
        SectionKind::Hull(hull) => {
            entity.insert(preview_hull_section(hull.clone()));
        }
        SectionKind::Controller(controller) => {
            entity.insert(preview_controller_section(controller.clone()));
        }
        SectionKind::Thruster(thruster) => {
            entity.insert(preview_thruster_section(thruster.clone()));
        }
        SectionKind::Turret(turret) => {
            entity.insert(preview_turret_section(turret.clone()));
        }
        SectionKind::Torpedo(torpedo) => {
            entity.insert(preview_torpedo_section(torpedo.clone()));
        }
        SectionKind::Railgun(railgun) => {
            entity.insert(preview_railgun_section(railgun.clone()));
        }
        SectionKind::Docking(docking) => {
            entity.insert(preview_docking_section(docking.clone()));
        }
        SectionKind::CargoIntake(intake) => {
            entity.insert(preview_cargo_intake_section(intake.clone()));
        }
        SectionKind::Mining(mining) => {
            entity.insert(preview_mining_section(mining.clone()));
        }
    }
    if role == PreviewRole::Display {
        // Dropped rather than never inserted: the preview bundle is one shared
        // recipe, and a display copy is that recipe minus its identity.
        entity.remove::<(SectionMarker, Collider)>();
    }
}

/// Clad a display copy of `sections` under `parent` in the skin a flown ship
/// of them wears, dressed in `style`.
///
/// The same reading, derivation and greeble solve as the spawn's `lay_skin`,
/// so the menu's hull view shows the plates the flown hull wears. Each plate
/// and piece is a picture only: a marker, a pose and a visibility, with no
/// collider, health or [`SectionFixture`]. The style goes on `parent` as a
/// [`ShipStyle`], where the dressing observers' ancestor walk finds it, as
/// they find a flown ship's on its root.
pub fn spawn_display_skin(
    commands: &mut Commands,
    parent: Entity,
    sections: &[DesignSection],
    style: Option<&ShipStyleConfig>,
) {
    commands
        .entity(parent)
        .insert(ShipStyle(style.map(|style| style.id.clone())));
    let placed: Vec<PlacedPart> = sections
        .iter()
        .map(|section| {
            RefitPlacedSection {
                position: section.position,
                rotation: section.rotation,
                config: &section.config,
            }
            .placed_part()
        })
        .collect();
    let (structure, phase, _) = read_structure(&placed);
    let mut plates = derive_skin(&structure);
    let decor = style.map_or_else(Vec::new, |style| {
        solve_decor(&structure, &mut plates, style)
    });
    let laid: Vec<Entity> = plates
        .iter()
        .map(|plate| {
            commands
                .spawn((
                    Name::new("Display Skin Plate"),
                    ShipSkinMarker(plate.shape),
                    Transform::from_translation(plate.cell.as_vec3() + phase)
                        .with_rotation(plate.rotation),
                    Visibility::Inherited,
                    ChildOf(parent),
                ))
                .id()
        })
        .collect();
    if let Some(style) = style {
        for placement in decor {
            commands.spawn((
                Name::new("Display Skin Decor"),
                ShipDecorMarker(style.fixtures[placement.fixture].model.clone()),
                decor_pose(&plates[placement.plate], placement.turns),
                Visibility::Inherited,
                ChildOf(laid[placement.plate]),
            ));
        }
    }
}
