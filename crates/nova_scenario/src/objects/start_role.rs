//! The START ROLE content kind: one New Game start a player picks.
//!
//! A role names a catalog hull and a kit: the credits and the items the ship
//! starts with, and its charter ([`RoleGoal`]): one finite milestone, a
//! predicate over facts the world save already carries. A met charter ends
//! nothing; the open world plays on. It is authored once per role as a
//! [`StartRole`] and read through [`GameStartRoles`]. The lint
//! (`lint_start_role`) refuses a hull no visible ship design carries, a hull
//! with no cargo intake or with an intake where a berthed ship parks, a hull
//! section above advancement 0, an unknown kit item, a kit the hull's hold
//! cannot carry, and a goal no save can meet or tell apart.
//!
//! New Game's Create and Load insert the [`OpenWorldSession`]: the seed and
//! the role the player picked. The `SpawnStartRole` action reads it and
//! spawns the role's hull, bound by [`start_role_bindings`].

use std::collections::BTreeMap;

use bevy::prelude::*;
use nova_events::prelude::{GeneratedBodyId, Meters3};
use nova_gameplay::prelude::{JobMixType, RoleIconType, ShipInventoryStock};
use nova_input::prelude::InputSource;
use nova_ship::prelude::{SectionKind, ADDED_MINING_KEYS};

use crate::objects::{
    ship_design::prelude::{ResolvedShipDesign, ShipDesignId},
    spaceship::prelude::SectionId,
};

/// Glob-import surface for the start role content kind.
pub mod prelude {
    pub use super::{
        start_role_bindings, GameStartRoles, OpenWorldSession, RoleGoal, RoleGoalType,
        StartBerthMarker, StartKit, StartRole, StartRoleDesignId, StartingStation,
    };
}

/// The world and the start the player asked for on the New Game screen.
///
/// Inserted by the menu's Create and by Load, for a scenario whose world is
/// `Generated((seed: None))`. It stays for the whole run, so a Retry streams
/// the same world again and spawns the same start. A new New Game replaces
/// it.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct OpenWorldSession {
    /// The world seed every sector is generated from.
    pub seed: u32,
    /// The start role the player ship spawns as.
    pub role: StartRoleDesignId,
    /// The allied station the player starts docked at.
    pub start: StartingStation,
}

impl OpenWorldSession {
    /// Whether the generated station `id` is the one the player started
    /// docked at: the one station that posts starter jobs.
    #[must_use]
    pub fn is_starting_station(&self, id: &GeneratedBodyId) -> bool {
        self.start.id == *id
    }
}

/// The station the player starts docked at: the living station nearest the
/// seed's origin whose civilization's seeded stance is Ally.
///
/// World generation picks it once per seed (`starting_station` in
/// `nova_world_base`) and refuses a seed with no such station; the world save
/// carries it so a Load and the jobs board read the same station.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct StartingStation {
    /// The station's generated body id, the id its `EntityId` carries.
    pub id: GeneratedBodyId,
    /// The station's world position, in metres.
    pub position: Meters3,
    /// Where the player ship spawns before it berths: in the station's open
    /// space, clear of its cluster.
    pub spawn: Meters3,
}

/// The fresh player ship that still has to berth at the starting station.
///
/// `SpawnStartRole` inserts it on a ship it spawns fresh, never on one a
/// Load thaws. `nova_world_base` berths and docks the ship once the station
/// stands and removes the marker when the ship is docked.
#[derive(Component, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct StartBerthMarker;

