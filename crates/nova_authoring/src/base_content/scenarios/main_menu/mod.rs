//! Private scenarios rendered behind the main menu.

mod ambience;
mod duel;
mod gauntlet;
mod scrapyard;
mod shared;
mod waystation;
mod weave;

pub(crate) use ambience::menu_ambience as ambience;
pub(crate) use duel::menu_duel as duel;
pub(crate) use gauntlet::menu_gauntlet as gauntlet;
pub(crate) use scrapyard::menu_scrapyard as scrapyard;
pub(crate) use waystation::menu_waystation as waystation;
pub(crate) use weave::menu_weave as weave;
