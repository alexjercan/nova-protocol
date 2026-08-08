//! The RON generation surface for the built-in scenarios. The scenario builders
//! are the single definition of each built-in; production loads their serialized
//! RON. This module rebuilds them with PATH-based asset refs and serializes them
//! deterministically for two consumers that must agree byte for byte: the
//! `content` CLI's `gen` subcommand WRITES the committed files (`cargo run -p
//! nova_assets --bin content -- gen`) and the `content_ron_parity` integration
//! test ASSERTS them. Not part of the game's public API.
//!
//! NOTE: the `ScenarioConfig` serde derives are already present in this crate's
//! build - `nova_modding` (a dependency) turns on `nova_scenario/serde`, and
//! Cargo feature unification carries it here.

use nova_gameplay::prelude::AssetRef;
use nova_modding::prelude::Content;
use nova_scenario::prelude::{CampaignConfig, ScenarioConfig};
use nova_ship::prelude::SectionConfig;

use crate::sections::{build_sections, SectionMeshRefs};

/// The built-in builders, the deterministic RON serializer they are written
/// through, and [`content_files`] - the file-by-file view `gen` writes and the
/// parity test asserts.
pub mod prelude {
    pub use super::{
        build_campaign_contents, build_campaigns, build_scenario_contents, build_scenarios,
        build_section_catalog, build_section_content, content_files, pretty_config,
        serialize_content,
    };
}

/// The skybox cubemap asset path (matches `GameAssets::cubemap`).
const CUBEMAP_PATH: &str = "self://textures/cubemap.png";
/// Broadside's deep-field sky: the alt cubemap, so chapter two reads as
/// a different place than the trainer belt.
const CUBEMAP_ALT_PATH: &str = "self://textures/cubemap_alt.png";
/// The asteroid texture asset path (matches `GameAssets::asteroid_texture`).
const ASTEROID_TEXTURE_PATH: &str = "self://textures/asteroid.png";

/// The section-prototype catalog built from PATH-based mesh refs - the source
/// the content parity test wraps as `Content::Section` items and serializes
/// into `assets/base/sections/base.content.ron` (production loads that file
/// via the base bundle and routes its items into `GameSections` via
/// `register_bundles`).
pub fn build_section_catalog() -> Vec<SectionConfig> {
    build_sections(&SectionMeshRefs::from_paths())
}

/// Build the built-in configs with path-based asset refs, in a stable
/// order. This is the source the parity test serializes and compares. The
/// ships now reference the section catalog by prototype id, so the scenario
/// generators no longer need the resolved `GameSections`.
pub fn build_scenarios() -> Vec<ScenarioConfig> {
    let cubemap = || AssetRef::from(CUBEMAP_PATH.to_string());
    let texture = || AssetRef::from(ASTEROID_TEXTURE_PATH.to_string());

    vec![
        crate::scenario::asteroid_next(cubemap()),
        crate::scenario::asteroid_field(cubemap(), texture()),
        crate::scenario::menu::menu_ambience(cubemap(), texture()),
        crate::scenario::menu::menu_waystation(cubemap(), texture()),
        crate::scenario::menu::menu_scrapyard(cubemap(), texture()),
        crate::scenario::shakedown::shakedown_run(cubemap(), texture()),
        crate::scenario::broadside::broadside(
            AssetRef::from(CUBEMAP_ALT_PATH.to_string()),
            texture(),
        ),
        crate::scenario::broadside::broadside_gunship(
            AssetRef::from(CUBEMAP_ALT_PATH.to_string()),
            texture(),
        ),
        crate::scenario::lifeline::lifeline(
            AssetRef::from(CUBEMAP_ALT_PATH.to_string()),
            texture(),
        ),
        crate::scenario::final_tally::final_tally(
            AssetRef::from(CUBEMAP_ALT_PATH.to_string()),
            texture(),
        ),
    ]
}

/// The base game's campaigns, in a stable order. Today just "Nova Protocol",
/// the base storyline, listing its chapters in play order - the three visible
/// chapter-heads plus the two `hidden` chained members (broadside_gunship, the
/// phase-two wave; final_tally, the epilogue), so both are reachable for
/// replay under the campaign header. The member ids reference the scenario-id
/// constants so a scenario rename cannot silently orphan a member.
pub fn build_campaigns() -> Vec<CampaignConfig> {
    vec![CampaignConfig {
        id: "nova_protocol".to_string(),
        name: "Nova Protocol".to_string(),
        scenarios: vec![
            crate::scenario::shakedown::SHAKEDOWN_SCENARIO_ID.to_string(),
            crate::scenario::broadside::BROADSIDE_SCENARIO_ID.to_string(),
            crate::scenario::broadside::BROADSIDE_GUNSHIP_SCENARIO_ID.to_string(),
            crate::scenario::lifeline::LIFELINE_SCENARIO_ID.to_string(),
            crate::scenario::final_tally::FINAL_TALLY_SCENARIO_ID.to_string(),
        ],
    }]
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
    let mut files = vec![(
        "base/sections/base.content.ron".to_string(),
        serialize_content(&build_section_content()),
    )];
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
