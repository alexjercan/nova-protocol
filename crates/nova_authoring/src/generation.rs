//! Deterministic RON serialization for the private built-in content inventory.
//! The builders under `base_content` are the single definition of each built-in;
//! production loads their serialized RON. This module rebuilds them with
//! path-based asset refs and serializes them
//! deterministically for two consumers that must agree byte for byte: the
//! `content` CLI's `gen` subcommand WRITES the committed files (`cargo run
//! content gen`) and the `content_ron_parity` integration test ASSERTS them.
//! Not part of the game's public API.
//!
//! NOTE: the `ScenarioConfig` serde derives are already present in this crate's
//! build - `nova_modding` (a dependency) turns on `nova_scenario/serde`, and
//! Cargo feature unification carries it here.

use nova_modding::prelude::Content;
use nova_scenario::prelude::{
    CampaignConfig, ScenarioConfig, ShipConfig, ShipSource, SpaceshipConfig, SpaceshipSectionConfig,
};
use nova_ship::prelude::{SectionConfig, ShipStyleConfig};

use crate::base_content;

/// The built-in builders, the deterministic RON serializer they are written
/// through, and [`content_files`] - the file-by-file view `gen` writes and the
/// parity test asserts.
pub mod prelude {
    pub use super::{
        build_campaign_contents, build_campaigns, build_scenario_contents, build_scenarios,
        build_section_catalog, build_section_content, build_ship_content, build_ships,
        build_style_content, build_styles, content_files, pretty_config, serialize_content,
        spawned_ship_sections,
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

/// The base game's campaigns, in a stable order. Today just "Nova Protocol",
/// the base storyline, listing its chapters in play order - the three visible
/// chapter-heads plus the two `hidden` chained members (broadside_gunship, the
/// phase-two wave; final_tally, the epilogue), so both are reachable for
/// replay under the campaign header. The member ids reference the scenario-id
/// constants so a scenario rename cannot silently orphan a member.
pub fn build_campaigns() -> Vec<CampaignConfig> {
    base_content::build().campaigns
}

/// The base game's skin styles, in a stable order - the look a ship's derived
/// cladding wears, named by id from its config.
pub fn build_styles() -> Vec<ShipStyleConfig> {
    base_content::build().styles
}

/// The base game's ships, in a stable order - the whole hulls the scenarios
/// spawn by id.
pub fn build_ships() -> Vec<ShipConfig> {
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

/// The section list one authored spawn flies: its inline hull's, or that of
/// the built-in ship it names. The join every scenario pin needs now that a
/// scenario REFERENCES a hull instead of carrying one; an unknown id resolves
/// to nothing (the content lint is what errors on it).
pub fn spawned_ship_sections(ship: &SpaceshipConfig) -> Vec<SpaceshipSectionConfig> {
    match &ship.hull {
        ShipSource::Inline(hull) => hull.sections.clone(),
        ShipSource::Prototype(id) => build_ships()
            .into_iter()
            .find(|entry| entry.id == *id)
            .map(|entry| entry.hull.sections)
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

/// Each built-in campaign wrapped as its own single-item `Vec<Content>`
/// (`[Content::Campaign(..)]`) keyed by campaign id - the shape each committed
/// `assets/base/campaigns/<id>.content.ron` file carries. The parity test
/// serializes each.
pub fn build_campaign_contents() -> Vec<(String, Vec<Content>)> {
    build_campaigns()
        .into_iter()
        .map(|campaign| (campaign.id.clone(), vec![Content::Campaign(campaign)]))
        .collect()
}

/// The deterministic pretty-printer for the built-in content RON. Matches
/// the hand-authored mod content style (e.g. `assets/mods/example/example.content.ron`):
/// struct names omitted, indented, so the data files stay diff-friendly and
/// reviewable.
pub fn pretty_config() -> ron::ser::PrettyConfig {
    ron::ser::PrettyConfig::default()
        .struct_names(false)
        .separate_tuple_members(true)
        .enumerate_arrays(false)
}

/// Serialize one content `Vec` the way the committed files are authored:
/// the deterministic pretty config plus a trailing newline (POSIX-clean).
pub fn serialize_content(content: &[Content]) -> String {
    let body = ron::ser::to_string_pretty(&content.to_vec(), pretty_config())
        .expect("serialize content Vec");
    format!("{body}\n")
}

/// Every builder-backed content file as (assets-root-relative path,
/// serialized body), in a stable order. The single file map both the
/// `content` CLI's `gen` subcommand (writes) and the parity test
/// (asserts) walk, so the two can never disagree about what exists or
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
    ];
    files.extend(build_scenario_contents().into_iter().map(|(id, content)| {
        (
            format!("base/scenarios/{id}.content.ron"),
            serialize_content(&content),
        )
    }));
    files.extend(build_campaign_contents().into_iter().map(|(id, content)| {
        (
            format!("base/campaigns/{id}.content.ron"),
            serialize_content(&content),
        )
    }));
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
                    SectionSource::Prototype(id) => catalog
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
            check(&format!("ship '{}'", ship.id), &ship.hull.sections);
        }

        for scenario in build_scenarios() {
            for event in &scenario.events {
                for action in &event.actions {
                    for object in ship_objects(action) {
                        let ScenarioObjectKind::Spaceship(ship) = &object.kind else {
                            continue;
                        };
                        // A Prototype hull is checked once above, where it is
                        // authored - the same rule a Prototype section follows.
                        let ShipSource::Inline(hull) = &ship.hull else {
                            continue;
                        };
                        check(
                            &format!("scenario '{}' ship '{}'", scenario.id, object.base.id),
                            &hull.sections,
                        );
                    }
                }
            }
        }
    }
}
