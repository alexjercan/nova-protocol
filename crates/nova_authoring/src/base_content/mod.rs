//! The base game's authored content inventory.
//!
//! The base game is a built-in mod. This domain owns its scenarios, section
//! prototypes, semantic craft assemblies, and path-based asset refs. The story
//! campaign is a shipped mod of its own (`mod_content`), and generic authoring
//! helpers, lint, and serialization tooling stay outside both.

use nova_gameplay::prelude::{ImpactSoundConfig, NarrativeChannelConfig};
use nova_scenario::prelude::{ScenarioConfig, ShipDesignPrototype};
use nova_ship::prelude::{SectionConfig, ShipGrammarConfig, ShipStyleConfig};
use nova_training::prelude::Lesson;

pub(crate) mod assets;
pub(crate) mod channels;
pub(crate) mod grammars;
pub(crate) mod impacts;
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
    pub(crate) impacts: Vec<ImpactSoundConfig>,
    pub(crate) grammars: Vec<ShipGrammarConfig>,
    pub(crate) channels: Vec<NarrativeChannelConfig>,
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
        impacts: impacts::impact_table(&assets),
        grammars: grammars::grammar_catalog(),
        channels: channels::channel_catalog(),
        lessons: lessons::lesson_catalog(),
    }
}
