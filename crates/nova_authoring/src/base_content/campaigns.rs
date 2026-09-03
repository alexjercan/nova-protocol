//! Built-in campaign inventory.

use nova_scenario::prelude::CampaignConfig;

use super::scenarios::nova_protocol::{FIRST_SHIFT_SCENARIO_ID, SECOND_SHIFT_SCENARIO_ID};

/// Every built-in campaign in stable generated-content order.
pub(crate) fn catalog() -> Vec<CampaignConfig> {
    vec![CampaignConfig {
        id: "nova_protocol".to_string(),
        name: "Nova Protocol".to_string(),
        scenarios: vec![
            FIRST_SHIFT_SCENARIO_ID.to_string(),
            SECOND_SHIFT_SCENARIO_ID.to_string(),
        ],
    }]
}
