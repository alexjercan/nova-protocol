//! The spaceship scenario object: its section list, where those sections come
//! from, and whether the player or the AI flies it.
//!
//! [`SectionSource`] is the seam that lets an authored ship reference the
//! shipped catalog by id or carry its own inline config.
//!
//! Touch this module when changing how an authored ship is described or
//! assembled.

use avian3d::prelude::*;
use bevy::{platform::collections::HashMap, prelude::*};
use bevy_enhanced_input::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;

use crate::objects::modification::prelude::SectionModification;

/// The spaceship scenario object, its config and section sources, the player and AI controller
/// configs, and `SpaceshipPlugin`.
pub mod prelude {
    pub use super::{
        spaceship_scenario_object, AIControllerConfig, PlayerControllerConfig, SectionSource,
        SpaceshipConfig, SpaceshipController, SpaceshipPlugin, SpaceshipSectionConfig,
        SpaceshipSectionsConfig, SPACESHIP_TYPE_NAME,
    };
}

/// The scenario/modding RON type name for a spaceship object.
pub const SPACESHIP_TYPE_NAME: &str = "spaceship";

/// Who drives a spaceship scenario object: nobody, the [`PlayerControllerConfig`]
/// player, or an [`AIControllerConfig`] bot. Authored in [`SpaceshipConfig`] and
/// carried on the ship root; `insert_spaceship_sections` reads it at spawn to
/// wire input bindings or AI directives and to tag the player/AI marker.
#[derive(Component, Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SpaceshipController {
    /// Nobody drives this ship; it station-keeps with no bindings or AI.
    None,
    /// A human player drives this ship, with the given input/config.
    Player(PlayerControllerConfig),
    /// An AI bot drives this ship, with the given patrol/orbit/combat config.
    AI(AIControllerConfig),
}

/// Player-driver settings for a [`SpaceshipController::Player`] ship: per-section
/// input bindings, an optional soft speed cap, and an infinite-ammo flag.
/// Authored in the scenario RON and consumed at spawn by
/// `insert_spaceship_sections`, which inserts the derived components on the ship
/// root (see the per-field docs).
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
    /// Per-section input bindings: the keys/buttons that drive each thruster,
    /// turret, or torpedo section, keyed by section id. Empty by default.
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
    /// the guns never run dry. The first/New Game scenario turns this on so the
    /// intro is not gated on ammo before a reload mechanic exists; `false` (the
    /// default) keeps the authored per-weapon magazines. Player-scoped: enemies
    /// are unaffected.
    pub infinite_ammo: bool,
}

/// AI-driver settings for a [`SpaceshipController::AI`] ship: its passive
/// routine (patrol or orbit), territorial leash, and arrival grace. Authored
/// in the scenario RON and consumed at spawn by
/// `insert_spaceship_sections`, which inserts the derived directive components
/// on the ship root (see the per-field docs).
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
    /// Arrival grace: the ship spawns on its passive routine and refuses to
    /// engage until this elapses - pair with a warning story beat so enemies
    /// ARRIVE instead of appearing hot. Being shot ends the grace immediately
    /// and permanently. Strict RON: `engage_delay: Some(8.0)`; omitted or
    /// non-positive values mean no grace.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub engage_delay: Option<f32>,
    /// Hostile-detection range override (world units): a passive ship leaves
    /// its routine for a hostile inside this range instead of the engine's
    /// 400 u default. Author it wide on a long-watch emplacement that must
    /// wake for targets parked outside everyone else's detection; short on a
    /// ship meant to ignore a nearby brawl. None = the default. See
    /// `AIEngageRange`.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub engage_range: Option<f32>,
    /// Point-defense range override (world units): the guns hold fire until
    /// an inbound hostile torpedo is inside this range instead of the
    /// engine's 150 u default. Author it short to stage intercepts close-in;
    /// past the turret's ~180 u reach it just wastes the opening shots.
    /// None = the default. See `AIPointDefenseRange`.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub pd_range: Option<f32>,
    /// Patrol waypoint-arrival slack override (world units) on top of the
    /// autopilot's arrival standoff; the engine default is 25. Small = the
    /// ship presses in close to each waypoint before turning (a nav drill
    /// hugging its beacons). Below ~2 risks stalling outside the advance
    /// gate - author small, not zero. None = the default. See
    /// `AIWaypointSlack`.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub waypoint_slack: Option<f32>,
    /// Translation-arrival standoff override (world units): how far from a
    /// GOTO goal this ship's computer comes to rest, instead of the engine's
    /// 50 u default. Author it small (with a small `waypoint_slack`) on a
    /// ship that must visibly REACH its waypoints. None = the default. See
    /// `FlightArrivalStandoff`.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub arrival_standoff: Option<f32>,
}

