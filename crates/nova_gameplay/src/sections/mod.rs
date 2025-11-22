//! This module contains all the sections of a spaceship.

use bevy::prelude::*;

pub mod base_section;
pub mod controller_section;
pub mod hull_section;
pub mod thruster_section;
pub mod torpedo_section;
pub mod turret_section;

pub mod prelude {
    pub use super::{
        base_section::prelude::*, controller_section::prelude::*, hull_section::prelude::*,
        thruster_section::prelude::*, torpedo_section::prelude::*, turret_section::prelude::*,
        SpaceshipRootMarker, SpaceshipSectionPlugin, SpaceshipSectionSystems,
    };
}

/// This will be the root component for the entire spaceship. All other sections will be children
/// of this entity.
#[derive(Component, Clone, Debug, Default, Reflect)]
pub struct SpaceshipRootMarker;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpaceshipSectionSystems;

/// A plugin that adds all the spaceship sections and their related systems.
#[derive(Default, Clone, Debug)]
pub struct SpaceshipSectionPlugin {
    pub render: bool,
}

impl Plugin for SpaceshipSectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            hull_section::HullSectionPlugin {
                render: self.render,
            },
            thruster_section::ThrusterSectionPlugin {
                render: self.render,
            },
            turret_section::TurretSectionPlugin {
                render: self.render,
            },
            controller_section::ControllerSectionPlugin {
                render: self.render,
            },
            torpedo_section::TorpedoSectionPlugin {
                render: self.render,
            },
        ));
    }
}
