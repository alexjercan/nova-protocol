//! Durable references a saved transient keeps to live bodies.
//!
//! A live transient points at other bodies by [`Entity`], which a save
//! cannot keep. A record keeps the body's [`EntityId`] instead, and a Load
//! resolves it against the bodies its window spawned. Ship and rock ids are
//! unique across a live window; section ids are unique only within a ship, so
//! a section reference pairs the two.
//!
//! The owner of `nova_world_base::save` resolves these references and refuses
//! a Load whose reference has no live match.

use bevy::prelude::*;
use nova_events::prelude::EntityId;

use crate::{inventory::CargoCanisterRuntimeId, lifetime::UnsettledBody};

/// The saved-reference records and the freeze fault.
pub mod prelude {
    pub use super::{
        SavedBodyRef, SavedOwner, SavedSectionRef, SavedTargetRef, TransientFreezeFault,
    };
}

/// The ship that fired a saved projectile.
///
/// `Gone` is not an absent owner. A projectile with no `ProjectileOwner` is
/// authorless, and its first collider consumes it. A projectile whose owner
/// died keeps a handle to a despawned entity: arming reads the launcher as
/// gone, the owner filter matches nothing, and the party comes from the
/// copied allegiance. `Gone` thaws to that same state.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SavedOwner {
    /// The live ship with this id.
    Ship(EntityId),
    /// The owner died before the save.
    Gone,
}

impl SavedOwner {
    /// The saved form of the owner handle `owner`.
    ///
    /// # Errors
    ///
    /// [`TransientFreezeFault::NoDurableId`] when `owner` is alive but has no
    /// [`EntityId`], so no Load could find it again.
    pub fn of(world: &World, owner: Entity) -> Result<Self, TransientFreezeFault> {
        let Ok(body) = world.get_entity(owner) else {
            return Ok(Self::Gone);
        };
        match body.get::<EntityId>() {
            Some(id) => Ok(Self::Ship(id.clone())),
            None => Err(TransientFreezeFault::NoDurableId {
                label: body.get::<Name>().map_or_else(
                    || format!("owner {owner}"),
                    |name| format!("owner '{name}'"),
                ),
            }),
        }
    }
}

/// A live body a saved transient points at, by its window-unique id.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SavedBodyRef(pub EntityId);

/// A live section a saved transient points at: its ship's id and its
/// ship-local section id.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SavedSectionRef {
    /// The id of the ship that holds the section.
    pub ship: EntityId,
    /// The section's id within that ship.
    pub section: EntityId,
}

/// A live body a saved torpedo tracks, by the key the save already has for
/// its kind.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SavedTargetRef {
    /// A ship, rock or severed wreck, by its window-unique id.
    Body(SavedBodyRef),
    /// A cargo canister, by its runtime id.
    Canister(CargoCanisterRuntimeId),
    /// Another saved transient (a torpedo, rock chunk, detached piece or
    /// shed fixture), by its index in the same save's transient list.
    Transient(usize),
}

/// Why a transient cannot be saved on this frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransientFreezeFault {
    /// A multi-frame process still runs on it. The save waits.
    Unsettled(UnsettledBody),
    /// A live body it points at has no [`EntityId`]. The save fails visibly
    /// and keeps the last good save.
    NoDurableId {
        /// The reference, for the failure line.
        label: String,
    },
}

impl std::fmt::Display for TransientFreezeFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsettled(reason) => write!(f, "{reason}"),
            Self::NoDurableId { label } => write!(f, "{label} has no durable id"),
        }
    }
}