/// A ship section's scenario-local id, used to key input bindings and address
/// the section from scenario scripts.
pub type SectionId = String;

/// Where a ship section's [`SectionConfig`] comes from. Resolved at spawn in
/// `insert_spaceship_sections` (mirrors `AssetRef`'s resolve-at-spawn): an
/// `Inline` config is used as-is; a `Prototype` is looked up by id in the
/// section-prototype catalog ([`GameSections`]). Keeping the compact
/// authored form (the id) in the scenario data is what lets a re-ported ship
/// reference a shared prototype instead of inlining its whole config.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
// Inline carries the full SectionConfig - hundreds of bytes (528 at the time
// of clippy's report) - next to Prototype's small id; boxing it (clippy's
// suggestion) cannot compile here because the enum derives Reflect and
// bevy_reflect 0.19 has no Reflect impl for Box<T>. This is spawn-time
// config data, not per-frame state, so the size stays.
#[allow(clippy::large_enum_variant)]
pub enum SectionSource {
    /// The full config, authored inline.
    Inline(SectionConfig),
    /// A reference to a catalog prototype by id, resolved against
    /// [`GameSections`] at spawn.
    Prototype(SectionId),
}

/// One entry in a ship's authored section list: where a section sits on the
/// hull, where its config comes from, and any spawn-time modifications.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpaceshipSectionConfig {
    /// The section's scenario-local id (keys input bindings and scripts).
    pub id: SectionId,
    /// The section's position relative to the ship root (world units).
    pub position: Vec3,
    /// The section's rotation relative to the ship root.
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

/// The ship's authored section list, carried on the ship root from
/// [`SpaceshipConfig::sections`]. `insert_spaceship_sections` reads it on
/// `Add<SpaceshipRootMarker>` to spawn each [`SpaceshipSectionConfig`] as a
/// child section entity.
#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect)]
pub struct SpaceshipSectionsConfig(pub Vec<SpaceshipSectionConfig>);

/// The scenario/modding RON surface for a spaceship object: its
/// [`SpaceshipController`], optional [`Allegiance`] override, structural
/// collapse threshold, and section list. Passed to `spaceship_scenario_object`
/// to build the ship-root bundle.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpaceshipConfig {
    /// Who drives the ship: nobody, a player, or an AI bot.
    pub controller: SpaceshipController,
    /// Which side the ship fights for. `None` (the authored default - omit
    /// the field) keeps the controller marker's requirement default: Player
    /// ships read `Allegiance::Player`, AI ships `Allegiance::Enemy`.
    /// `Some(..)` overrides it - the authorable surface for NEUTRAL
    /// bystanders (a drifting hauler the AI must not shoot) or scripted
    /// exceptions. In strict RON the `Option` keeps its variant:
    /// `allegiance: Some(Neutral)`.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub allegiance: Option<Allegiance>,
    /// Structural collapse: the fraction of the hull the ship was BUILT with
    /// (its pinned maximum health) below which what is left comes apart and
    /// the whole ship is destroyed. `None` (the authored default - omit the
    /// field) uses [`DEFAULT_STRUCTURAL_COLLAPSE_THRESHOLD`]. Lower means the
    /// ship must be dismantled further before it goes, which is how a capital
    /// takes more killing than a fighter; `Some(0.0)` is "strip every last
    /// section". In strict RON the `Option` keeps its variant:
    /// `collapse_threshold: Some(0.1)`.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub collapse_threshold: Option<f32>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    /// The ship's sections (hull, thrusters, weapons, controller) and their
    /// placement. Empty by default; each is spawned as a child at load.
    pub sections: Vec<SpaceshipSectionConfig>,
    /// Whether the ship wears a DERIVED skin: cladding computed from the
    /// structure above at spawn, with nothing authored and nothing saved. See
    /// [`ShipSkin`].
    ///
    /// `false` by default, and off for every shipped ship: the derivation reads
    /// a hull as unit cells, which the catalog's cube sections are and the
    /// modelled semantic parts are not.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "is_false"))]
    pub skin: bool,
}

