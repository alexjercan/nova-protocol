//! Hidden relay back into the asteroid-field sandbox.

use bevy::prelude::Image;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

pub(crate) fn asteroid_next(cubemap: AssetRef<Image>) -> ScenarioConfig {
    let events = vec![ScenarioEventConfig {
        name: EventConfig::OnStart,
        filters: vec![],
        // Non-lingering cut: this relay carries no Outcome overlay, so a
        // lingering switch would strand the player in an empty scenario until a
        // stray Enter press. linger: false makes it an immediate cut back into
        // the field - one acknowledgement, seamless loop.
        actions: vec![EventActionConfig::NextScenario(NextScenarioActionConfig {
            scenario_id: "asteroid_field".to_string(),
            linger: false,
            delay: None,
        })],
    }];

    ScenarioConfig {
        description: "The next scenario after the asteroid field.".to_string(),
        // A continuation reached only via NextScenario chaining, not an entry point.
        hidden: true,
        events,
        ..ScenarioConfig::new(
            "asteroid_next".to_string(),
            "Asteroid Field - Next".to_string(),
            cubemap,
        )
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// The asteroid_next relay is a pure OnStart bridge with no Outcome
    /// overlay, so its switch must be a non-lingering cut: a lingering switch
    /// here would strand the player in this empty scenario until a stray Enter
    /// press. Mirrors the filter-events shape of shakedown's
    /// player_death_routes_back test.
    #[test]
    fn asteroid_next_bridge_is_a_non_lingering_cut() {
        let config = asteroid_next(AssetRef::default());

        let bridges: Vec<&NextScenarioActionConfig> = config
            .events
            .iter()
            .filter(|event| matches!(event.name, EventConfig::OnStart))
            .flat_map(|event| event.actions.iter())
            .filter_map(|action| match action {
                EventActionConfig::NextScenario(next) => Some(next),
                _ => None,
            })
            .collect();

        assert_eq!(bridges.len(), 1, "the relay has exactly one OnStart cut");
        assert_eq!(bridges[0].scenario_id, "asteroid_field");
        assert!(
            !bridges[0].linger,
            "a bare relay with no Outcome overlay must cut immediately, not linger"
        );
    }
}
