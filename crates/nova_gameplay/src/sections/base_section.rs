use std::fmt::Debug;

use avian3d::prelude::*;
use bevy::prelude::*;

use super::prelude::*;
use crate::prelude::{destructible_body, ExplodableEntity};

pub mod prelude {
    pub use super::{
        base_section, preview_section, BaseSectionConfig, GameSections, SectionConfig,
        SectionInactiveMarker, SectionKind, SectionMarker, SectionRenderOf,
    };
}

#[derive(Component, Clone, Debug, Reflect)]
pub struct SectionMarker;

#[derive(Component, Clone, Debug, Reflect)]
pub struct SectionInactiveMarker;

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect, PartialEq, Eq)]
pub struct SectionRenderOf(pub Entity);

#[derive(Component, Clone, Debug, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BaseSectionConfig {
    pub id: String,
    pub name: String,
    pub description: String,
    pub mass: f32,
    pub health: f32,
}

#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::large_enum_variant)]
pub enum SectionKind {
    Hull(HullSectionConfig),
    Thruster(ThrusterSectionConfig),
    Controller(ControllerSectionConfig),
    Turret(TurretSectionConfig),
    Torpedo(TorpedoSectionConfig),
}

#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SectionConfig {
    pub base: BaseSectionConfig,
    pub kind: SectionKind,
}

#[derive(Resource, Clone, Debug, Deref, DerefMut, Default)]
pub struct GameSections(pub Vec<SectionConfig>);

impl GameSections {
    pub fn get_section(&self, id: &str) -> Option<&SectionConfig> {
        self.iter().find(|section| section.base.id == id)
    }
}

pub fn base_section(config: BaseSectionConfig) -> impl Bundle {
    debug!("base_section: config {:?}", config);

    (
        Name::new(config.name.clone()),
        SectionMarker,
        Collider::cuboid(1.0, 1.0, 1.0),
        destructible_body(config.health, config.mass),
        // bevy_common_systems' destructible_body is the generic Health + density + visibility
        // bundle; nova adds ExplodableEntity so the section enters the explode pipeline.
        ExplodableEntity,
    )
}

/// A lightweight, pickable stand-in for a section, used by the editor to preview a ship
/// configuration without spawning a live combat ship.
///
/// It renders (via the kind-specific `*_section` bundle inserted alongside it) and can be
/// clicked to place adjacent sections, but unlike [`base_section`] it carries no `Health`,
/// `ColliderDensity` or `ExplodableEntity`, so it never enters the integrity/damage
/// pipeline. As long as neither it nor its root has a `RigidBody`, avian keeps its collider
/// in the standalone spatial-query tree (still pickable) and never links it with
/// `ColliderOf`, so no integrity graph is built for the preview ship and none of the
/// gameplay/health systems act on it.
pub fn preview_section(config: BaseSectionConfig) -> impl Bundle {
    debug!("preview_section: config {:?}", config);

    (
        Name::new(config.name.clone()),
        SectionMarker,
        Collider::cuboid(1.0, 1.0, 1.0),
        Visibility::Inherited,
    )
}
