//! The shakedown script's pins: static cross-checks over the built config
//! ([`pins`]) and a scripted `App` that walks the five beats end to end
//! ([`walk`]).

mod pins;
mod walk;

use super::*;

fn scenario() -> ScenarioConfig {
    shakedown_run(AssetRef::default(), AssetRef::default())
}

/// Every action across all handlers, flattened.
fn all_actions(config: &ScenarioConfig) -> impl Iterator<Item = &EventActionConfig> {
    config.events.iter().flat_map(|event| event.actions.iter())
}
