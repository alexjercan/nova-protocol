//! The temporary navigation MARK every scripted scenario navigates by.
//!
//! A mark is a lit beacon, an arrival gate on the same place, and the HUD chip
//! that points at it - three objects a script raises together and takes down
//! together. Shared furniture rather than the training range's own, because
//! the range, the handbook's practice drills and the campaign's chapters all
//! send a ship to a place and want to hear when it arrives.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_scenario::prelude::*;

use crate::scenario_helpers::prelude::*;

/// The beacon ink a mark is drawn in.
const MARK_COLOR: Color = Color::srgb(0.3, 0.9, 1.0);

/// One temporary navigation mark: a lit beacon the script puts up for one
/// beat and takes down again.
///
/// Taking it down means DESPAWN, not just dropping the HUD chip: a finished
/// mark left burning is a mark the player keeps flying to.
pub(crate) struct Mark {
    /// Scenario id, and the id the marker and despawn are addressed to.
    pub(crate) id: &'static str,
    /// What the beacon and its HUD chip read.
    pub(crate) label: &'static str,
    pub(crate) position: Meters3,
    /// Trigger volume. A hand-flown mark wants a tight one.
    pub(crate) area: Meters,
}

impl Mark {
    /// Put the mark up and point the HUD at it.
    pub(crate) fn raise(&self) -> Vec<EventActionConfig> {
        vec![
            spawn_object(ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: self.id.to_string(),
                    name: self.label.to_string(),
                    position: self.position,
                    rotation: Quat::IDENTITY,
                },
                kind: ScenarioObjectKind::Beacon(BeaconConfig {
                    label: self.label.to_string(),
                    radius: Meters(20.0),
                    color: MARK_COLOR,
                    area_radius: Some(self.area),
                    lock_signature: None,
                }),
            }),
            attach_objective_marker(self.id, self.label),
        ]
    }

    /// The id of this mark's separate ARRIVAL GATE.
    pub(crate) fn gate_id(&self) -> String {
        format!("{}_gate", self.id)
    }

    /// Raise the arrival gate: a trigger volume on the mark's own place, put
    /// up in the same step as the card that names the beat. The gate, not the
    /// beacon, decides the beat, so a handler armed for it can never be spent
    /// before its card exists.
    pub(crate) fn raise_gate(&self) -> EventActionConfig {
        EventActionConfig::CreateScenarioArea(ScenarioAreaConfig {
            id: self.gate_id(),
            name: format!("{} Gate", self.label),
            position: self.position,
            rotation: Quat::IDENTITY,
            radius: self.area,
        })
    }

    /// OnEnter of this mark's arrival gate by one named ship.
    pub(crate) fn entered_by(&self, ship: &str) -> EventFilterConfig {
        entity_pair(self.gate_id(), ship)
    }

    /// Take the mark and its gate down: the chip first, then both bodies.
    pub(crate) fn clear(&self) -> Vec<EventActionConfig> {
        vec![
            detach_objective_marker(self.id),
            despawn_object(self.id),
            despawn_object(self.gate_id()),
        ]
    }
}
