//! How much acceleration hull metal takes before it tears.
//!
//! A single definition on purpose - a game-design dial the whole world shares,
//! and two constants that must agree is the same fault as a re-typed id. The
//! other half of the world's scale, the meters an engine world unit is worth,
//! lives with the quantity types in [`units`](crate::units).

use crate::units::prelude::*;

/// Structural load a hull tolerates at any point on it.
///
/// The whole game shares one number: hull metal is hull metal, so there is no
/// "which section's limit binds" question to answer. It is a game-design choice
/// with no physical derivation - the single dial that sets the size curve for
/// attitude control, while the ratios between ship sizes stay fixed by geometry
/// whatever it is set to.
pub const LOAD_LIMIT: MetersPerSecondSquared = MetersPerSecondSquared(8.0 * 9.81);
