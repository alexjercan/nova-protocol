//! Built-in scenario inventory, grouped by player-facing purpose.

use nova_scenario::prelude::ScenarioConfig;

use super::assets::BaseContentAssets;

pub(crate) mod drills;
pub(crate) mod main_menu;
pub(crate) mod marks;
pub(crate) mod pacing;
pub(crate) mod season_one;
pub(crate) mod tutorial;

/// The keybind-dock chips a scenario pulses, by the verb each one draws.
/// Shared: the training range, the practice drills and the campaign all point
/// at the same chip when they hand a verb over.
pub(crate) const HINT_STOP: &str = "STOP";
pub(crate) const HINT_RCS: &str = "RCS";
pub(crate) const HINT_RADAR: &str = "RADAR";
pub(crate) const HINT_GOTO: &str = "GOTO";
pub(crate) const HINT_ORBIT: &str = "ORBIT";
pub(crate) const HINT_DOCK: &str = "DOCK";

/// Fixed seed shared by deterministic built-in scatter fields.
pub(crate) const SCATTER_SEED: u64 = 0x0605_0403_0201_0000;

/// Every built-in scenario in stable generated-content order.
pub(crate) fn catalog(assets: &BaseContentAssets) -> Vec<ScenarioConfig> {
    let cubemap = || assets.cubemap.clone();
    let texture = || assets.asteroid_texture.clone();

    let mut catalog = vec![
        main_menu::waystation(cubemap(), texture()),
        main_menu::gauntlet(cubemap(), texture()),
        main_menu::weave(cubemap(), texture()),
        main_menu::duel(assets),
        tutorial::tutorial(cubemap(), texture()),
        season_one::chapter_one(cubemap(), texture()),
    ];
    catalog.extend(drills::catalog(assets));
    catalog
}
