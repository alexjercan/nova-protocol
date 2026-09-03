//! End to end over the REAL generated base content: the campaign membership
//! contract. It lives here rather than in `nova_assets` because its input is
//! the authoring toolchain's own generators, not a synthetic fixture.

use nova_assets::merge_bundles;
use nova_authoring::generation;
use nova_modding::prelude::Content;

/// Merge the built-in scenarios and the built-in campaign, then resolve the
/// "nova_protocol" campaign's membership. It must list its chapters in play
/// order, and every member must resolve to a merged scenario. This is the
/// "real mapping, not display-name parsing" contract: the order comes from the
/// campaign's own list, not from anything a title happens to say.
#[test]
fn merged_campaign_resolves_members_in_play_order() {
    let mut items: Vec<Content> = Vec::new();
    for (_, content) in generation::build_scenario_contents() {
        items.extend(content);
    }
    for (_, content) in generation::build_campaign_contents() {
        items.extend(content);
    }

    let outcome = merge_bundles([items.iter()]);
    assert!(
        outcome.conflicts.is_empty(),
        "base content merges clean: {:?}",
        outcome.conflicts
    );

    let campaign = outcome
        .campaigns
        .get("nova_protocol")
        .expect("the base Nova Protocol campaign registers");
    assert_eq!(
        campaign.scenarios,
        vec!["first_shift", "second_shift"],
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

    // Chapter one is the New Game entry and chapter two is chained from it, so
    // BOTH are visible in the flat picker: nothing in this campaign is
    // reachable only through the campaign mapping.
    for member in &campaign.scenarios {
        assert!(
            !outcome.scenarios[member].hidden,
            "'{member}' is a listed chapter, not a hidden chained wave"
        );
    }
}