/// The input bindings a start role's hull, or a hull bought at a station,
/// spawns with.
///
/// Every turret takes the primary trigger (left mouse and the right gamepad
/// trigger), every railgun `R` and every torpedo bay `F`. The first mining
/// emitter takes `V` and each later one the next of [`ADDED_MINING_KEYS`],
/// since one press must deploy one emitter. No other section is bound: the
/// flight computer drives the thrusters.
///
/// # Panics
///
/// On a hull with more mining emitters than [`ADDED_MINING_KEYS`]: the
/// spawn refuses two emitters on one key, and a generated hull carries at
/// most two.
pub fn start_role_bindings(hull: &ResolvedShipDesign) -> BTreeMap<SectionId, Vec<InputSource>> {
    let mut emitters = 0;
    hull.sections
        .iter()
        .filter_map(|section| {
            let sources = match section.config.kind {
                SectionKind::Turret(_) => vec![
                    InputSource::Mouse(MouseButton::Left),
                    InputSource::Gamepad(GamepadButton::RightTrigger2),
                ],
                SectionKind::Railgun(_) => vec![InputSource::Keyboard(KeyCode::KeyR)],
                SectionKind::Torpedo(_) => vec![InputSource::Keyboard(KeyCode::KeyF)],
                SectionKind::Mining(_) => {
                    let Some(&key) = ADDED_MINING_KEYS.get(emitters) else {
                        panic!(
                            "start_role_bindings: emitter '{}' is past the {} mining keys",
                            section.id,
                            ADDED_MINING_KEYS.len()
                        );
                    };
                    emitters += 1;
                    vec![InputSource::Keyboard(key)]
                }
                SectionKind::Hull(_)
                | SectionKind::Thruster(_)
                | SectionKind::Controller(_)
                | SectionKind::Docking(_)
                | SectionKind::CargoIntake(_) => return None,
            };
            Some((section.id.clone(), sources))
        })
        .collect()
}

/// The id a start role is referenced by.
///
/// A `DesignId`: an authored catalog identifier, resolved against
/// [`GameStartRoles`]. Serialized as the bare id string, so authored RON
/// writes `id: "industrial"`. An id no loaded role carries resolves to
/// nothing; nothing maps it to another role.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct StartRoleDesignId(String);

impl StartRoleDesignId {
    /// The id as authored.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for StartRoleDesignId {
    fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}

impl std::fmt::Display for StartRoleDesignId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// One New Game start: the hull the player flies, the kit it starts with and
/// its charter.
///
/// Every field is required in RON. A hull no visible ship design carries, a
/// hull with no cargo intake or with an intake where a berthed ship parks, an
/// unknown kit item, a kit heavier than the hull's hold and a goal with a
/// zero count, more rings than posters or an unknown hull are refused at
/// lint, see `lint_start_role`.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct StartRole {
    /// The id the role is picked by.
    pub id: StartRoleDesignId,
    /// The name a picker shows.
    pub name: String,
    /// The shape a picker and the Load list draw beside the name.
    pub icon: RoleIconType,
    /// How the role plays, for a reader.
    pub about: String,
    /// The catalog ship design the player starts in.
    pub hull: ShipDesignId,
    /// What the ship starts with.
    pub kit: StartKit,
    /// Which job kinds the station boards lean to for this start.
    pub jobs: JobMixType,
    /// The role's one finite milestone.
    pub goal: RoleGoal,
    /// Background text for the role.
    pub lore: String,
}

/// One finite milestone: the heading the Map shows and its predicate. Met,
/// it stamps the save and changes nothing else; play goes on.
#[derive(Clone, Debug, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct RoleGoal {
    /// The heading the Map's CHARTER block shows, for a reader.
    pub title: String,
    /// What the save must show.
    pub kind: RoleGoalType,
}

/// The predicates a goal may be. Closed: a role picks one and sets its
/// counts. Every predicate reads facts the world save already carries: the
/// jobs the player handed in, the player's hull, the stations the player
/// owns with what they store, and the escorts in the player's fleet.
///
/// A "ring" is one lattice step of civilizations from the world origin, read
/// off the posting station's cell.
#[derive(Clone, Debug, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RoleGoalType {
    /// Hand in `jobs` Procure or Escort jobs posted by `stations` distinct
    /// stations whose cells span `rings` distinct rings.
    StationNetwork {
        /// How many trade jobs to hand in, above zero.
        jobs: u32,
        /// How many distinct posting stations, above zero.
        stations: u32,
        /// How many distinct rings those stations span, at most `stations`.
        rings: u32,
    },
    /// Hand in `hulls` Neutralize jobs posted from `rings` distinct rings.
    KillList {
        /// How many neutralize jobs to hand in, above zero.
        hulls: u32,
        /// How many distinct rings their posters span, at most `hulls`.
        rings: u32,
    },
    /// Fly a hull whose live sections' recipe value at ask is at least the
    /// catalog design `hull`'s.
    Flagship {
        /// The catalog design that sets the bar.
        hull: ShipDesignId,
    },
    /// Own a station whose storage holds, for every ship part family, the
    /// full build recipe of at least one loaded section of that family: a
    /// yard that can build a whole ship. One station must cover every
    /// family; two half-stocked stations do not add up.
    OwnYard,
    /// Own `stations` stations and fly with `escorts` fleet escorts.
    MiningEmpire {
        /// How many stations to own, above zero.
        stations: u32,
        /// How many escorts to fly with, above zero and at most the fleet's
        /// slots (`FLEET_SIDES`).
        escorts: u32,
    },
    /// Turn a civilization Enemy by the player's own deeds (a civilization
    /// the seed drew Enemy does not count) and neutralize `hulls` intact
    /// hulls of it while it stands Enemy.
    Outlaw {
        /// How many intact hulls to neutralize while the civilization is
        /// Enemy, above zero.
        hulls: u32,
    },
}

