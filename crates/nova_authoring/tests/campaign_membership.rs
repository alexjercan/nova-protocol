//! End to end over the REAL generated base content: the campaign membership
//! contract. It lives here rather than in `nova_assets` because its input is
//! the authoring toolchain's own generators, not a synthetic fixture.

use nova_assets::merge_bundles;
use nova_authoring::scenario_generation;
use nova_modding::prelude::Content;

/// Merge the built-in scenarios and the built-in campaign, then resolve the
/// "nova_protocol" campaign's membership. It must list its five chapters in
/// play order - including the two `hidden` chained members (broadside_gunship,
/// the phase-two wave; final_tally, the epilogue) - and every member must
/// resolve to a merged scenario, hidden ones included. This is the "real
/// mapping, not display-name parsing" contract: the order comes from the
/// campaign's own list, and a hidden member is reachable despite being filtered
/// from the flat picker.
#[test]
fn merged_campaign_resolves_members_in_order_including_hidden() {
    let mut items: Vec<Content> = Vec::new();
    for (_, content) in scenario_generation::build_scenario_contents() {
        items.extend(content);
    }
    for (_, content) in scenario_generation::build_campaign_contents() {
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
        vec![
            "shakedown_run",
            "broadside",
            "broadside_gunship",
            "lifeline",
            "final_tally",
        ],
        "members resolve in the campaign's declared play order"
    );

    // Every member resolves to a merged scenario - the mapping never lists a
    // ghost - and the two chained chapters are genuinely `hidden` (so they are
    // reachable ONLY via this mapping, not the flat picker).
    for member in &campaign.scenarios {
        assert!(
            outcome.scenarios.contains_key(member),
            "campaign member '{member}' resolves to a real scenario"
        );
    }
    for hidden_member in ["broadside_gunship", "final_tally"] {
        assert!(
            outcome.scenarios[hidden_member].hidden,
            "'{hidden_member}' is hidden from the flat picker yet listed for replay"
        );
    }
}
