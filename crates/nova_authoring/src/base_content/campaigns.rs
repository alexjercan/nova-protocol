//! Built-in campaign inventory.

use nova_scenario::prelude::CampaignConfig;

use super::scenarios::nova_protocol::{
    BROADSIDE_GUNSHIP_SCENARIO_ID, BROADSIDE_SCENARIO_ID, FINAL_TALLY_SCENARIO_ID,
    LIFELINE_SCENARIO_ID, SHAKEDOWN_SCENARIO_ID,
};

/// Every built-in campaign in stable generated-content order.
pub(crate) fn catalog() -> Vec<CampaignConfig> {
    vec![CampaignConfig {
        id: "nova_protocol".to_string(),
        name: "Nova Protocol".to_string(),
        scenarios: vec![
            SHAKEDOWN_SCENARIO_ID.to_string(),
            BROADSIDE_SCENARIO_ID.to_string(),
            BROADSIDE_GUNSHIP_SCENARIO_ID.to_string(),
            LIFELINE_SCENARIO_ID.to_string(),
            FINAL_TALLY_SCENARIO_ID.to_string(),
        ],
    }]
}
