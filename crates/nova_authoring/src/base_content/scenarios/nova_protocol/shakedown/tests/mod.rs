//! The shakedown script's pins: static cross-checks over the built config
//! ([`pins`]) and a scripted `App` that walks the five beats end to end
//! ([`walk`]).

mod pins;
mod walk;

use super::*;

fn scenario() -> ScenarioConfig {
    shakedown_run(AssetRef::default(), AssetRef::default())
}

/// Every action across all handlers, flattened - including the actions a
/// `Sequence` step carries, which are as much part of the script as the frame
/// that queued them.
fn all_actions(config: &ScenarioConfig) -> Vec<&EventActionConfig> {
    let mut out = Vec::new();
    for action in config.events.iter().flat_map(|event| event.actions.iter()) {
        action.walk(&mut |action| out.push(action));
    }
    out
}