/// `skip_serializing_if` predicate for a `bool` that defaults to false, so an
/// unclad ship keeps the field out of its RON entirely.
#[cfg(feature = "serde")]
fn is_false(flag: &bool) -> bool {
    !*flag
}

/// Build the ship-root bundle from a [`SpaceshipConfig`]: the marker, type name,
/// controller, collapse threshold, and section list the `insert_spaceship_sections`
/// observer reads to spawn the section children and wire the driver at spawn.
pub fn spaceship_scenario_object(config: SpaceshipConfig) -> impl Bundle {
    debug!("spaceship_scenario_object: config {:?}", config);

    let collapse_threshold = match config.collapse_threshold {
        Some(fraction) => StructuralCollapseThreshold::new(fraction),
        None => StructuralCollapseThreshold::default(),
    };

    (
        SpaceshipRootMarker,
        EntityTypeName::new(SPACESHIP_TYPE_NAME),
        config.controller,
        SpaceshipSectionsConfig(config.sections),
        collapse_threshold,
        ShipSkin(config.skin),
        RigidBody::Dynamic,
        // Physics advances Transform only on fixed ticks (64 Hz by default);
        // everything watched by the render-rate camera must interpolate between
        // them or it stair-steps. Invisible while the chase camera was bolted
        // rigidly to the ship (both stepped together), but the camera smoothing
        // from the flight-feel retune eases at render rate and exposed the
        // steps as twitch.
        TransformInterpolation,
    )
}