/// What a start role's ship starts with.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct StartKit {
    /// The credits the ship starts with.
    pub credits: u32,
    /// The items the ship's hold starts with. Must fit the hull's hold.
    pub stock: ShipInventoryStock,
}

/// The loaded start role catalog: every [`StartRole`] the enabled mods
/// registered, in merge order. Filled by the mod merge (`nova_assets`'s
/// `register_bundles`) and read through
/// [`get_role`](GameStartRoles::get_role).
#[derive(Resource, Clone, Debug, Deref, DerefMut, Default)]
pub struct GameStartRoles(pub Vec<StartRole>);

impl GameStartRoles {
    /// The role `id` names, or `None` if nothing authored it.
    pub fn get_role(&self, id: &StartRoleDesignId) -> Option<&StartRole> {
        self.0.iter().find(|role| role.id == *id)
    }
}

#[cfg(test)]
mod tests {
    use nova_events::prelude::Meters;
    use nova_ship::prelude::{
        BaseSectionConfig, HullSectionConfig, MiningSectionConfig, RailgunSectionConfig,
        SectionConfig, ThrusterSectionConfig, TorpedoSectionConfig, TurretSectionConfig,
    };

    use super::*;
    use crate::objects::ship_design::prelude::ResolvedSection;

    fn section(id: &str, kind: SectionKind) -> ResolvedSection {
        ResolvedSection {
            id: id.to_string(),
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            config: SectionConfig {
                base: BaseSectionConfig {
                    id: id.to_string(),
                    ..default()
                },
                kind,
            },
        }
    }

    /// A start hull binds each weapon class on its own key and each mining
    /// emitter on `V`, and leaves plating and drives unbound.
    #[test]
    fn a_start_hull_binds_turrets_to_the_trigger_railguns_to_r_torpedoes_to_f_and_miners_to_v() {
        let hull = ResolvedShipDesign {
            sections: vec![
                section("plate", SectionKind::Hull(HullSectionConfig::default())),
                section(
                    "drive",
                    SectionKind::Thruster(ThrusterSectionConfig::default()),
                ),
                section("pdc", SectionKind::Turret(TurretSectionConfig::default())),
                section(
                    "spinal",
                    SectionKind::Railgun(RailgunSectionConfig::default()),
                ),
                section("bay", SectionKind::Torpedo(TorpedoSectionConfig::default())),
                section(
                    "beam",
                    SectionKind::Mining(MiningSectionConfig {
                        render_mesh: default(),
                        render_mesh_transform: None,
                        pulse_sound: default(),
                        refusal_sound: default(),
                        door_open_sound: default(),
                        door_close_sound: default(),
                        reach: Meters(100.0),
                        pulse_interval_seconds: 1.0,
                        carve_radius_cells: 1.5,
                    }),
                ),
            ],
            ..default()
        };

        let bindings = start_role_bindings(&hull);

        assert_eq!(
            bindings,
            BTreeMap::from([
                (
                    "pdc".to_string(),
                    vec![
                        InputSource::Mouse(MouseButton::Left),
                        InputSource::Gamepad(GamepadButton::RightTrigger2),
                    ],
                ),
                (
                    "spinal".to_string(),
                    vec![InputSource::Keyboard(KeyCode::KeyR)]
                ),
                (
                    "bay".to_string(),
                    vec![InputSource::Keyboard(KeyCode::KeyF)]
                ),
                (
                    "beam".to_string(),
                    vec![InputSource::Keyboard(KeyCode::KeyV)]
                ),
            ])
        );
    }
}
