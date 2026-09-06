//! End to end over the REAL generated content: the campaign membership
//! contract. It lives here rather than in `nova_assets` because its input is
//! the authoring toolchain's own generators, not a synthetic fixture.

use nova_assets::merge_bundles;
use nova_authoring::generation;
use nova_modding::prelude::Content;

/// Merge the base scenarios, the story mod's scenarios and its campaign, then
/// resolve the "nova_protocol" campaign's membership. It must list its
/// chapters in play order, and every member must resolve to a merged
/// scenario. This is the "real mapping, not display-name parsing" contract:
/// the order comes from the campaign's own list, not from anything a title
/// happens to say.
#[test]
fn merged_campaign_resolves_members_in_play_order() {
    let mut base: Vec<Content> = Vec::new();
    for (_, content) in generation::build_scenario_contents() {
        base.extend(content);
    }
    let mut story: Vec<Content> = Vec::new();
    for (_, content) in generation::build_story_scenario_contents() {
        story.extend(content);
    }
    for (_, content) in generation::build_campaign_contents() {
        story.extend(content);
    }

    let outcome = merge_bundles([base.iter(), story.iter()]);
    assert!(
        outcome.conflicts.is_empty(),
        "base and story content merge clean: {:?}",
        outcome.conflicts
    );

    let campaign = outcome
        .campaigns
        .get("nova_protocol")
        .expect("the Nova Protocol campaign registers");
    assert_eq!(
        campaign.scenarios,
        vec!["first_shift"],
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

    // Every listed chapter is visible in the flat picker too: nothing in this
    // campaign is reachable only through the campaign mapping. A chapter chained
    // from another still gets its own row.
    for member in &campaign.scenarios {
        assert!(
            !outcome.scenarios[member].hidden,
            "'{member}' is a listed chapter, not a hidden chained wave"
        );
    }
}

/// The base game ships no campaign of its own: New Game starts the training
/// range, and the story is the mod's to list.
#[test]
fn the_base_game_ships_no_campaign() {
    let base_campaigns = generation::content_files()
        .into_iter()
        .filter(|(rel, _)| rel.starts_with("base/campaigns/"))
        .count();
    assert_eq!(base_campaigns, 0);
    assert!(
        generation::build_scenarios()
            .iter()
            .any(|scenario| scenario.id == "tutorial"),
        "the training range is base content"
    );
}
