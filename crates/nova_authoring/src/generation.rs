//! Deterministic RON serialization for the private built-in content inventory.
//! The builders under `base_content` are the single definition of each
//! built-in; production loads their serialized RON. This module rebuilds them
//! with path-based asset refs and serializes them deterministically for two
//! consumers that must agree byte for byte: the `content` CLI's `gen`
//! subcommand WRITES the committed files (`cargo run content gen`) and the
//! `content_ron_parity` integration test ASSERTS them. Not part of the game's
//! public API.
//!
//! The `ScenarioConfig` serde derives are already present in this crate's
//! build - `nova_modding` (a dependency) turns on `nova_scenario/serde`, and
//! Cargo feature unification carries it here.

use nova_modding::prelude::Content;
use nova_scenario::prelude::{
    CampaignConfig, ScenarioConfig, ShipDesignPrototype, ShipDesignSource, SpaceshipConfig,
    SpaceshipSectionConfig,
};
use nova_ship::prelude::{SectionConfig, ShipStyleConfig};
use nova_training::prelude::Lesson;
use nova_ui::theme::UiThemeConfig;

use crate::base_content;

/// The built-in builders, the deterministic RON serializer they are written
/// through, and [`content_files`] - the file-by-file view `gen` writes and the
/// parity test asserts.
pub mod prelude {
    pub use super::{
        build_campaign_content, build_campaigns, build_lesson_content, build_lessons,
        build_scenario_contents, build_scenarios, build_section_catalog, build_section_content,
        build_ship_content, build_ships, build_style_content, build_styles, content_files,
        serialize_content, spawned_ship_sections,
    };
}

/// The section-prototype catalog built from PATH-based mesh refs - the source
/// the content parity test wraps as `Content::Section` items and serializes
/// into `assets/base/sections/base.content.ron` (production loads that file
/// via the base bundle and routes its items into `GameSections` via
/// `register_bundles`).
pub fn build_section_catalog() -> Vec<SectionConfig> {
    base_content::build().sections
}

/// Build the built-in configs with path-based asset refs, in a stable
/// order. This is the source the parity test serializes and compares. The
/// ships now reference the section catalog by prototype id, so the scenario
/// generators no longer need the resolved `GameSections`.
pub fn build_scenarios() -> Vec<ScenarioConfig> {
    base_content::build().scenarios
}

/// The base game's campaigns, in a stable order - the groups the Scenarios
/// board lists its chapters under.
pub fn build_campaigns() -> Vec<CampaignConfig> {
    base_content::build().campaigns
}

/// The campaign catalog wrapped as one `Vec<Content>` of `Content::Campaign`
/// items - the shape the committed `assets/base/campaigns/base.content.ron`
/// file carries.
///
/// ONE file for every campaign, like the styles: a campaign is a short list of
/// scenario ids, and the set is read as a whole.
pub fn build_campaign_content() -> Vec<Content> {
    build_campaigns()
        .into_iter()
        .map(Content::Campaign)
        .collect()
}

/// The base game's skin styles, in a stable order - the look a ship's derived
/// cladding wears, named by id from its config.
pub fn build_styles() -> Vec<ShipStyleConfig> {
    base_content::build().styles
}

/// The base game's ships, in a stable order - the whole hulls the scenarios
/// spawn by id.
pub fn build_ships() -> Vec<ShipDesignPrototype> {
    base_content::build().ships
}

/// The section catalog wrapped as one `Vec<Content>` of `Content::Section`
/// items - the shape the committed `assets/base/sections/base.content.ron` file
/// carries. The parity test serializes this.
pub fn build_section_content() -> Vec<Content> {
    build_section_catalog()
        .into_iter()
        .map(|section| Content::Section(Box::new(section)))
        .collect()
}

