//! The START ROLE content kind: one New Game start a player picks.
//!
//! A role names a catalog hull and a kit: the credits and the items the ship
//! starts with. It is authored once per role as a [`StartRole`] and read
//! through [`GameStartRoles`]. The lint (`lint_start_role`) refuses a hull no
//! visible ship design carries, an unknown kit item and a kit the hull's hold
//! cannot carry.
//!
//! New Game's Create and Load insert the [`OpenWorldSession`]: the seed and
//! the role the player picked. The `SpawnStartRole` action reads it and
//! spawns the role's hull, bound by [`start_role_bindings`].

use std::collections::BTreeMap;

use bevy::prelude::*;
use nova_gameplay::prelude::ShipInventoryStock;
use nova_input::prelude::InputSource;
use nova_ship::prelude::SectionKind;

use crate::objects::{
    ship_design::prelude::{ResolvedShipDesign, ShipDesignId},
    spaceship::prelude::SectionId,
};

/// Glob-import surface for the start role content kind.
pub mod prelude {
    pub use super::{
        start_role_bindings, GameStartRoles, OpenWorldSession, StartKit, StartRole,
        StartRoleDesignId,
    };
}

/// The world and the start the player asked for on the New Game screen.
///
/// Inserted by the menu's Create and by Load, for a scenario whose world is
/// `Generated((seed: None))`. It stays for the whole run, so a Retry streams
/// the same world again and spawns the same start. A new New Game replaces
/// it.
#[derive(Resource, Clone, Debug, Eq, PartialEq)]
pub struct OpenWorldSession {
    /// The world seed every sector is generated from.
    pub seed: u32,
    /// The start role the player ship spawns as.
    pub role: StartRoleDesignId,
}

/// The input bindings a start role's hull spawns with.
///
/// Every turret takes the primary trigger (left mouse and the right gamepad
/// trigger), every railgun `R`, every torpedo bay `F` and every mining
/// emitter `V`. No other section is bound: the flight computer drives the
/// thrusters.
pub fn start_role_bindings(hull: &ResolvedShipDesign) -> BTreeMap<SectionId, Vec<InputSource>> {
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
                SectionKind::Mining(_) => vec![InputSource::Keyboard(KeyCode::KeyV)],
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

/// One New Game start: the hull the player flies and the kit it starts with.
///
/// Every field is required in RON. A hull no visible ship design carries, an
/// unknown kit item and a kit heavier than the hull's hold are refused at
/// lint, see `lint_start_role`.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct StartRole {
    /// The id the role is picked by.
    pub id: StartRoleDesignId,
    /// The name a picker shows.
    pub name: String,
    /// How the role plays, for a reader.
    pub about: String,
    /// The catalog ship design the player starts in.
    pub hull: ShipDesignId,
    /// What the ship starts with.
    pub kit: StartKit,
    /// Background text for the role.
    pub lore: String,
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
