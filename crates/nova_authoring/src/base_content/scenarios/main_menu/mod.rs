//! Private scenarios rendered behind the main menu.

mod ambience;
mod scrapyard;
mod shared;
mod waystation;

pub(crate) use ambience::menu_ambience as ambience;
pub(crate) use scrapyard::menu_scrapyard as scrapyard;
pub(crate) use waystation::menu_waystation as waystation;
