use bevy::{platform::collections::HashMap, prelude::*};
use bevy_enhanced_input::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;

use crate::objects::modification::prelude::SectionModification;

pub mod prelude {
    pub use super::{
        spaceship_scenario_object, AIControllerConfig, PlayerControllerConfig, SectionSource,
        SpaceshipConfig, SpaceshipController, SpaceshipPlugin, SpaceshipSectionConfig,
        SpaceshipSectionsConfig, SPACESHIP_TYPE_NAME,
    };
}

pub const SPACESHIP_TYPE_NAME: &str = "spaceship";

#[derive(Component, Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SpaceshipController {
    None,
    Player(PlayerControllerConfig),
    AI(AIControllerConfig),
}

#[derive(Clone, Debug, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PlayerControllerConfig {
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            with = "crate::objects::binding_input::binding_map_serde",
            skip_serializing_if = "HashMap::is_empty"
        )
    )]
    pub input_mapping: HashMap<SectionId, Vec<Binding>>,
    /// Soft manual-speed cap (u/s), inserted as [`FlightSpeedCap`] on the
    /// ship root: the manual burn tapers off approaching it (the starter
    /// scenario's don't-sail-into-the-void guard; playtest 2026-07-12
    /// finding 1). None = unbounded Newtonian burn, the default.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub speed_cap: Option<f32>,
    /// Give this player ship unlimited ammunition: its weapon sections are
    /// built with `ammo_capacity = None`, so no [`SectionAmmo`] is attached and
    /// the guns never run dry (the finite-ammo default is task 20260525-133025).
    /// The first/New Game scenario turns this on so the intro is not gated on
    /// ammo before a reload mechanic exists; `false` (the default) keeps the
    /// authored per-weapon magazines. Player-scoped: enemies are unaffected.
    pub infinite_ammo: bool,
}

#[derive(Clone, Debug, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AIControllerConfig {
    /// Waypoint loop the ship patrols while nothing hostile is in detection
    /// range (world coordinates). Empty = no patrol assignment: the ship
    /// station-keeps instead.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub patrol: Vec<Vec3>,
    /// Scenario id of a gravity-well entity to orbit while nothing hostile
    /// is in detection range. Takes precedence over `patrol` when both are
    /// set (passive fallback: orbit > patrol > idle). None = no orbit
    /// assignment.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub orbit: Option<String>,
    /// Territorial tether radius (world units): combat breaks off beyond
    /// this distance from the patrol centroid (or the spawn position when
    /// there is no route) and the ship returns to its routine. None = the
    /// ship chases freely. See `AILeash`.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub leash: Option<f32>,
}

pub type SectionId = String;

/// Where a ship section's [`SectionConfig`] comes from. Resolved at spawn in
/// `insert_spaceship_sections` (mirrors `AssetRef`'s resolve-at-spawn): an
/// `Inline` config is used as-is; a `Prototype` is looked up by id in the
/// section-prototype catalog ([`GameSections`]). Keeping the compact
/// authored form (the id) in the scenario data is what lets a re-ported ship
/// reference a shared prototype instead of inlining its whole config.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SectionSource {
    /// The full config, authored inline.
    Inline(SectionConfig),
    /// A reference to a catalog prototype by id, resolved against
    /// [`GameSections`] at spawn.
    Prototype(SectionId),
}

#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpaceshipSectionConfig {
    pub id: SectionId,
    pub position: Vec3,
    pub rotation: Quat,
    /// Where the section's config comes from - inline, or a catalog prototype
    /// referenced by id.
    pub source: SectionSource,
    /// Data-only deltas applied to the resolved section at spawn (inserted as
    /// components, applied by observers). Empty by default; authored files may
    /// omit the field.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub modifications: Vec<SectionModification>,
}

