//! What the live world can complete.
//!
//! The catalog says an argument is a ship or a section
//! ([`CommandArg::Live`](nova_os::prelude::CommandArg::Live)); only the world
//! knows which ones exist. This publishes the values under the tokens the
//! catalog names, and the terminal's Tab does the rest - so a new arg-bearing
//! command completes the moment it declares what its argument is.

use bevy::prelude::*;
use nova_events::prelude::EntityId;
use nova_gameplay::prelude::*;
use nova_input::prelude::*;
use nova_os::prelude::*;
use nova_scenario::prelude::*;

/// Publish every live set the command catalog can complete against.
///
/// Runs only while the CRT is up, and [`NovaOsTerminal::merge_live_values`]
/// writes only on a real change, so a world that did not move does not flag the
/// terminal as changed.
pub(crate) fn publish_live_values(
    mut terminal: ResMut<NovaOsTerminal>,
    mut published_ships: Local<Vec<String>>,
    q_ships: Query<(Entity, &EntityId), With<SpaceshipRootMarker>>,
    q_sections: Query<(&ChildOf, &EntityId), With<SectionMarker>>,
    scenarios: Option<Res<GameScenarios>>,
    events: Option<Res<NovaEventWorld>>,
    bindings: Option<Res<InputBindings>>,
) {
    let mut ships: Vec<(Entity, String)> = q_ships
        .iter()
        .map(|(entity, id)| (entity, id.0.clone()))
        .collect();
    ships.sort_by(|left, right| left.1.cmp(&right.1));

    let mut entries: Vec<(String, Vec<String>)> = vec![(
        live::SHIP.to_string(),
        ships.iter().map(|(_, id)| id.clone()).collect(),
    )];
    // Section ids are per hull, so they are published UNDER their ship: two
    // gunships both carry `pdc_aft_port`, and `section block_raider <TAB>` has
    // to offer that hull's, not the union.
    for (entity, id) in &ships {
        let mut codes: Vec<String> = q_sections
            .iter()
            .filter(|(child_of, _)| child_of.parent() == *entity)
            .map(|(_, code)| code.0.clone())
            .collect();
        codes.sort_unstable();
        entries.push((format!("{}:{id}", live::SECTION), codes));
    }
    // A ship that died keeps its key, so empty it: `merge_live_values` only
    // ever adds, and a dead hull must stop completing.
    let live_ids: Vec<String> = ships.into_iter().map(|(_, id)| id).collect();
    for gone in published_ships.iter().filter(|id| !live_ids.contains(id)) {
        entries.push((format!("{}:{gone}", live::SECTION), Vec::new()));
    }
    *published_ships = live_ids;

    if let Some(scenarios) = scenarios {
        let mut ids: Vec<String> = scenarios.keys().cloned().collect();
        ids.sort_unstable();
        entries.push((live::SCENARIO.to_string(), ids));
    }
    if let Some(events) = events {
        let mut names: Vec<String> = events.variables().map(|(key, _)| key.clone()).collect();
        names.sort_unstable();
        entries.push((live::VARIABLE.to_string(), names));
    }
    if let Some(bindings) = bindings {
        let mut names: Vec<String> = bindings
            .iter()
            .map(|action| action.name.to_string())
            .collect();
        names.sort_unstable();
        entries.push((live::ACTION.to_string(), names));
        // Every source `bind` accepts, spelled the way the game prints it -
        // which is one of the spellings `InputSource::parse` reads back.
        entries.push((
            live::SOURCE.to_string(),
            InputSource::bindable()
                .map(|source| source.label())
                .collect(),
        ));
    }
    terminal.merge_live_values(entries);
}
