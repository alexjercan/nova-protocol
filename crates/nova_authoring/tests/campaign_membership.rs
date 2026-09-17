//! End to end over the REAL generated content: the campaign membership
//! contract. It lives here rather than in `nova_assets` because half its input
//! is the authoring toolchain's own generators, not a synthetic fixture.
//!
//! The base game ships ONE campaign now - season one, the mainline story - and
//! a mod may declare its own over the same scenarios. Both shapes are checked
//! here: the shipped list must resolve, and a mod-declared list must merge over
//! base without a conflict.

use nova_assets::merge_bundles;
use nova_authoring::generation;
use nova_modding::prelude::Content;
use nova_scenario::prelude::CampaignConfig;

/// The base campaign and the chapter it opens on.
const CAMPAIGN: &str = "season_one";
const CHAPTER_ONE: &str = "season_one_chapter_one";

/// The scenario a MOD's campaign is declared over: the training range New Game
/// starts, which is base content and not a member of the base campaign.
const MEMBER: &str = "tutorial";

/// Base's own content, merged the way the game merges it.
fn base_content() -> Vec<Content> {
    let mut base: Vec<Content> = Vec::new();
    for (_, content) in generation::build_scenario_contents() {
        base.extend(content);
    }
    base.extend(generation::build_campaign_content());
    base
}

/// The shipped campaign resolves: it is registered under its own id, its
/// members are real scenarios, and each one is a launchable chapter.
#[test]
fn the_shipped_campaign_resolves_its_chapters() {
    let base = base_content();
    let outcome = merge_bundles([base.iter()]);
    assert!(
        outcome.conflicts.is_empty(),
        "base content merges clean: {:?}",
        outcome.conflicts
    );

    let campaign = outcome
        .campaigns
        .get(CAMPAIGN)
        .expect("the base campaign registers");
    assert_eq!(
        campaign.scenarios,
        vec![CHAPTER_ONE],
        "the season lists its chapters in play order"
    );

    for member in &campaign.scenarios {
        let scenario = outcome
            .scenarios
            .get(member)
            .unwrap_or_else(|| panic!("campaign member '{member}' resolves to a real scenario"));
        // A backdrop is scenery that poses its own camera and hands the player
        // no ship, and a practice range belongs to the lesson that names it.
        // Either is a lint Error in a campaign, and the picker would render no
        // row for one.
        assert!(
            scenario.role.picker_lists(),
            "'{member}' is a launchable chapter, not a backdrop or a practice range"
        );
    }
}

/// Merge the base content and a mod-style campaign over it, then resolve the
/// mod campaign's membership. It must list its chapters in declared play order,
/// and every member must resolve to a merged scenario. This is the "real
/// mapping, not display-name parsing" contract: the order comes from the
/// campaign's own list, not from anything a title happens to say.
#[test]
fn a_merged_mod_campaign_resolves_members_in_play_order() {
    let base = base_content();
    let story = [Content::Campaign(CampaignConfig {
        id: "campaign_under_test".to_string(),
        name: "Campaign Under Test".to_string(),
        scenarios: vec![MEMBER.to_string()],
    })];

    let outcome = merge_bundles([base.iter(), story.iter()]);
    assert!(
        outcome.conflicts.is_empty(),
        "base and campaign content merge clean: {:?}",
        outcome.conflicts
    );

    let campaign = outcome
        .campaigns
        .get("campaign_under_test")
        .expect("the campaign registers");
    assert_eq!(
        campaign.scenarios,
        vec![MEMBER],
        "members resolve in the campaign's declared play order"
    );
    assert!(
        outcome.scenarios.contains_key(MEMBER),
        "campaign member '{MEMBER}' resolves to a real scenario"
    );
    assert!(
        outcome.campaigns.contains_key(CAMPAIGN),
        "a mod's campaign joins the base campaign rather than replacing it"
    );
}

/// The campaign ships as ONE generated file, beside the scenarios it names.
#[test]
fn the_campaign_ships_as_one_generated_file() {
    let campaigns: Vec<_> = generation::content_files()
        .into_iter()
        .filter(|(rel, _)| rel.contains("/campaigns/"))
        .map(|(rel, _)| rel)
        .collect();
    assert_eq!(campaigns, vec!["base/campaigns/base.content.ron"]);
    assert!(
        generation::build_scenarios()
            .iter()
            .any(|scenario| scenario.id == CHAPTER_ONE),
        "the campaign's first chapter is base content"
    );
}
