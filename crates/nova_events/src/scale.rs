//! The world's physical scale: what one world unit is worth in meters, and how
//! much acceleration hull metal takes before it tears.
//!
//! Both are single definitions on purpose - display and physics must agree, and
//! two constants that must agree is the same fault as a re-typed id. Change
//! these only to retune the whole game at once.

/// Meters in one world unit.
///
/// Every world-space length - a section box, a lock range, a structural arm -
/// is in units; anything expressed in SI (a G limit, a player-facing readout)
/// has to cross here first. Dropping the conversion makes a hull ten times
/// sharper than it is.
pub const METERS_PER_UNIT: f32 = 10.0;

/// Structural load a hull tolerates at any point on it, in m/s2.
///
/// The whole game shares one number: hull metal is hull metal, so there is no
/// "which section's limit binds" question to answer. It is a game-design choice
/// with no physical derivation - the single dial that sets the size curve for
/// attitude control, while the ratios between ship sizes stay fixed by geometry
/// whatever it is set to.
pub const LOAD_LIMIT: f32 = 8.0 * 9.81;
