use bevy::{platform::collections::HashMap, prelude::*};
use bevy_enhanced_input::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;

pub mod prelude {
    pub use super::{
        spaceship_scenario_object, AIControllerConfig, PlayerControllerConfig, SpaceshipConfig,
        SpaceshipController, SpaceshipPlugin, SpaceshipSectionConfig, SpaceshipSectionsConfig,
        SPACESHIP_TYPE_NAME,
    };
}

pub const SPACESHIP_TYPE_NAME: &str = "spaceship";

#[derive(Component, Clone, Debug, Reflect)]
pub enum SpaceshipController {
    None,
    Player(PlayerControllerConfig),
    AI(AIControllerConfig),
}

#[derive(Clone, Debug, Default, Reflect)]
pub struct PlayerControllerConfig {
    pub input_mapping: HashMap<SectionId, Vec<Binding>>,
}

#[derive(Clone, Debug, Reflect)]
pub struct AIControllerConfig {}

pub type SectionId = String;

#[derive(Clone, Debug, Reflect)]
pub struct SpaceshipSectionConfig {
    pub id: SectionId,
    pub position: Vec3,
    pub rotation: Quat,
    pub config: SectionConfig,
}

#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect)]
pub struct SpaceshipSectionsConfig(pub Vec<SpaceshipSectionConfig>);

#[derive(Clone, Debug)]
pub struct SpaceshipConfig {
    pub controller: SpaceshipController,
    pub sections: Vec<SpaceshipSectionConfig>,
}

pub fn spaceship_scenario_object(config: SpaceshipConfig) -> impl Bundle {
    debug!("spaceship_scenario_object: config {:?}", config);

    (
        SpaceshipRootMarker,
        EntityTypeName::new(SPACESHIP_TYPE_NAME),
        config.controller,
        SpaceshipSectionsConfig(config.sections),
    )
}

pub struct SpaceshipPlugin;

impl Plugin for SpaceshipPlugin {
    fn build(&self, app: &mut App) {
        debug!("SpaceshipPlugin: build");

        app.add_observer(insert_spaceship_sections);
    }
}

fn insert_spaceship_sections(
    add: On<Add, SpaceshipRootMarker>,
    mut commands: Commands,
    q_spaceship: Query<(&SpaceshipSectionsConfig, &SpaceshipController), With<SpaceshipRootMarker>>,
) {
    let entity = add.entity;
    trace!("insert_spaceship_sections: entity {:?}", entity);

    let Ok((sections_config, controller_config)) = q_spaceship.get(entity) else {
        error!(
            "insert_spaceship_sections: entity {:?} not found in q_spaceship",
            entity
        );
        return;
    };

    commands.entity(entity).with_children(|parent| {
        for section in sections_config.iter() {
            let mut section_entity = parent.spawn((
                EntityId::new(section.id.clone()),
                EntityTypeName::new(section.config.base.id.clone()),
                base_section(section.config.base.clone()),
                Transform::from_translation(section.position).with_rotation(section.rotation),
            ));

            match &section.config.kind {
                SectionKind::Hull(hull_config) => {
                    section_entity.insert(hull_section(hull_config.clone()));
                }
                SectionKind::Controller(controller_config) => {
                    section_entity.insert(controller_section(controller_config.clone()));
                }
                SectionKind::Thruster(thruster_config) => {
                    section_entity.insert(thruster_section(thruster_config.clone()));

                    match controller_config {
                        SpaceshipController::None => {}
                        SpaceshipController::Player(config) => {
                            if let Some(bindings) = config.input_mapping.get(&section.id) {
                                section_entity
                                    .insert(SpaceshipThrusterInputBinding(bindings.clone()));
                            };
                        }
                        SpaceshipController::AI(_) => {}
                    }
                }
                SectionKind::Turret(turret_config) => {
                    section_entity.insert(turret_section(turret_config.clone()));

                    match controller_config {
                        SpaceshipController::None => {}
                        SpaceshipController::Player(config) => {
                            if let Some(bindings) = config.input_mapping.get(&section.id) {
                                section_entity
                                    .insert(SpaceshipTurretInputBinding(bindings.clone()));
                            }
                        }
                        SpaceshipController::AI(_) => {}
                    }
                }
                SectionKind::Torpedo(torpedo_config) => {
                    section_entity.insert(torpedo_section(torpedo_config.clone()));

                    match controller_config {
                        SpaceshipController::None => {}
                        SpaceshipController::Player(config) => {
                            if let Some(bindings) = config.input_mapping.get(&section.id) {
                                section_entity
                                    .insert(SpaceshipTorpedoInputBinding(bindings.clone()));
                            }
                        }
                        SpaceshipController::AI(_) => {}
                    }
                }
            }
        }
    });

    match controller_config {
        SpaceshipController::None => {}
        SpaceshipController::Player(_) => {
            commands.entity(entity).insert(PlayerSpaceshipMarker);
        }
        SpaceshipController::AI(_) => {
            commands.entity(entity).insert(AISpaceshipMarker);
        }
    }
}
