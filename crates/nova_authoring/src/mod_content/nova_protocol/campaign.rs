//! The story mod's campaign inventory.

use nova_scenario::prelude::CampaignConfig;

use super::{first_shift::FIRST_SHIFT_SCENARIO_ID, MOD_ID};

/// Every campaign the story mod ships, in stable generated-content order.
///
/// One so far: "Nova Protocol", listing its chapters in play order. The
/// member ids reference the scenario-id constants so a scenario rename cannot
/// silently orphan a member.
pub(crate) fn catalog() -> Vec<CampaignConfig> {
    vec![CampaignConfig {
        id: MOD_ID.to_string(),
        name: "Nova Protocol".to_string(),
        scenarios: vec![FIRST_SHIFT_SCENARIO_ID.to_string()],
    }]
}
