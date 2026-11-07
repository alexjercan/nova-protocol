//! The ids a mod project mints, and how they turn into the ids it publishes.
//!
//! THE SCHEME. Three constraints meet here. A local mod keeps [`EDITOR_ID_PREFIX`]
//! because that prefix is what makes a save unable to reach a downloaded mod and
//! a download unable to land in the builder's list. The portal refuses that
//! prefix and any `_`, so a published id cannot carry it. And a `dep://` or a
//! dependency entry must still name the same mod on both sides of the line.
//!
//! So a mod has two spellings of one name: the LOCAL id `editor_<slug>` it is
//! worked on under, and the PUBLISH id `<slug>` a package carries, where a save
//! from before this scheme also has its `_` turned into `-`. [`publish_id`] and
//! [`local_id`] are the only crossing, and packaging and import both go through
//! them, so `self://` refs never change and `dep://` refs change by that one
//! rule.
//!
//! Content ids do not carry the mod's prefix at all. A scenario is
//! `<namespace>__<slug>` in snake_case, the one spelling a scenario id takes,
//! minted once and kept across a rename. `__` cannot occur in either half, so
//! two mods cannot mint one id by where a word falls. Every design it writes is
//! `<scenario>.<node>`: the merge overlays designs last-wins by id, so a bare
//! `ship_1` in two enabled mods gave one of them the other's hull.

use std::collections::HashSet;

pub(crate) use nova_assets::mod_cache::prelude::EDITOR_ID_PREFIX;
use nova_modding::prelude::BASE_MOD_ID;

use crate::scenario::SANDBOX_ID;

/// Why a typed name cannot become a mod id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NameProblem {
    /// The name holds nothing an id can be made of.
    Unusable,
    /// The name derives an id the game keeps for itself.
    Reserved,
}

/// `name` as lower-case ASCII words joined by `-`, the only spelling the portal
/// accepts. Empty when the name holds no letter or digit.
pub(crate) fn slug(name: &str) -> String {
    let mut slug = String::new();
    let mut separated = false;
    for character in name.chars() {
        if !character.is_ascii_alphanumeric() {
            separated = true;
            continue;
        }
        if separated && !slug.is_empty() {
            slug.push('-');
        }
        separated = false;
        slug.push(character.to_ascii_lowercase());
    }
    slug
}

/// The local mod id a typed name creates.
///
/// Derived rather than typed: an id is a directory name, an enable key and a
/// merge namespace at once. [`SANDBOX_ID`] is the range Play lowers the open
/// document into, and `base` is the game, so neither can be a builder's mod.
pub(crate) fn project_id(name: &str) -> Result<String, NameProblem> {
    let slug = slug(name);
    if slug.is_empty() {
        return Err(NameProblem::Unusable);
    }
    let id = format!("{EDITOR_ID_PREFIX}{slug}");
    if id == SANDBOX_ID || slug == BASE_MOD_ID {
        return Err(NameProblem::Reserved);
    }
    Ok(id)
}

/// What a scenario id of mod `id` starts with.
///
/// The local slug and not the publish id: `editor_a_b` and `editor_a-b` are two
/// mods on one machine, and they must not mint the same scenario id.
pub(crate) fn namespace_of(id: &str) -> &str {
    id.strip_prefix(EDITOR_ID_PREFIX).unwrap_or(id)
}

/// The id a local mod is published under.
pub(crate) fn publish_id(local: &str) -> String {
    namespace_of(local).replace('_', "-")
}

/// Whether a mod `id` would land on one of `known`. By publish id, because
/// `editor_a_b` and `editor_a-b` are two directories and one portal entry.
pub(crate) fn is_taken(id: &str, known: &[String]) -> bool {
    let published = publish_id(id);
    known.iter().any(|held| publish_id(held) == published)
}

/// The local id a published mod is worked on under.
#[cfg_attr(
    target_arch = "wasm32",
    expect(
        dead_code,
        reason = "only Package reads this, and the web has no Package"
    )
)]
pub(crate) fn local_id(published: &str) -> String {
    format!("{EDITOR_ID_PREFIX}{published}")
}

/// A new scenario id under `namespace`, past every id in `taken`.
pub(crate) fn scenario_id(namespace: &str, name: &str, taken: &HashSet<String>) -> String {
    let stem = match slug(name) {
        slug if slug.is_empty() => "scenario".to_string(),
        slug => slug.replace('-', "_"),
    };
    let first = format!("{}__{stem}", namespace.replace('-', "_"));
    if !taken.contains(&first) {
        return first;
    }
    (2..)
        .map(|ordinal| format!("{first}_{ordinal}"))
        .find(|id| !taken.contains(id))
        .unwrap_or(first)
}

/// The design id of node `node` in scenario `scenario`.
pub(crate) fn design_id(scenario: &str, node: &str) -> String {
    format!("{scenario}.{node}")
}

/// The node a design id of `scenario` was written for, or the id as it stands
/// when another scenario, a hand-written file or an older save wrote it.
pub(crate) fn node_of_design<'a>(scenario: &str, design: &'a str) -> &'a str {
    design
        .strip_prefix(scenario)
        .and_then(|rest| rest.strip_prefix('.'))
        .unwrap_or(design)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_derives_an_id_the_portal_accepts_once_published() {
        let id = project_id("My  Range!").expect("the name holds words");

        assert_eq!(id, "editor_my-range");
        assert_eq!(publish_id(&id), "my-range");
        assert_eq!(local_id(&publish_id(&id)), id);
    }

    #[test]
    fn an_older_save_publishes_without_its_underscores() {
        assert_eq!(publish_id("editor_my_old_range"), "my-old-range");
        assert_eq!(namespace_of("editor_my_old_range"), "my_old_range");
    }

    #[test]
    fn the_sandbox_and_the_base_game_are_not_a_builders_mod() {
        assert_eq!(project_id("Sand box"), Ok("editor_sand-box".to_string()));
        assert_eq!(project_id("SANDBOX!"), Err(NameProblem::Reserved));
        assert_eq!(project_id("base"), Err(NameProblem::Reserved));
        assert_eq!(project_id("!!!"), Err(NameProblem::Unusable));
    }

    #[test]
    fn a_second_scenario_of_one_name_gets_its_own_id() {
        let taken: HashSet<String> = ["mine__range".to_string()].into_iter().collect();

        assert_eq!(scenario_id("mine", "Range", &taken), "mine__range_2");
        assert_eq!(scenario_id("mine", "Arena", &taken), "mine__arena");
    }

    /// `ScenarioConfig::new` holds every scenario id to snake_case, and a mod
    /// name with a dash or a scratch token must still mint one.
    #[test]
    fn a_minted_scenario_id_is_snake_case() {
        let id = scenario_id("red-run", "Free Fire", &HashSet::new());

        assert_eq!(id, "red_run__free_fire");
    }

    #[test]
    fn two_spellings_of_one_publish_id_are_one_name() {
        let known = vec!["editor_a_b".to_string()];

        assert!(is_taken("editor_a-b", &known));
        assert!(!is_taken("editor_a-c", &known));
    }

    #[test]
    fn a_design_id_comes_back_as_its_node() {
        let id = design_id("mine__range", "ship_1");

        assert_eq!(node_of_design("mine__range", &id), "ship_1");
        assert_eq!(node_of_design("mine__range", "ship_1"), "ship_1");
        assert_eq!(node_of_design("mine__other", &id), id);
    }
}