#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect)]
pub struct SpaceshipSectionsConfig(pub Vec<SpaceshipSectionConfig>);

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpaceshipConfig {
    pub controller: SpaceshipController,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
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

        // `insert_spaceship_sections` resolves Prototype sources against
        // `GameSections`, so the plugin self-provides an (empty) default: production
        // and the editor overwrite it with the loaded catalog, and Inline-only spawns
        // (examples, previews) then need no catalog wiring. Makes the `Res<GameSections>`
        // dependency self-satisfying instead of a spawn-order footgun.
        app.init_resource::<GameSections>();

        app.add_observer(insert_spaceship_sections);

        // Section modifications: the per-variant components + their apply-on-add
        // observers (DisableVerb / SetHealth / Rename).
        crate::objects::modification::register_section_modifications(app);
    }
}

fn insert_spaceship_sections(
    add: On<Add, SpaceshipRootMarker>,
    mut commands: Commands,
    game_sections: Res<GameSections>,
    q_spaceship: Query<
        (&SpaceshipSectionsConfig, &SpaceshipController, &Transform),
        With<SpaceshipRootMarker>,
    >,
) {
    let entity = add.entity;
    trace!("insert_spaceship_sections: entity {:?}", entity);

    let Ok((sections_config, controller_config, transform)) = q_spaceship.get(entity) else {
        error!(
            "insert_spaceship_sections: entity {:?} not found in q_spaceship",
            entity
        );
        return;
    };
    let spawn_position = transform.translation;

    // A player ship flagged for infinite ammo has its weapons built without a
    // magazine: overriding `ammo_capacity` to None means `insert_turret_section`
    // / `insert_torpedo_section` attach no `SectionAmmo`, which is exactly the
    // unlimited-ammo default. Enemy ships are never flagged, so they keep theirs.
    let infinite_ammo =
        matches!(controller_config, SpaceshipController::Player(config) if config.infinite_ammo);

    commands.entity(entity).with_children(|parent| {
        for section in sections_config.iter() {
            // Resolve the section's source to an owned SectionConfig: an inline
            // config is used as-is; a prototype is looked up in the catalog
            // (missing -> error + skip this section, no panic).
            let config: SectionConfig = match &section.source {
                SectionSource::Inline(config) => config.clone(),
                SectionSource::Prototype(id) => match game_sections.get_section(id) {
                    Some(config) => config.clone(),
                    None => {
                        error!(
                            "insert_spaceship_sections: unknown section prototype '{}' for \
                             section '{}'; skipping",
                            id, section.id
                        );
                        continue;
                    }
                },
            };

            let mut section_entity = parent.spawn((
                EntityId::new(section.id.clone()),
                EntityTypeName::new(config.base.id.clone()),
                base_section(config.base.clone()),
                Transform::from_translation(section.position).with_rotation(section.rotation),
            ));

            match &config.kind {
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
                    let mut turret_config = turret_config.clone();
                    if infinite_ammo {
                        turret_config.ammo_capacity = None;
                    }
                    section_entity.insert(turret_section(turret_config));

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
                    let mut torpedo_config = torpedo_config.clone();
                    if infinite_ammo {
                        torpedo_config.ammo_capacity = None;
                    }
                    section_entity.insert(torpedo_section(torpedo_config));

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

            // Insert the authored modification components; their observers apply
            // each delta where relevant (and are inert elsewhere).
            SectionModification::insert_all(&section.modifications, &mut section_entity);
        }
    });

    match controller_config {
        SpaceshipController::None => {}
        SpaceshipController::Player(config) => {
            commands.entity(entity).insert(PlayerSpaceshipMarker);
            if let Some(cap) = config.speed_cap {
                commands.entity(entity).insert(FlightSpeedCap(cap));
            }
        }
        SpaceshipController::AI(config) => {
            commands.entity(entity).insert(AISpaceshipMarker);
            if !config.patrol.is_empty() {
                commands
                    .entity(entity)
                    .insert(AIPatrolRoute::new(config.patrol.clone()));
            }
            if let Some(well) = &config.orbit {
                commands.entity(entity).insert(AIOrbitDirective {
                    well: EntityId::new(well.clone()),
                });
            }
            if let Some(radius) = config.leash {
                // Anchor on the patrol centroid: the route IS the
                // territory. A routeless ship tethers to where it spawned.
                let center = if config.patrol.is_empty() {
                    spawn_position
                } else {
                    config.patrol.iter().sum::<Vec3>() / config.patrol.len() as f32
                };
                commands.entity(entity).insert(AILeash { center, radius });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The AI controller config maps to the per-entity directive components
    /// exactly: patrol -> AIPatrolRoute, orbit -> AIOrbitDirective, absent
    /// fields insert nothing (task 20260711-212521).
    #[test]
    fn ai_config_maps_to_directive_components() {
        let mut world = World::new();
        // The observer resolves each section's source against the catalog; these
        // tests use Inline sources, so an empty catalog is fine.
        world.init_resource::<GameSections>();
        world.add_observer(insert_spaceship_sections);

        let spawn = |world: &mut World, config: AIControllerConfig| {
            let entity = world
                .spawn((
                    // The observer reads the spawn Transform for the leash
                    // anchor; production ships get one from the base
                    // scenario bundle.
                    Transform::default(),
                    spaceship_scenario_object(SpaceshipConfig {
                        controller: SpaceshipController::AI(config),
                        sections: vec![],
                    }),
                ))
                .id();
            world.flush();
            entity
        };

        let orbiter = spawn(
            &mut world,
            AIControllerConfig {
                orbit: Some("planetoid".to_string()),
                ..default()
            },
        );
        let directive = world.entity(orbiter).get::<AIOrbitDirective>().unwrap();
        assert_eq!(*directive.well, "planetoid");
        assert!(world.entity(orbiter).get::<AIPatrolRoute>().is_none());
        assert!(world.entity(orbiter).contains::<AISpaceshipMarker>());

        let patroller = spawn(
            &mut world,
            AIControllerConfig {
                patrol: vec![Vec3::ZERO, Vec3::X],
                ..default()
            },
        );
        assert!(world.entity(patroller).get::<AIOrbitDirective>().is_none());
        assert!(world.entity(patroller).get::<AIPatrolRoute>().is_some());

        // Both set: both components are inserted - the patrol route is
        // SHADOWED by the orbit's passive precedence (nova_gameplay), not
        // dropped, per the config doc's contract.
        let both = spawn(
            &mut world,
            AIControllerConfig {
                patrol: vec![Vec3::ZERO, Vec3::X],
                orbit: Some("planetoid".to_string()),
                leash: None,
            },
        );
        assert!(world.entity(both).get::<AIOrbitDirective>().is_some());
        assert!(world.entity(both).get::<AIPatrolRoute>().is_some());
    }

    /// A Player ship flagged `infinite_ammo` builds its weapon with no magazine
    /// (`ammo_capacity` None, so `insert_turret_section` attaches no
    /// `SectionAmmo` - that half is covered by the ammo tests); unflagged it
    /// keeps the authored `Some(10)`. Asserting on the section's config helper
    /// is the right boundary for this scenario-side override.
    #[test]
    fn player_infinite_ammo_strips_the_weapon_magazine() {
        fn turret_ammo_capacity(infinite_ammo: bool) -> Option<u32> {
            let mut world = World::new();
            world.init_resource::<GameSections>();
            world.add_observer(insert_spaceship_sections);
            world.spawn((
                Transform::default(),
                spaceship_scenario_object(SpaceshipConfig {
                    controller: SpaceshipController::Player(PlayerControllerConfig {
                        infinite_ammo,
                        ..default()
                    }),
                    sections: vec![SpaceshipSectionConfig {
                        id: "turret".to_string(),
                        position: Vec3::ZERO,
                        rotation: Quat::IDENTITY,
                        source: SectionSource::Inline(SectionConfig {
                            base: BaseSectionConfig {
                                id: "turret".to_string(),
                                ..default()
                            },
                            kind: SectionKind::Turret(TurretSectionConfig {
                                ammo_capacity: Some(10),
                                ..default()
                            }),
                        }),
                        modifications: vec![],
                    }],
                }),
            ));
            world.flush();
            let mut q = world.query::<&TurretSectionConfigHelper>();
            q.iter(&world)
                .next()
                .expect("a turret section was spawned")
                .ammo_capacity
        }

        assert_eq!(
            turret_ammo_capacity(true),
            None,
            "infinite_ammo must strip the weapon magazine"
        );
        assert_eq!(
            turret_ammo_capacity(false),
            Some(10),
            "without the flag the authored magazine is kept"
        );
    }
}
