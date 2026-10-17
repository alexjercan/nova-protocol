//! The New Game harness: the scenario New Game loads onto the generated world.
//!
//! The scenario spawns the player's ship as the start role picked on the New
//! Game screen, and the lights. It declares `world: Generated((seed: None))`,
//! so it takes the seed and the role the player picked and the Scenarios
//! picker renders no row for it. `nova_world_base` sees the world block and
//! the one player ship and streams the seeded sectors in around it.
//!
//! A fresh start posts charter 0, "dock at the nearest station", and the
//! world crate points its marker at that station. The player's first dock at
//! a living station completes it: the `OnDocked` handler filters on the
//! subject's `living_station` type name, so a derelict station or a wreck
//! does not, and on the player as the docking ship. A resumed load posts
//! nothing: the save carries the objective if it still stands and the spent
//! handlers if it does not. The one outcome is the player's death, a Defeat
//! with nothing queued: the menu offers Load last save in a saved world and
//! Main Menu everywhere.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_scenario::prelude::*;
use nova_world_base::prelude::{CHARTER_ZERO_OBJECTIVE_ID, OPEN_WORLD_SCENARIO_ID};

use crate::{base_content::assets::BaseContentAssets, scenario_helpers::prelude::entity};

/// The player's ship, by the id events and tools address it with.
const PLAYER_ID: &str = "player";

/// The objective line charter 0 shows.
const CHARTER_ZERO_MESSAGE: &str = "Charter 0: dock at the nearest station";

/// The handler that completes charter 0: the player docks with a living
/// station. A `living_station` subject, not any hull or any station, so a
/// dead station and a wreck do not complete it.
fn charter_zero_docked() -> ScenarioEventConfig {
    ScenarioEventConfig {
        label: None,
        name: EventConfig::OnDocked,
        once: true,
        filters: vec![EventFilterConfig::Entity(EntityFilterConfig {
            type_name: Some(LIVING_STATION_TYPE_NAME.to_string()),
            other_id: Some(PLAYER_ID.to_string()),
            ..Default::default()
        })],
        actions: vec![EventActionConfig::ObjectiveComplete(
            ObjectiveCompleteActionConfig {
                id: CHARTER_ZERO_OBJECTIVE_ID.to_string(),
            },
        )],
    }
}

/// The open-world bootstrap scenario.
pub(crate) fn open_world(assets: &BaseContentAssets) -> ScenarioConfig {
    let mut start = vec![EventActionConfig::SpawnStartRole(StartRoleSpawnConfig {
        id: PLAYER_ID.to_string(),
        position: Meters3::ZERO,
        rotation: Quat::IDENTITY,
    })];
    start.extend(ThreePointRig::around(OPEN_WORLD_SCENARIO_ID, Meters3::ZERO, 25.0).actions());
    let mut fresh_start = start.clone();
    fresh_start.push(EventActionConfig::Objective(ObjectiveActionConfig {
        id: CHARTER_ZERO_OBJECTIVE_ID.to_string(),
        message: CHARTER_ZERO_MESSAGE.to_string(),
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
            // load put the same ship and the same lights at the origin, and
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
            charter_zero_docked(),
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
    use nova_events::prelude::{GameEventsPlugin, OnDockedEvent};
    use nova_gameplay::prelude::GameObjectives;

    use super::*;

    /// A headless event world holding charter 0 and the harness's `OnDocked`
    /// handler, warmed one frame.
    fn charter_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.world_mut()
            .resource_mut::<NovaEventWorld>()
            .push_objective(ObjectiveActionConfig {
                id: CHARTER_ZERO_OBJECTIVE_ID.to_string(),
                message: CHARTER_ZERO_MESSAGE.to_string(),
            });
        let handler = charter_zero_docked().build_handler();
        app.world_mut().spawn(handler);
        app.update();
        app
    }

    fn dock(app: &mut App, station_type_name: &str, docking_ship: &str) {
        let info = DockingEventInfo {
            id: "sector_0_0_0_cluster_0_0_0_station_0".to_string(),
            type_name: station_type_name.to_string(),
            other_id: docking_ship.to_string(),
            other_type_name: SPACESHIP_TYPE_NAME.to_string(),
        };
        app.world_mut().commands().fire::<OnDockedEvent>(info);
        app.update();
        app.update();
    }

    fn charter_stands(app: &App) -> bool {
        app.world()
            .resource::<NovaEventWorld>()
            .objectives()
            .iter()
            .any(|objective| objective.id == CHARTER_ZERO_OBJECTIVE_ID)
    }

    /// Charter 0 means a living station: the player's dock with a dead
    /// station or a wreck leaves it standing, and a dock by another ship
    /// at a living station does too. The player's dock with a living
    /// station completes it.
    #[test]
    fn only_the_players_dock_at_a_living_station_completes_charter_zero() {
        let mut app = charter_app();
        assert!(charter_stands(&app), "a fresh start lists charter 0");

        dock(&mut app, DERELICT_STATION_TYPE_NAME, PLAYER_ID);
        assert!(charter_stands(&app), "a dead station does not complete it");

        dock(&mut app, SPACESHIP_TYPE_NAME, PLAYER_ID);
        assert!(charter_stands(&app), "a wreck does not complete it");

        dock(&mut app, LIVING_STATION_TYPE_NAME, "trader");
        assert!(
            charter_stands(&app),
            "another ship's dock does not complete it"
        );

        dock(&mut app, LIVING_STATION_TYPE_NAME, PLAYER_ID);
        assert!(
            !charter_stands(&app),
            "the player's dock at a living station completes it"
        );
    }
}
