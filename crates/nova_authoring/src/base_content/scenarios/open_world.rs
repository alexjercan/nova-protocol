//! The New Game harness: the scenario New Game loads onto the generated world.
//!
//! The scenario spawns the player's ship as the start role picked on the New
//! Game screen, and the lights. It declares `world: Generated((seed: None))`,
//! so it takes the seed and the role the player picked and the Scenarios
//! picker renders no row for it. `nova_world_base` sees the world block and
//! the one player ship and streams the seeded sectors in around it.
//!
//! The player starts docked at the session's starting station, the allied
//! station nearest the seed's origin, which `SpawnStartRole` reads from the
//! session. A fresh start posts charter 0's first step, "undock from your
//! starting station". The player's undock from a living station completes
//! it and posts the second step, "dock at another living station": the
//! `OnUndocked` handler filters on the subject's `living_station` type name
//! and on the player as the docking ship. No handler here completes the
//! second step: the station it must not count is the seed's, which no
//! author can name, so `nova_world_base` completes it on the player's dock
//! at a living station other than the starting one and points its marker at
//! the nearest such station. A resumed load posts nothing: the save carries
//! the objectives if they still stand and the spent handlers if they do not.
//! The one outcome is the player's death, a Defeat with nothing queued: the
//! menu offers Load last save in a saved world and Main Menu everywhere.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_scenario::prelude::*;
use nova_world_base::prelude::{
    CHARTER_ZERO_OBJECTIVE_ID, CHARTER_ZERO_UNDOCK_OBJECTIVE_ID, OPEN_WORLD_SCENARIO_ID,
};

use crate::{base_content::assets::BaseContentAssets, scenario_helpers::prelude::entity};

/// The player's ship, by the id events and tools address it with.
const PLAYER_ID: &str = "player";

/// The objective line charter 0's first step shows.
const CHARTER_ZERO_UNDOCK_MESSAGE: &str = "Charter 0: undock from your starting station";

/// The objective line charter 0's second step shows.
const CHARTER_ZERO_MESSAGE: &str = "Charter 0: dock at another living station";

/// The handler that completes charter 0's first step and posts the second:
/// the player undocks from a living station. A `living_station` subject,
/// not any hull, so letting go of a wreck does not.
fn charter_zero_undocked() -> ScenarioEventConfig {
    ScenarioEventConfig {
        label: None,
        name: EventConfig::OnUndocked,
        once: true,
        filters: vec![EventFilterConfig::Entity(EntityFilterConfig {
            type_name: Some(LIVING_STATION_TYPE_NAME.to_string()),
            other_id: Some(PLAYER_ID.to_string()),
            ..Default::default()
        })],
        actions: vec![
            EventActionConfig::ObjectiveComplete(ObjectiveCompleteActionConfig {
                id: CHARTER_ZERO_UNDOCK_OBJECTIVE_ID.to_string(),
            }),
            EventActionConfig::Objective(ObjectiveActionConfig {
                id: CHARTER_ZERO_OBJECTIVE_ID.to_string(),
                message: CHARTER_ZERO_MESSAGE.to_string(),
            }),
        ],
    }
}

