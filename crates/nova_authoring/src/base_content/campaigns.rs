//! The base game's campaigns: the named runs of chapters the Scenarios board
//! groups under one header.
//!
//! A campaign is a LIST, not a container. Every chapter it names is also an
//! ordinary picker row that can be launched on its own; what the campaign adds
//! is the order they are meant to be flown in, and the header they are drawn
//! under. Only `role: Chapter` scenarios may be listed - lint refuses a
//! backdrop or a practice range here - so this file is the one place that says
//! which built-in scenarios are STORY.

use nova_scenario::prelude::CampaignConfig;

use super::scenarios::season_one;

/// The campaign id referenced by its chapter scenarios.
pub(crate) const SEASON_ONE_CAMPAIGN_ID: &str = "season_one";

/// The Scenarios board heading for this campaign.
const SEASON_ONE_NAME: &str = "Season 1";

/// Every built-in campaign, in stable generated-content order.
pub(crate) fn campaign_catalog() -> Vec<CampaignConfig> {
    vec![CampaignConfig {
        id: SEASON_ONE_CAMPAIGN_ID.to_string(),
        name: SEASON_ONE_NAME.to_string(),
        scenarios: vec![season_one::CHAPTER_ONE_SCENARIO_ID.to_string()],
    }]
}
