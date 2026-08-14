//! Private scenarios rendered behind the main menu.
//!
//! The four backdrops form a Factorio-style CAROUSEL: each scene plays its
//! act and then hands off to the NEXT one with a `NextScenario` (gauntlet ->
//! weave -> duel -> waystation -> gauntlet). Scenes with a natural ending
//! (the gauntlet's fallen stand, the duel's erased victor) hand off from
//! their aftermath; the endless ones (weave, waystation) carry a rotation
//! time limit. The menu's random draw picks only the ENTRY point of the
//! ring.

mod duel;
mod gauntlet;
mod shared;
mod waystation;
mod weave;

pub(crate) use duel::menu_duel as duel;
pub(crate) use gauntlet::menu_gauntlet as gauntlet;
pub(crate) use waystation::menu_waystation as waystation;
pub(crate) use weave::menu_weave as weave;
