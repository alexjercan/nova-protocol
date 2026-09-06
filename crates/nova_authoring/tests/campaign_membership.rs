//! End to end over the REAL generated content: the campaign membership
//! contract. It lives here rather than in `nova_assets` because half its input
//! is the authoring toolchain's own generators, not a synthetic fixture.
//!
//! Nothing SHIPS a campaign today - the base game never has, and the story mod
//! that did is gone while the writing is redone. The campaign format is still
//! a mod's to use, so the campaign here is declared the way a mod declares one,
//! over generated scenarios that really exist.

use nova_assets::merge_bundles;
use nova_authoring::generation;
use nova_modding::prelude::Content;
use nova_scenario::prelude::CampaignConfig;

/// The scenario a campaign's one visible member resolves to: the training
/// range New Game starts.
const MEMBER: &str = "tutorial";

/// Merge the base scenarios and a mod-style campaign over them, then resolve
/// the campaign's membership. It must list its chapters in declared play
/// order, and every member must resolve to a merged scenario. This is the
/// "real mapping, not display-name parsing" contract: the order comes from the
/// campaign's own list, not from anything a title happens to say.
#[test]
fn merged_campaign_resolves_members_in_play_order() {
    let mut base: Vec<Content> = Vec::new();
    for (_, content) in generation::build_scenario_contents() {
        base.extend(content);
    }
    let story = vec![Content::Campaign(CampaignConfig {
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

    // Every member resolves to a merged scenario - the mapping never lists a
    // ghost.
    for member in &campaign.scenarios {
        assert!(
            outcome.scenarios.contains_key(member),
            "campaign member '{member}' resolves to a real scenario"
        );
    }

    // Every listed chapter is visible in the flat picker too: nothing in a
    // campaign is reachable only through the campaign mapping. A chapter
    // chained from another still gets its own row.
    for member in &campaign.scenarios {
        assert!(
            !outcome.scenarios[member].hidden,
            "'{member}' is a listed chapter, not a hidden chained wave"
        );
    }
}

/// The game ships no campaign of its own: New Game starts the training range,
/// and a campaign is a mod's to list.
#[test]
fn the_game_ships_no_campaign() {
    let campaigns = generation::content_files()
        .into_iter()
        .filter(|(rel, _)| rel.contains("/campaigns/"))
        .count();
    assert_eq!(campaigns, 0);
    assert!(
        generation::build_scenarios()
            .iter()
            .any(|scenario| scenario.id == MEMBER),
        "the training range is base content"
    );
}
