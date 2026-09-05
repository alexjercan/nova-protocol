//! Built-in scenario inventory, grouped by player-facing purpose.

use nova_scenario::prelude::ScenarioConfig;

use super::assets::BaseContentAssets;

pub(crate) mod main_menu;
pub(crate) mod nova_protocol;

/// Fixed seed shared by deterministic built-in scatter fields.
pub(crate) const SCATTER_SEED: u64 = 0x0605_0403_0201_0000;
/// Watched scenario clock name used by mainline pacing helpers.
pub(crate) const SCENARIO_ELAPSED_VAR: &str = "scenario_elapsed";

/// Every built-in scenario in stable generated-content order.
pub(crate) fn catalog(assets: &BaseContentAssets) -> Vec<ScenarioConfig> {
    let cubemap = || assets.cubemap.clone();
    let texture = || assets.asteroid_texture.clone();

    vec![
        main_menu::waystation(cubemap(), texture()),
        main_menu::gauntlet(cubemap(), texture()),
        main_menu::weave(cubemap(), texture()),
        main_menu::duel(cubemap(), texture()),
        nova_protocol::first_shift(cubemap(), texture(), &assets.portraits),
        nova_protocol::second_shift(cubemap(), texture(), &assets.portraits),
    ]
}