/// The style catalog wrapped as one `Vec<Content>` of `Content::Style` items -
/// the shape the committed `assets/base/styles/base.content.ron` file carries.
///
/// ONE file for every style rather than one file each, like the sections: a
/// style is small, the set is short, and a look is read against the others.
pub fn build_style_content() -> Vec<Content> {
    build_styles().into_iter().map(Content::Style).collect()
}

/// The section list one authored spawn flies: its inline design's, or that of
/// the built-in design it names. The join every scenario pin needs now that a
/// scenario REFERENCES a design instead of carrying one; an unknown id
/// resolves to nothing (the content lint is what errors on it).
pub fn spawned_ship_sections(ship: &SpaceshipConfig) -> Vec<SpaceshipSectionConfig> {
    match &ship.design {
        ShipDesignSource::Inline(design) => design.sections.clone(),
        ShipDesignSource::Prototype { id, .. } => build_ships()
            .into_iter()
            .find(|entry| entry.id == *id)
            .map(|entry| entry.design.sections)
            .unwrap_or_default(),
    }
}

/// The ship catalog wrapped as one `Vec<Content>` of `Content::Ship` items -
/// the shape the committed `assets/base/ships/base.content.ron` file carries.
///
/// ONE file for every ship, like the sections and the styles: the set is short
/// and a hull is read against the others it shares parts with.
pub fn build_ship_content() -> Vec<Content> {
    build_ships().into_iter().map(Content::Ship).collect()
}

/// The base game's training lessons, in the order the file carries them.
pub fn build_lessons() -> Vec<Lesson> {
    base_content::build().lessons
}

/// The lesson catalog wrapped as one `Vec<Content>` of `Content::Lesson`
/// items - the shape the committed `assets/base/training/base.content.ron`
/// file carries.
///
/// ONE file for the whole handbook: a lesson is read against its neighbours -
/// what the category holds, where `order` puts it - and a file per lesson
/// would hide the running order the screen actually draws.
pub fn build_lesson_content() -> Vec<Content> {
    build_lessons().into_iter().map(Content::Lesson).collect()
}

/// The base mod's UI themes, in the order the file carries them - the default
/// first, so the Settings picker lists it first.
pub fn build_ui_themes() -> Vec<UiThemeConfig> {
    nova_ui::theme::base::base_ui_themes()
}

/// The UI themes wrapped as one `Vec<Content>` of `Content::UiTheme` items -
/// the shape the committed `assets/base/ui_themes/base.content.ron` file
/// carries.
///
/// ONE file for both looks: a theme is read against the other looks it shares a
/// role table with, and the file IS the worked example a mod copies to author
/// its own.
pub fn build_ui_theme_content() -> Vec<Content> {
    build_ui_themes()
        .into_iter()
        .map(|theme| Content::UiTheme(Box::new(theme)))
        .collect()
}

/// The built-in scenarios, each wrapped as its own single-item
/// `Vec<Content>` (`[Content::Scenario(..)]`) keyed by scenario id - the
/// shape each committed `assets/scenarios/<id>.content.ron` file carries. The
/// parity test serializes each.
pub fn build_scenario_contents() -> Vec<(String, Vec<Content>)> {
    build_scenarios()
        .into_iter()
        .map(|scenario| (scenario.id.clone(), vec![Content::Scenario(scenario)]))
        .collect()
}

/// Serialize one content `Vec` the way the committed files are authored.
///
/// The formatter itself lives in `nova_modding` beside the format, because the
/// in-game editor writes content files too and two writers with two formatters
/// would produce two dialects of one format. This wrapper is the offline
/// tool's: a failed serialize of code-built content is a bug in the builders,
/// not a condition the CLI recovers from.
pub fn serialize_content(content: &[Content]) -> String {
    nova_modding::serialize_content(content).expect("serialize content Vec")
}

