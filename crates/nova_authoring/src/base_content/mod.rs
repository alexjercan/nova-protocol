//! The base game's authored content inventory.
//!
//! The base game is a built-in mod. This domain owns its campaigns, scenarios,
//! section prototypes, semantic craft assemblies, and path-based asset refs.
//! Generic authoring helpers, lint, and serialization tooling stay outside it.

use nova_scenario::prelude::{CampaignConfig, ScenarioConfig};
use nova_ship::prelude::{SectionConfig, ShipStyleConfig};

pub(crate) mod assets;
pub(crate) mod campaigns;
pub(crate) mod scenarios;
pub(crate) mod sections;
pub(crate) mod ships;
pub(crate) mod styles;

use assets::BaseContentAssets;

/// Complete built-in content before it is wrapped and serialized as mod items.
pub(crate) struct BaseContent {
    pub(crate) sections: Vec<SectionConfig>,
    pub(crate) scenarios: Vec<ScenarioConfig>,
    pub(crate) campaigns: Vec<CampaignConfig>,
    pub(crate) styles: Vec<ShipStyleConfig>,
}

/// Build every built-in content family from one explicit asset inventory.
pub(crate) fn build() -> BaseContent {
    let assets = BaseContentAssets::from_paths();
    BaseContent {
        sections: sections::section_catalog(&assets),
        scenarios: scenarios::catalog(&assets),
        campaigns: campaigns::catalog(),
        styles: styles::style_catalog(&assets),
    }
}
