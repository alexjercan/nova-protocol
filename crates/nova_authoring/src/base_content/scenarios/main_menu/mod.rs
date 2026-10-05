//! Private scenarios rendered behind the main menu.
//!
//! Each scene plays its act and then reports it finished with
//! `BackdropDone`; the menu owns the rotation and picks what plays next.
//! Scenes with a natural ending (the gauntlet's fallen stand, the duel's
//! erased victor) report from their aftermath; the endless ones (weave,
//! waystation) carry a rotation time limit.

mod duel;
mod gauntlet;
mod shared;
mod waystation;
mod weave;

/// The four backdrop scenario ids.
const MENU_WAYSTATION_SCENARIO_ID: &str = "menu_waystation";
const MENU_GAUNTLET_SCENARIO_ID: &str = "menu_gauntlet";
const MENU_WEAVE_SCENARIO_ID: &str = "menu_weave";
const MENU_DUEL_SCENARIO_ID: &str = "menu_duel";

pub(crate) use duel::menu_duel as duel;
pub(crate) use gauntlet::menu_gauntlet as gauntlet;
pub(crate) use waystation::menu_waystation as waystation;
pub(crate) use weave::menu_weave as weave;
