//! The Nova Protocol story mod: the campaign, its chapter, and the story
//! vocabulary around it.
//!
//! Shipped under `assets/mods/nova_protocol/`, enabled on a fresh install, and
//! reachable only from the Scenarios picker: New Game starts the base game's
//! training range instead. The story borrows the base game's sky, rocks,
//! sounds and the player's own portrait through `dep://base/`, and ships the
//! rest of its faces and its thumbnail itself.

mod campaign;
mod cast;
mod first_shift;
mod portraits;
mod stage;

pub(crate) use first_shift::first_shift;
pub use first_shift::{first_shift_scene, FirstShiftScene};
use nova_gameplay::prelude::AssetRef;
use nova_scenario::prelude::{CampaignConfig, ScenarioConfig};
pub use portraits::CampaignPortraits;

pub(crate) use crate::base_content::{
    scenarios::{pacing, SCENARIO_ELAPSED_VAR},
    ships,
};

/// The mod's catalog id, its directory name under `assets/mods/`, and the id
/// of the one campaign it carries.
pub(crate) const MOD_ID: &str = "nova_protocol";
/// The mod's directory under `assets/`, the prefix its generated files carry.
pub(crate) const MOD_DIR: &str = "mods/nova_protocol";

/// The story's content families, in generated-content order.
pub(crate) struct NovaProtocolContent {
    pub(crate) scenarios: Vec<ScenarioConfig>,
    pub(crate) campaigns: Vec<CampaignConfig>,
}

/// Build the story mod's content with the refs its generated files carry.
pub(crate) fn build() -> NovaProtocolContent {
    let cubemap = AssetRef::from("dep://base/textures/cubemap.png".to_string());
    let asteroid_texture = AssetRef::from("dep://base/textures/asteroid.png".to_string());
    NovaProtocolContent {
        scenarios: vec![first_shift(
            cubemap,
            asteroid_texture,
            &CampaignPortraits::authored(),
        )],
        campaigns: campaign::catalog(),
    }
}

#[cfg(test)]
mod tests;
