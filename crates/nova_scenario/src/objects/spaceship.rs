//! The spaceship scenario object: which hull it spawns, where that hull comes
//! from, and whether the player or the AI flies it.
//!
//! [`SectionSource`] is the seam that lets an authored ship reference the
//! shipped catalog by id or carry its own inline config;
//! [`ShipSource`](crate::objects::ship::prelude::ShipSource) is the same seam
//! one level up, over the whole hull.
//!
//! Touch this module when changing how a ship is SPAWNED. What a ship IS lives
//! in [`ship`](crate::objects::ship).

use std::collections::BTreeMap;

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;
use nova_input::prelude::InputSource;
use nova_ship::prelude::*;

use crate::objects::{
    modification::prelude::SectionModification,
    ship::prelude::{GameShips, ShipHull, ShipSectionModification, ShipSource},
};

/// The spaceship scenario object, its config and section sources, the player and AI controller
/// configs, and `SpaceshipPlugin`.
pub mod prelude {
    pub use super::{
        spaceship_scenario_object, AIControllerConfig, PlayerControllerConfig, SectionId,
        SectionSource, SpaceshipConfig, SpaceshipController, SpaceshipHull, SpaceshipModifications,
        SpaceshipPlugin, SpaceshipSectionConfig,
    };
}

/// Who drives a spaceship scenario object: nobody, the [`PlayerControllerConfig`]
/// player, or an [`AIControllerConfig`] bot. Authored in [`SpaceshipConfig`] and
/// carried on the ship root; `insert_spaceship_sections` reads it at spawn to
/// wire input bindings or AI directives and to tag the player/AI marker.
#[derive(Component, Clone, Debug, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SpaceshipController {
    /// Nobody drives this ship; it station-keeps with no bindings or AI.
    #[default]
    None,
    /// A human player drives this ship, with the given input/config.
    Player(PlayerControllerConfig),
    /// An AI bot drives this ship, with the given patrol/orbit/combat config.
    AI(AIControllerConfig),
}

