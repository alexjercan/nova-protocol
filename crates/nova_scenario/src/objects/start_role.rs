//! The START ROLE content kind: one New Game start a player picks.
//!
//! A role names a catalog hull and a kit: the credits and the items the ship
//! starts with. It is authored once per role as a [`StartRole`] and read
//! through [`GameStartRoles`]. The lint (`lint_start_role`) refuses a hull no
//! visible ship design carries, an unknown kit item and a kit the hull's hold
//! cannot carry.

use bevy::prelude::*;
use nova_gameplay::prelude::ShipInventoryStock;

use crate::objects::ship_design::prelude::ShipDesignId;

/// Glob-import surface for the start role content kind.
pub mod prelude {
    pub use super::{GameStartRoles, StartKit, StartRole, StartRoleDesignId};
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