/// Every builder-backed content file as (assets-root-relative path,
/// serialized body), in a stable order, all of it under `base/`. The single
/// file map both the `content` CLI's `gen` subcommand (writes) and the parity
/// test (asserts) walk, so the two can never disagree about what exists or
/// what it contains.
pub fn content_files() -> Vec<(String, String)> {
    let mut files = vec![
        (
            "base/sections/base.content.ron".to_string(),
            serialize_content(&build_section_content()),
        ),
        (
            "base/styles/base.content.ron".to_string(),
            serialize_content(&build_style_content()),
        ),
        (
            "base/ships/base.content.ron".to_string(),
            serialize_content(&build_ship_content()),
        ),
        (
            "base/training/base.content.ron".to_string(),
            serialize_content(&build_lesson_content()),
        ),
        (
            "base/ui_themes/base.content.ron".to_string(),
            serialize_content(&build_ui_theme_content()),
        ),
    ];
    files.extend(build_scenario_contents().into_iter().map(|(id, content)| {
        (
            format!("base/scenarios/{id}.content.ron"),
            serialize_content(&content),
        )
    }));
    // After the scenarios: a campaign is a list of ids that must already exist,
    // so this is the reading order as well as the writing one.
    files.push((
        "base/campaigns/base.content.ron".to_string(),
        serialize_content(&build_campaign_content()),
    ));
    files
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use nova_scenario::prelude::{
        EventActionConfig, ScenarioObjectConfig, ScenarioObjectKind, SectionSource,
    };
    use nova_ship::prelude::{derive_link_point_graph, PlacedSectionLinkPoints};

    use super::*;

    fn ship_objects(action: &EventActionConfig) -> Vec<&ScenarioObjectConfig> {
        match action {
            EventActionConfig::SpawnScenarioObject(object) => vec![object],
            EventActionConfig::ScatterObjects(scatter) => vec![&scatter.template],
            _ => Vec::new(),
        }
    }

    /// Every hull the base game ships - the ship CATALOG entries and the
    /// one-off hulls a scenario still authors inline - resolves, mates, and
    /// carries no cube-era section id.
    #[test]
    fn every_built_in_parts_ship_has_a_valid_link_point_graph() {
        let catalog: HashMap<_, _> = build_section_catalog()
            .into_iter()
            .map(|section| (section.base.id.clone(), section))
            .collect();

        let check = |label: &str, sections: &[SpaceshipSectionConfig]| {
            let resolved: Vec<_> = sections
                .iter()
                .map(|section| match &section.source {
                    SectionSource::Inline(config) => config,
                    SectionSource::Prototype { id, .. } => catalog
                        .get(id.as_str())
                        .unwrap_or_else(|| panic!("missing prototype '{id}'")),
                })
                .collect();
            let placed: Vec<_> = sections
                .iter()
                .zip(&resolved)
                .map(|(section, config)| PlacedSectionLinkPoints {
                    position: section.position,
                    rotation: section.rotation,
                    link_points: &config.base.link_points,
                })
                .collect();
            let mates = derive_link_point_graph(&placed)
                .unwrap_or_else(|errors| panic!("{label} has invalid points: {errors:?}"));
            assert!(
                sections.len() <= 1 || !mates.is_empty(),
                "{label} has no structural mates"
            );
            assert!(
                sections
                    .iter()
                    .all(|section| !section.id.starts_with("cube_")),
                "{label} retains a cube section id"
            );
        };

        for ship in build_ships() {
            check(&format!("ship '{}'", ship.id), &ship.design.sections);
        }

        for scenario in build_scenarios() {
            for event in &scenario.events {
                for action in &event.actions {
                    for object in ship_objects(action) {
                        let ScenarioObjectKind::Spaceship(ship) = &object.kind else {
                            continue;
                        };
                        // A Prototype design is checked once above, where it
                        // is authored - the same rule a Prototype section
                        // follows.
                        let ShipDesignSource::Inline(design) = &ship.design else {
                            continue;
                        };
                        check(
                            &format!("scenario '{}' ship '{}'", scenario.id, object.base.id),
                            &design.sections,
                        );
                    }
                }
            }
        }
    }
}