/// Player-driver settings for a [`SpaceshipController::Player`] ship:
/// per-section input bindings and an optional soft speed cap.
/// Authored in the scenario RON and consumed at spawn by
/// `insert_spaceship_sections`, which inserts the derived components on the ship
/// root (see the per-field docs).
#[derive(Clone, Debug, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PlayerControllerConfig {
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "BTreeMap::is_empty")
    )]
    /// Per-section input bindings: the keys/buttons that drive each thruster,
    /// turret, or torpedo section, keyed by section id. Empty by default.
    ///
    /// [`InputSource`] rather than `bevy_enhanced_input`'s `Binding`: a
    /// section binds a modifier-free button and nothing else, and `Binding`
    /// carries three variants (`AnyKey`, `Custom`, `None`) that mean nothing
    /// in a file. The rig converts at spawn, so upstream's type lives inside
    /// rig construction and nowhere in the data model.
    ///
    /// Ordered, not hashed: this writes `input_mapping:` into the GENERATED
    /// `assets/base/**/*.content.ron`, and a hash-ordered map makes
    /// `content -- gen` produce a different file every run.
    pub input_mapping: BTreeMap<SectionId, Vec<InputSource>>,
    /// Soft manual-speed cap, inserted as [`FlightSpeedCap`] on the
    /// ship root: the manual burn tapers off approaching it (the starter
    /// scenario's don't-sail-into-the-void guard; playtest 2026-07-12
    /// finding 1). None = unbounded Newtonian burn, the default.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub speed_cap: Option<MetersPerSecond>,
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
    pub patrol: Vec<Meters3>,
    /// Scenario id of a gravity-well entity to orbit while nothing hostile
    /// is in detection range. Takes precedence over `patrol` when both are
    /// set (passive fallback: orbit > patrol > idle). None = no orbit
    /// assignment.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub orbit: Option<String>,
    /// Territorial tether radius: combat breaks off beyond
    /// this distance from the patrol centroid (or the spawn position when
    /// there is no route) and the ship returns to its routine. None = the
    /// ship chases freely. See `AILeash`.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub leash: Option<Meters>,
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
    /// Hostile-detection range override: a passive ship leaves its routine
    /// for a hostile inside this range instead of the engine's 4 km default.
    /// Author it wide on a long-watch emplacement that must wake for targets
    /// parked outside everyone else's detection; short on a ship meant to
    /// ignore a nearby brawl. None = the default. See `AIEngageRange`.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub engage_range: Option<Meters>,
    /// Point-defense range override: the guns hold fire until an inbound
    /// hostile torpedo is inside this range instead of the engine's 1.5 km
    /// default. Author it short to stage intercepts close-in; past the
    /// turret's ~1.8 km reach it just wastes the opening shots.
    /// None = the default. See `AIPointDefenseRange`.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub pd_range: Option<Meters>,
    /// Patrol waypoint-arrival slack override on top of the autopilot's
    /// arrival standoff; the engine default is 250 m. Small = the ship
    /// presses in close to each waypoint before turning (a nav drill hugging
    /// its beacons). Below ~20 m risks stalling outside the advance gate -
    /// author small, not zero. None = the default. See
    /// `AIWaypointSlack`.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub waypoint_slack: Option<Meters>,
    /// Whether this armed ship flies itself but never fights: it patrols,
    /// orbits, avoids and station-keeps exactly as any AI ship does, and never
    /// acquires a target or pulls a trigger. See `AINonCombatant`.
    ///
    /// An UNARMED AI ship gets this behavior automatically - a hauler with no
    /// mount cannot fight, so there is nothing to switch off. This field is for
    /// the armed hull that should not: a military escort holding formation
    /// through a scene it takes no part in.
    ///
    /// Not something to emulate with a long `engage_delay` or a tiny
    /// `engage_range`. Both are timers and distances that eventually expire or
    /// are crossed, so the ship opens fire in the middle of a beat that assumed
    /// it would not; this is a standing statement about the hull.
    ///
    /// For a ship the SCENARIO drives shot by shot, use
    /// `SpaceshipController::None` and the scripted helm and weapon actions
    /// instead. The two cover the whole space between them: this one flies
    /// itself and never shoots, that one does exactly and only what it is told.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "is_false"))]
    pub non_combatant: bool,
    /// Translation-arrival standoff override: how far from a GOTO goal this
    /// ship's computer comes to rest, instead of the engine's 500 m default.
    /// Author it small (with a small `waypoint_slack`) on a ship that must
    /// visibly REACH its waypoints. None = the default. See
    /// `FlightArrivalStandoff`.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub arrival_standoff: Option<Meters>,
}

/// `skip_serializing_if` predicate for a `bool` that defaults to false, so an
/// ordinary combatant keeps the field out of its RON entirely.
#[cfg(feature = "serde")]
fn is_false(flag: &bool) -> bool {
    !*flag
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
#[expect(
    clippy::large_enum_variant,
    reason = "spawn-time config, and bevy_reflect 0.19 cannot box the variant"
)]
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
    /// The section's mount cell relative to the ship root, in BUILD-GRID
    /// cells - the one authored vector that is not a distance. A cell is one
    /// engine world unit, 10 m on a side, and sections stack by whole cells.
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

/// The hull a spawned ship flies, carried on the ship root from
/// [`SpaceshipConfig::hull`]. `insert_spaceship_sections` reads it on
/// `Add<SpaceshipRootMarker>`, resolves it against [`GameShips`], and spawns
/// each [`SpaceshipSectionConfig`] as a child section entity.
#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect)]
pub struct SpaceshipHull(pub ShipSource);

/// The per-spawn deltas this ship applies over its resolved hull, carried on
/// the ship root from [`SpaceshipConfig::modifications`].
#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect)]
pub struct SpaceshipModifications(pub Vec<ShipSectionModification>);

