//! The shape a start role's icon is drawn as.
//!
//! A start role is content (`StartRole` in `nova_scenario`) and the masks are
//! drawn by the interface, which cannot depend on the scenario crate; the
//! enum lives here, below both, as [`JobMixType`](crate::jobs::JobMixType)
//! does.

use bevy::prelude::*;

/// Glob-import surface: `use nova_gameplay::role_icon::prelude::*`.
pub mod prelude {
    pub use super::RoleIconType;
}

/// The shapes a role's icon may be. Closed: the interface draws each one, so
/// a mod role picks a shape and cannot add one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RoleIconType {
    /// A drill bit.
    Drill,
    /// A three-pronged grabber.
    Claw,
    /// A ribbed freight container.
    Container,
    /// A bounty star.
    Star,
    /// A shield with a rank chevron.
    Shield,
    /// Two crossed blades.
    Blades,
}
