use std::fmt::Debug;

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_common_systems::prelude::*;

use super::prelude::*;

pub mod prelude {
    pub use super::{
        base_section, BaseSectionConfig, GameSections, SectionConfig, SectionKind, SectionMarker,
        SectionRenderOf,
    };
}

#[derive(Component, Clone, Debug, Reflect)]
pub struct SectionMarker;

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect, PartialEq, Eq)]
pub struct SectionRenderOf(pub Entity);

#[derive(Component, Clone, Debug, Default, Reflect)]
pub struct BaseSectionConfig {
    pub id: String,
    pub name: String,
    pub description: String,
    pub mass: f32,
    pub health: f32,
}

#[derive(Clone, Debug, Reflect)]
#[allow(clippy::large_enum_variant)]
pub enum SectionKind {
    Hull(HullSectionConfig),
    Thruster(ThrusterSectionConfig),
    Controller(ControllerSectionConfig),
    Turret(TurretSectionConfig),
    Torpedo(TorpedoSectionConfig),
}

#[derive(Clone, Debug, Reflect)]
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
        ColliderDensity(config.mass),
        Health::new(config.health),
        ExplodableEntity,
        Visibility::Inherited,
    )
}