/// Spawns spaceship scenario objects: resolves each ship's section list into
/// child section entities and wires the player/AI controller.
/// Adds the `Add<SpaceshipRootMarker>` section-insert observer, seeds an empty
/// [`GameSections`] prototype catalog, and registers the section-modification
/// components and their apply-on-add observers.
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

    // An AI ship with no turret or torpedo section cannot fight; it becomes a
    // non-combatant below so it flies its routine and never chases. Tracked
    // through the section loop.
    let mut has_weapon = false;

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
                    has_weapon = true;
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
                    has_weapon = true;
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
            // An unarmed AI ship (no turret/torpedo section) cannot fight, so
            // it flies its patrol/orbit/idle routine and never chases - a
            // convoy hauler or civilian escort. It stays targetable by
            // hostiles, so a Player-aligned convoy is still hunted and must be
            // defended.
            if !has_weapon {
                commands.entity(entity).insert(AINonCombatant);
            }
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
            // Non-positive delays are "no grace" (documented on the field):
            // a zero timer would be born finished anyway, so the guard just
            // keeps the component off ships that never asked for one.
            if let Some(delay) = config.engage_delay {
                if delay > 0.0 {
                    commands.entity(entity).insert(AIEngageGrace::new(delay));
                }
            }
            // Same guard shape: a non-positive range would make the ship
            // blind, which no author means; the default range needs no
            // component at all.
            if let Some(range) = config.engage_range {
                if range > 0.0 {
                    commands.entity(entity).insert(AIEngageRange(range));
                }
            }
            if let Some(range) = config.pd_range {
                if range > 0.0 {
                    commands.entity(entity).insert(AIPointDefenseRange(range));
                }
            }
            if let Some(slack) = config.waypoint_slack {
                if slack > 0.0 {
                    commands.entity(entity).insert(AIWaypointSlack(slack));
                }
            }
            if let Some(standoff) = config.arrival_standoff {
                if standoff > 0.0 {
                    commands
                        .entity(entity)
                        .insert(FlightArrivalStandoff(standoff));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The AI controller config maps to the per-entity directive components
    /// exactly: patrol -> AIPatrolRoute, orbit -> AIOrbitDirective, absent
    /// fields insert nothing.
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
                        collapse_threshold: None,
                        skin: false,
                        allegiance: None,
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
                engage_delay: None,
                engage_range: None,
                pd_range: None,
                waypoint_slack: None,
                arrival_standoff: None,
            },
        );
        assert!(world.entity(both).get::<AIOrbitDirective>().is_some());
        assert!(world.entity(both).get::<AIPatrolRoute>().is_some());

        // The detection and point-defense overrides map on; the default
        // inserts nothing (the orbiter above authored none).
        let watcher = spawn(
            &mut world,
            AIControllerConfig {
                engage_range: Some(1600.0),
                pd_range: Some(150.0),
                waypoint_slack: Some(5.0),
                arrival_standoff: Some(10.0),
                ..default()
            },
        );
        assert_eq!(
            world.entity(watcher).get::<AIEngageRange>().map(|r| r.0),
            Some(1600.0)
        );
        assert_eq!(
            world
                .entity(watcher)
                .get::<AIPointDefenseRange>()
                .map(|r| r.0),
            Some(150.0)
        );
        assert_eq!(
            world.entity(watcher).get::<AIWaypointSlack>().map(|s| s.0),
            Some(5.0)
        );
        assert_eq!(
            world
                .entity(watcher)
                .get::<FlightArrivalStandoff>()
                .map(|s| **s),
            Some(10.0)
        );
        assert!(world.entity(orbiter).get::<AIEngageRange>().is_none());
        assert!(world.entity(orbiter).get::<AIPointDefenseRange>().is_none());
        assert!(world.entity(orbiter).get::<AIWaypointSlack>().is_none());
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
                    collapse_threshold: None,
                    skin: false,
                    allegiance: None,
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

    /// An AI ship with no turret/torpedo section is tagged `AINonCombatant` at
    /// spawn, so it flies its routine and never chases; an armed AI ship is
    /// not. Non-AI ships never get the tag regardless.
    #[test]
    fn an_unarmed_ai_ship_is_flagged_non_combatant() {
        let mut world = World::new();
        world.init_resource::<GameSections>();
        world.add_observer(insert_spaceship_sections);

        let turret_section = || SpaceshipSectionConfig {
            id: "turret".to_string(),
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            source: SectionSource::Inline(SectionConfig {
                base: BaseSectionConfig {
                    id: "turret".to_string(),
                    ..default()
                },
                kind: SectionKind::Turret(TurretSectionConfig::default()),
            }),
            modifications: vec![],
        };
        let spawn = |world: &mut World, controller, sections| {
            let entity = world
                .spawn((
                    Transform::default(),
                    spaceship_scenario_object(SpaceshipConfig {
                        collapse_threshold: None,
                        skin: false,
                        allegiance: None,
                        controller,
                        sections,
                    }),
                ))
                .id();
            world.flush();
            entity
        };

        let unarmed = spawn(
            &mut world,
            SpaceshipController::AI(AIControllerConfig::default()),
            vec![],
        );
        assert!(
            world.entity(unarmed).contains::<AINonCombatant>(),
            "an unarmed AI ship must be a non-combatant"
        );

        let armed = spawn(
            &mut world,
            SpaceshipController::AI(AIControllerConfig::default()),
            vec![turret_section()],
        );
        assert!(
            !world.entity(armed).contains::<AINonCombatant>(),
            "an armed AI ship must NOT be a non-combatant"
        );

        // A player ship, unarmed, is not an AI ship at all - no tag.
        let player = spawn(
            &mut world,
            SpaceshipController::Player(PlayerControllerConfig::default()),
            vec![],
        );
        assert!(!world.entity(player).contains::<AINonCombatant>());
    }

    /// The arrival grace wires from config to component only for positive
    /// delays: Some(5) inserts, Some(0)/None do not.
    #[test]
    fn engage_delay_inserts_the_grace_only_when_positive() {
        let mut world = World::new();
        world.init_resource::<GameSections>();
        world.add_observer(insert_spaceship_sections);
        let spawn = |world: &mut World, config: AIControllerConfig| {
            let entity = world
                .spawn((
                    Transform::default(),
                    spaceship_scenario_object(SpaceshipConfig {
                        collapse_threshold: None,
                        skin: false,
                        controller: SpaceshipController::AI(config),
                        allegiance: None,
                        sections: vec![],
                    }),
                ))
                .id();
            world.flush();
            entity
        };

        let graced = spawn(
            &mut world,
            AIControllerConfig {
                engage_delay: Some(5.0),
                ..default()
            },
        );
        let grace = world.entity(graced).get::<AIEngageGrace>().unwrap();
        assert!((grace.timer.duration() - 5.0).abs() < f32::EPSILON);

        let zero = spawn(
            &mut world,
            AIControllerConfig {
                engage_delay: Some(0.0),
                ..default()
            },
        );
        assert!(
            world.entity(zero).get::<AIEngageGrace>().is_none(),
            "non-positive delays mean no grace"
        );

        let none = spawn(&mut world, AIControllerConfig::default());
        assert!(world.entity(none).get::<AIEngageGrace>().is_none());
    }

    /// The collapse threshold reaches the ship root: authored as given,
    /// unauthored as the engine default, out of range clamped.
    #[test]
    fn the_collapse_threshold_is_authored_per_ship() {
        let mut world = World::new();
        world.init_resource::<GameSections>();
        world.add_observer(insert_spaceship_sections);
        let spawn = |world: &mut World, collapse_threshold| {
            let entity = world
                .spawn((
                    Transform::default(),
                    spaceship_scenario_object(SpaceshipConfig {
                        collapse_threshold,
                        skin: false,
                        allegiance: None,
                        controller: SpaceshipController::None,
                        sections: vec![],
                    }),
                ))
                .id();
            world.flush();
            world
                .entity(entity)
                .get::<StructuralCollapseThreshold>()
                .copied()
        };

        assert_eq!(
            spawn(&mut world, None),
            Some(StructuralCollapseThreshold(
                DEFAULT_STRUCTURAL_COLLAPSE_THRESHOLD
            )),
            "an unauthored ship collapses at the engine default"
        );
        assert_eq!(
            spawn(&mut world, Some(0.1)),
            Some(StructuralCollapseThreshold(0.1)),
            "a capital authored to be taken apart further keeps its number"
        );
        // A threshold below zero is unreachable even at zero health, which
        // would restore the 0-HP ghost; the clamp is what makes it safe to
        // author.
        assert_eq!(
            spawn(&mut world, Some(-1.0)),
            Some(StructuralCollapseThreshold(0.0))
        );
    }

    /// The documented strict-RON syntax parses, omitted defaults to None, and
    /// an unauthored ship does not serialize the field at all.
    #[cfg(feature = "serde")]
    #[test]
    fn collapse_threshold_ron_parses_defaults_and_stays_unserialized() {
        let authored: SpaceshipConfig =
            ron::from_str(r#"(controller: None, collapse_threshold: Some(0.1))"#)
                .expect("the documented syntax parses");
        assert_eq!(authored.collapse_threshold, Some(0.1));

        let omitted: SpaceshipConfig =
            ron::from_str(r#"(controller: None)"#).expect("omitted field parses");
        assert_eq!(omitted.collapse_threshold, None);

        let written = ron::to_string(&omitted).expect("a config serializes");
        assert!(
            !written.contains("collapse_threshold"),
            "an unauthored ship must not gain the field on a round trip: {written}"
        );
    }

    /// The documented strict-RON syntax parses, omitted defaults to None.
    #[cfg(feature = "serde")]
    #[test]
    fn engage_delay_ron_parses_and_defaults() {
        let authored: SpaceshipController =
            ron::from_str(r#"AI((patrol: [(0.0, 0.0, 0.0)], engage_delay: Some(6.0)))"#)
                .expect("the documented syntax parses");
        let SpaceshipController::AI(config) = authored else {
            panic!("AI variant");
        };
        assert_eq!(config.engage_delay, Some(6.0));

        let omitted: SpaceshipController =
            ron::from_str(r#"AI((leash: Some(400.0)))"#).expect("omitted field parses");
        let SpaceshipController::AI(config) = omitted else {
            panic!("AI variant");
        };
        assert_eq!(config.engage_delay, None);
    }
}