/// The scenario/modding RON surface for a spaceship object: WHICH hull it
/// spawns, who drives it, which side it is on, and the deltas this one spawn
/// applies over the shared hull. Passed to `spaceship_scenario_object` to build
/// the ship-root bundle.
///
/// The split is the point: everything reusable lives in the
/// [`ShipHull`](crate::objects::ship::prelude::ShipHull) this names, so eleven
/// scenarios spawning the corvette reference one ship instead of carrying
/// eleven copies of its section list.
#[derive(Clone, Debug, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpaceshipConfig {
    /// The hull: a catalog ship by id, or one authored inline.
    pub hull: ShipSource,
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
    /// Data-only deltas this spawn applies to named sections of the resolved
    /// hull, applied AFTER each section's own list so the spawn wins. Empty by
    /// default; authored files may omit the field.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub modifications: Vec<ShipSectionModification>,
}

/// Build the ship-root bundle from a [`SpaceshipConfig`]: the marker, type
/// name, controller, and the hull reference the `insert_spaceship_sections`
/// observer resolves to spawn the section children and wire the driver at
/// spawn.
///
/// The hull's own components (collapse threshold, skin, style) are inserted by
/// that observer rather than here: a `Prototype` hull is not known until the
/// catalog is read, and the catalog is a resource only a system can see.
pub fn spaceship_scenario_object(config: SpaceshipConfig) -> impl Bundle {
    trace!("spaceship_scenario_object: config {:?}", config);

    (
        SpaceshipRootMarker,
        EntityTypeName::new(SPACESHIP_TYPE_NAME),
        config.controller,
        SpaceshipHull(config.hull),
        SpaceshipModifications(config.modifications),
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

/// Spawns spaceship scenario objects: resolves each ship's hull and section
/// list into child section entities and wires the player/AI controller.
/// Adds the `Add<SpaceshipRootMarker>` section-insert observer, seeds empty
/// [`GameSections`] and [`GameShips`] catalogs, and registers the
/// section-modification components and their apply-on-add observers.
pub struct SpaceshipPlugin;

impl Plugin for SpaceshipPlugin {
    fn build(&self, app: &mut App) {
        trace!("SpaceshipPlugin: build");

        // `insert_spaceship_sections` resolves Prototype sources against
        // `GameSections` and `GameShips`, so the plugin self-provides (empty)
        // defaults: production and the editor overwrite them with the loaded
        // catalogs, and Inline-only spawns (examples, previews) then need no
        // catalog wiring. Makes both resource dependencies self-satisfying
        // instead of a spawn-order footgun.
        app.init_resource::<GameSections>();
        app.init_resource::<GameShips>();

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
    game_ships: Res<GameShips>,
    q_spaceship: Query<
        (
            &SpaceshipHull,
            &SpaceshipModifications,
            &SpaceshipController,
            &Transform,
        ),
        With<SpaceshipRootMarker>,
    >,
) {
    let entity = add.entity;
    trace!("insert_spaceship_sections: entity {:?}", entity);

    let Ok((hull_source, spawn_modifications, controller_config, transform)) =
        q_spaceship.get(entity)
    else {
        // NOT an error: a root with no [`SpaceshipHull`] is a hull somebody
        // built by hand rather than one a scenario authored, and an example or
        // a test is entitled to spawn one. This observer only owns the AUTHORED
        // path. Logging it as an error made `system_section_severing` fail
        // `log_clean` for behaving correctly.
        debug!(
            "insert_spaceship_sections: entity {:?} carries no authored hull, so it is not a scenario ship",
            entity
        );
        return;
    };
    let spawn_position = transform.translation;

    // A prototype naming no catalog ship flies as an empty root (error + empty
    // hull, no panic) - the same log-and-carry-on contract a missing section
    // prototype gets below, one level up.
    let empty = ShipHull::default();
    let hull = match hull_source.resolve(&game_ships) {
        Some(hull) => hull,
        None => {
            error!(
                "insert_spaceship_sections: entity {:?} references unknown ship {:?}; \
                 spawning an empty hull",
                entity, hull_source.0
            );
            &empty
        }
    };

    // The hull's own components. Inserted here rather than in the spawn bundle
    // because a Prototype hull is not known until the catalog is read; they
    // land in the same command flush as the sections below, which is the batch
    // the skin derivation and the integrity graph both key off.
    let collapse_threshold = match hull.collapse_threshold {
        Some(fraction) => StructuralCollapseThreshold::new(fraction),
        None => StructuralCollapseThreshold::default(),
    };
    commands.entity(entity).insert((
        collapse_threshold,
        ShipSkin(hull.skin),
        ShipStyle(hull.style.clone()),
        ShipCollapseSound(hull.collapse_sound.clone()),
    ));

    // An AI ship with no turret or torpedo section cannot fight; it becomes a
    // non-combatant below so it flies its routine and never chases. Tracked
    // through the section loop.
    let mut has_weapon = false;

    commands.entity(entity).with_children(|parent| {
        for section in hull.sections.iter() {
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

            // The last point anything knows this section's KIND. A live section
            // carries its sockets and its collider and nothing that says what
            // sort of part it is, and the derived skin has to know which face a
            // part fires through to leave that one cell of it bare.
            if let Some(exit) = SectionExit::of(&config) {
                section_entity.insert(exit);
            }

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
                    let turret_config = turret_config.clone();
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
                    let torpedo_config = torpedo_config.clone();
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
                SectionKind::Railgun(railgun_config) => {
                    has_weapon = true;
                    let railgun_config = railgun_config.clone();
                    section_entity.insert(railgun_section(railgun_config));

                    match controller_config {
                        SpaceshipController::None => {}
                        SpaceshipController::Player(config) => {
                            if let Some(bindings) = config.input_mapping.get(&section.id) {
                                section_entity
                                    .insert(SpaceshipRailgunInputBinding(bindings.clone()));
                            }
                        }
                        SpaceshipController::AI(_) => {}
                    }
                }
            }

            // Insert the authored modification components; their observers apply
            // each delta where relevant (and are inert elsewhere). The hull's
            // own list first, then this spawn's overrides for the section - a
            // later component insert replaces an earlier one, so the spawn wins.
            let mut modifications = section.modifications.clone();
            for override_ in spawn_modifications.iter() {
                if override_.section == section.id {
                    modifications.extend(override_.modifications.iter().cloned());
                }
            }
            SectionModification::insert_all(&modifications, &mut section_entity);
        }
    });

    match controller_config {
        SpaceshipController::None => {}
        SpaceshipController::Player(config) => {
            commands.entity(entity).insert(PlayerSpaceshipMarker);
            if let Some(cap) = config.speed_cap {
                // Engine boundary: the flight code compares the cap against an
                // avian velocity every tick, so it crosses once, here.
                commands
                    .entity(entity)
                    .insert(FlightSpeedCap(cap.to_engine()));
            }
        }
        SpaceshipController::AI(config) => {
            commands.entity(entity).insert(AISpaceshipMarker);
            // An unarmed AI ship (no turret/torpedo section) cannot fight, so
            // it flies its patrol/orbit/idle routine and never chases - a
            // convoy hauler or civilian escort. It stays targetable by
            // hostiles, so a Player-aligned convoy is still hunted and must be
            // defended.
            //
            // `non_combatant` is the same standing-down, ASKED FOR: an armed
            // hull that flies itself and takes no part in the fight. One
            // component either way, so nothing downstream has to know which of
            // the two reasons applied.
            if !has_weapon || config.non_combatant {
                commands.entity(entity).insert(AINonCombatant);
            }
            if !config.patrol.is_empty() {
                // Engine boundary: the route is steered against avian
                // positions, so the waypoints cross once, at the spawn that
                // authored them. Every AI directive below does the same.
                commands.entity(entity).insert(AIPatrolRoute::new(
                    config
                        .patrol
                        .iter()
                        .map(|point| point.to_engine())
                        .collect(),
                ));
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
                    config
                        .patrol
                        .iter()
                        .map(|point| point.to_engine())
                        .sum::<Vec3>()
                        / config.patrol.len() as f32
                };
                commands.entity(entity).insert(AILeash {
                    center,
                    radius: radius.to_engine(),
                });
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
                if range > Meters::ZERO {
                    commands
                        .entity(entity)
                        .insert(AIEngageRange(range.to_engine()));
                }
            }
            if let Some(range) = config.pd_range {
                if range > Meters::ZERO {
                    commands
                        .entity(entity)
                        .insert(AIPointDefenseRange(range.to_engine()));
                }
            }
            if let Some(slack) = config.waypoint_slack {
                if slack > Meters::ZERO {
                    commands
                        .entity(entity)
                        .insert(AIWaypointSlack(slack.to_engine()));
                }
            }
            if let Some(standoff) = config.arrival_standoff {
                if standoff > Meters::ZERO {
                    commands
                        .entity(entity)
                        .insert(FlightArrivalStandoff(standoff.to_engine()));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::{modification::prelude::SectionHealthOverride, ship::prelude::ShipConfig};

    /// The AI controller config maps to the per-entity directive components
    /// exactly: patrol -> AIPatrolRoute, orbit -> AIOrbitDirective, absent
    /// fields insert nothing.
    #[test]
    fn ai_config_maps_to_directive_components() {
        let mut world = World::new();
        // The observer resolves each source against a catalog; these tests use
        // Inline hulls and sections, so empty catalogs are fine.
        world.init_resource::<GameSections>();
        world.init_resource::<GameShips>();
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
                        ..default()
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
                patrol: vec![Meters3::ZERO, Meters3::new(10.0, 0.0, 0.0)],
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
                patrol: vec![Meters3::ZERO, Meters3::new(10.0, 0.0, 0.0)],
                orbit: Some("planetoid".to_string()),
                leash: None,
                engage_delay: None,
                engage_range: None,
                pd_range: None,
                waypoint_slack: None,
                non_combatant: false,
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
                engage_range: Some(Meters(16_000.0)),
                pd_range: Some(Meters(1_500.0)),
                waypoint_slack: Some(Meters(50.0)),
                arrival_standoff: Some(Meters(100.0)),
                ..default()
            },
        );
        assert_eq!(
            world.entity(watcher).get::<AIEngageRange>().map(|r| r.0),
            Some(1600.0),
            "16 km of detection is 1,600 world units"
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

    #[test]
    fn an_unarmed_ai_ship_is_flagged_non_combatant() {
        let mut world = World::new();
        world.init_resource::<GameSections>();
        world.init_resource::<GameShips>();
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
                        controller,
                        hull: ShipSource::Inline(ShipHull {
                            sections,
                            ..default()
                        }),
                        ..default()
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

        // The asked-for case: armed, and told to stand down anyway.
        let escort = spawn(
            &mut world,
            SpaceshipController::AI(AIControllerConfig {
                non_combatant: true,
                ..default()
            }),
            vec![turret_section()],
        );
        assert!(
            world.entity(escort).contains::<AINonCombatant>(),
            "an armed AI ship authored non_combatant must stand down"
        );
        assert!(
            world.entity(escort).contains::<AISpaceshipMarker>(),
            "and it is still an AI ship: it flies its own routine"
        );
    }

    /// The arrival grace wires from config to component only for positive
    /// delays: Some(5) inserts, Some(0)/None do not.
    #[test]
    fn engage_delay_inserts_the_grace_only_when_positive() {
        let mut world = World::new();
        world.init_resource::<GameSections>();
        world.init_resource::<GameShips>();
        world.add_observer(insert_spaceship_sections);
        let spawn = |world: &mut World, config: AIControllerConfig| {
            let entity = world
                .spawn((
                    Transform::default(),
                    spaceship_scenario_object(SpaceshipConfig {
                        controller: SpaceshipController::AI(config),
                        ..default()
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
        world.init_resource::<GameShips>();
        world.add_observer(insert_spaceship_sections);
        let spawn = |world: &mut World, collapse_threshold| {
            let entity = world
                .spawn((
                    Transform::default(),
                    spaceship_scenario_object(SpaceshipConfig {
                        hull: ShipSource::Inline(ShipHull {
                            collapse_threshold,
                            ..default()
                        }),
                        ..default()
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
    /// an unauthored hull does not serialize the field at all.
    #[cfg(feature = "serde")]
    #[test]
    fn collapse_threshold_ron_parses_defaults_and_stays_unserialized() {
        let authored: SpaceshipConfig =
            ron::from_str(r#"(controller: None, hull: Inline((collapse_threshold: Some(0.1))))"#)
                .expect("the documented syntax parses");
        let ShipSource::Inline(hull) = &authored.hull else {
            panic!("an inline hull");
        };
        assert_eq!(hull.collapse_threshold, Some(0.1));

        let omitted: SpaceshipConfig =
            ron::from_str(r#"(controller: None, hull: Inline(()))"#).expect("omitted field parses");
        let ShipSource::Inline(hull) = &omitted.hull else {
            panic!("an inline hull");
        };
        assert_eq!(hull.collapse_threshold, None);

        let written = ron::to_string(&omitted).expect("a config serializes");
        assert!(
            !written.contains("collapse_threshold"),
            "an unauthored hull must not gain the field on a round trip: {written}"
        );
    }

    /// A ship referenced by id spawns the CATALOG hull - its sections, its
    /// skin, its collapse threshold - and the spawn's own modifications land on
    /// the named section on top of the hull's own. This is the whole point of
    /// the split: eleven scenarios name one corvette and each still gets to
    /// harden its own.
    #[cfg(feature = "serde")]
    #[test]
    fn a_ship_referenced_by_id_spawns_the_catalog_hull_with_spawn_overrides() {
        let mut world = World::new();
        world.init_resource::<GameSections>();
        world.insert_resource(GameShips(vec![ShipConfig {
            id: "corvette".to_string(),
            name: "Corvette".to_string(),
            hull: ShipHull {
                collapse_threshold: Some(0.25),
                sections: vec![SpaceshipSectionConfig {
                    id: "fuselage".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                    source: SectionSource::Inline(SectionConfig {
                        base: BaseSectionConfig {
                            id: "fuselage".to_string(),
                            health: 100.0,
                            ..default()
                        },
                        kind: SectionKind::Hull(HullSectionConfig::default()),
                    }),
                    modifications: vec![SectionModification::SetHealth(200.0)],
                }],
                ..default()
            },
        }]));
        world.add_observer(insert_spaceship_sections);

        let entity = world
            .spawn((
                Transform::default(),
                spaceship_scenario_object(SpaceshipConfig {
                    hull: ShipSource::Prototype("corvette".to_string()),
                    modifications: vec![ShipSectionModification {
                        section: "fuselage".to_string(),
                        modifications: vec![SectionModification::SetHealth(500.0)],
                    }],
                    ..default()
                }),
            ))
            .id();
        world.flush();

        assert_eq!(
            world.entity(entity).get::<StructuralCollapseThreshold>(),
            Some(&StructuralCollapseThreshold(0.25)),
            "the catalog hull's threshold reaches the spawned root"
        );
        let children = world.entity(entity).get::<Children>().expect("sections");
        assert_eq!(children.len(), 1, "the catalog hull's one section spawned");
        assert_eq!(
            world
                .entity(children[0])
                .get::<SectionHealthOverride>()
                .map(|health| health.0),
            Some(500.0),
            "the spawn's override is applied after the hull's own, so it wins"
        );
    }

    /// A hull id nothing authored spawns an EMPTY root rather than panicking -
    /// the same log-and-carry-on contract a missing section prototype gets.
    #[test]
    fn an_unknown_ship_id_spawns_an_empty_hull() {
        let mut world = World::new();
        world.init_resource::<GameSections>();
        world.init_resource::<GameShips>();
        world.add_observer(insert_spaceship_sections);

        let entity = world
            .spawn((
                Transform::default(),
                spaceship_scenario_object(SpaceshipConfig {
                    hull: ShipSource::Prototype("no_such_ship".to_string()),
                    ..default()
                }),
            ))
            .id();
        world.flush();

        assert!(world.entity(entity).get::<Children>().is_none());
        assert!(world
            .entity(entity)
            .contains::<StructuralCollapseThreshold>());
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
            ron::from_str(r#"AI((leash: Some(4000.0)))"#).expect("omitted field parses");
        let SpaceshipController::AI(config) = omitted else {
            panic!("AI variant");
        };
        assert_eq!(config.engage_delay, None);
    }
}
