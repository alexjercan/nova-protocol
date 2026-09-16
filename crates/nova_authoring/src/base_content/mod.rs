//! The base game's authored content inventory.
//!
//! The base game is a built-in mod. This domain owns its scenarios, section
//! prototypes, semantic craft assemblies, and path-based asset refs. The story
//! campaign is a shipped mod of its own (`mod_content`), and generic authoring
//! helpers, lint, and serialization tooling stay outside both.

use nova_scenario::prelude::{ScenarioConfig, ShipDesignPrototype};
use nova_ship::prelude::{SectionConfig, ShipStyleConfig};
use nova_training::prelude::Lesson;

pub(crate) mod assets;
pub(crate) mod lessons;
pub(crate) mod scenarios;
pub(crate) mod sections;
pub(crate) mod ships;
pub(crate) mod styles;

use assets::BaseContentAssets;

/// Complete built-in content before it is wrapped and serialized as mod items.
pub(crate) struct BaseContent {
    pub(crate) sections: Vec<SectionConfig>,
    pub(crate) ships: Vec<ShipDesignPrototype>,
    pub(crate) scenarios: Vec<ScenarioConfig>,
    pub(crate) styles: Vec<ShipStyleConfig>,
    pub(crate) lessons: Vec<Lesson>,
}

/// Build every built-in content family from one explicit asset inventory.
pub(crate) fn build() -> BaseContent {
    let assets = BaseContentAssets::from_paths();
    BaseContent {
        sections: sections::section_catalog(&assets),
        ships: ships::ship_catalog(&assets),
        scenarios: scenarios::catalog(&assets),
        styles: styles::style_catalog(&assets),
        lessons: lessons::lesson_catalog(),
    }
}