/// The open-world bootstrap scenario.
pub(crate) fn open_world(assets: &BaseContentAssets) -> ScenarioConfig {
    let mut start = vec![EventActionConfig::SpawnStartRole(StartRoleSpawnConfig {
        id: PLAYER_ID.to_string(),
    })];
    // Directional: the rig lights the whole world, wherever the start is.
    start.extend(ThreePointRig::around(OPEN_WORLD_SCENARIO_ID, Meters3::ZERO, 25.0).actions());
    let mut fresh_start = start.clone();
    fresh_start.push(EventActionConfig::Objective(ObjectiveActionConfig {
        id: CHARTER_ZERO_UNDOCK_OBJECTIVE_ID.to_string(),
        message: CHARTER_ZERO_UNDOCK_MESSAGE.to_string(),
    }));

    ScenarioConfig {
        description: "An open world generated from your seed: sparse asteroid clusters, \
                      planetoids and generated ships, streamed in around your ship as you fly."
            .to_string(),
        world: ScenarioWorldType::Generated(GeneratedWorldConfig {
            seed: None,
            bounds: None,
        }),
        events: vec![
            // One spawn list for both openings: a fresh start and a resumed
            // load put the same ship and the same lights in the world, and
            // the save's ship state lands on the ship once it stands. Only
            // the fresh start posts charter 0: a load restores the saved
            // objectives.
            ScenarioEventConfig {
                label: None,
                name: EventConfig::OnStart,
                once: true,
                filters: vec![],
                actions: fresh_start,
            },
            ScenarioEventConfig {
                label: None,
                name: EventConfig::OnResume,
                once: true,
                filters: vec![],
                actions: start,
            },
            charter_zero_undocked(),
            ScenarioEventConfig {
                label: None,
                name: EventConfig::OnDestroyed,
                once: true,
                filters: vec![entity(PLAYER_ID)],
                actions: vec![EventActionConfig::Outcome(OutcomeActionConfig::new(
                    ScenarioOutcomeKind::Defeat,
                    "Your ship was destroyed.",
                ))],
            },
        ],
        ..ScenarioConfig::new(OPEN_WORLD_SCENARIO_ID, "Open World", assets.cubemap.clone())
    }
}

#[cfg(test)]
mod tests {
    use nova_events::prelude::{GameEventsPlugin, OnUndockedEvent};
    use nova_gameplay::prelude::GameObjectives;

    use super::*;

    /// A headless event world holding charter 0's first step and the
    /// harness's `OnUndocked` handler, warmed one frame.
    fn charter_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.world_mut()
            .resource_mut::<NovaEventWorld>()
            .push_objective(ObjectiveActionConfig {
                id: CHARTER_ZERO_UNDOCK_OBJECTIVE_ID.to_string(),
                message: CHARTER_ZERO_UNDOCK_MESSAGE.to_string(),
            });
        let handler = charter_zero_undocked().build_handler();
        app.world_mut().spawn(handler);
        app.update();
        app
    }

    fn undock(app: &mut App, partner_type_name: &str, docking_ship: &str) {
        let info = DockingEventInfo {
            id: "sector_1_0_0_cluster_1_0_0_station_0".to_string(),
            type_name: partner_type_name.to_string(),
            other_id: docking_ship.to_string(),
            other_type_name: SPACESHIP_TYPE_NAME.to_string(),
        };
        app.world_mut().commands().fire::<OnUndockedEvent>(info);
        app.update();
        app.update();
    }

    fn listed(app: &App, id: &str) -> bool {
        app.world()
            .resource::<NovaEventWorld>()
            .objectives()
            .iter()
            .any(|objective| objective.id == id)
    }

    /// Charter 0's first step means the player's undock from a living
    /// station: letting go of a wreck, and another ship's undock, leave it
    /// standing. The player's undock completes it and posts the second
    /// step.
    #[test]
    fn only_the_players_undock_from_a_living_station_posts_charter_zeros_second_step() {
        let mut app = charter_app();
        assert!(
            listed(&app, CHARTER_ZERO_UNDOCK_OBJECTIVE_ID),
            "a fresh start lists the undock step"
        );
        assert!(
            !listed(&app, CHARTER_ZERO_OBJECTIVE_ID),
            "the dock step waits for the undock"
        );

        undock(&mut app, SPACESHIP_TYPE_NAME, PLAYER_ID);
        assert!(
            listed(&app, CHARTER_ZERO_UNDOCK_OBJECTIVE_ID),
            "letting go of a wreck does not complete it"
        );

        undock(&mut app, LIVING_STATION_TYPE_NAME, "trader");
        assert!(
            listed(&app, CHARTER_ZERO_UNDOCK_OBJECTIVE_ID),
            "another ship's undock does not complete it"
        );

        undock(&mut app, LIVING_STATION_TYPE_NAME, PLAYER_ID);
        assert!(
            !listed(&app, CHARTER_ZERO_UNDOCK_OBJECTIVE_ID),
            "the player's undock from a living station completes it"
        );
        assert!(
            listed(&app, CHARTER_ZERO_OBJECTIVE_ID),
            "and posts the dock step"
        );
    }
}
