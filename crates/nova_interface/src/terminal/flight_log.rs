//! The combined flight-log model, derived from the story feed, the active
//! objective list, and what the ship's own systems report.
//!
//! Nothing here paints rows. The log reaches the player through the `log`
//! command in the command modal.
//!
//! Touch this module when changing how logged events are recorded.

use bevy::prelude::*;
use nova_gameplay::objectives::{GameObjectives, Objective};
use nova_hud::prelude::*;
use nova_ship::prelude::{CombatLockDrop, CombatLockDropped};

use super::components::*;

/// Update the NOVA OS's combined flight log from the story feed and active
/// objective list.
pub(crate) fn sync_nova_os_logs(
    story: Res<StoryFeed>,
    objectives: Res<GameObjectives>,
    mut log: ResMut<NovaOsFlightLog>,
) {
    if story.0.len() < log.seen_story {
        log.clear();
    }

    for line in story.0.iter().skip(log.seen_story) {
        log.entries.push(NovaOsFlightLogEntry {
            kind: NovaOsFlightLogEntryKind::Comms,
            objective_id: None,
            speaker: Some(line.speaker.clone()),
            message: line.text.clone(),
            icon: line.icon.clone(),
        });
    }
    log.seen_story = story.0.len();

    let completed: Vec<Objective> = log
        .previous_active
        .iter()
        .filter(|old| {
            !objectives
                .objectives
                .iter()
                .any(|current| current.id == old.id)
        })
        .cloned()
        .collect();
    for objective in completed {
        log.entries.push(NovaOsFlightLogEntry {
            kind: NovaOsFlightLogEntryKind::ObjectiveCompleted,
            objective_id: Some(objective.id.clone()),
            speaker: None,
            message: objective.message.clone(),
            icon: None,
        });
        log.active_objective_entries
            .retain(|entry| entry.id != objective.id);
    }

    for objective in &objectives.objectives {
        if let Some(active) = log
            .active_objective_entries
            .iter()
            .find(|entry| entry.id == objective.id)
            .cloned()
        {
            if let Some(entry) = log.entries.get_mut(active.entry_index) {
                entry.message = objective.message.clone();
            }
            continue;
        }

        let entry_index = log.entries.len();
        log.entries.push(NovaOsFlightLogEntry {
            kind: NovaOsFlightLogEntryKind::ObjectivePosted,
            objective_id: Some(objective.id.clone()),
            speaker: None,
            message: objective.message.clone(),
            icon: None,
        });
        log.active_objective_entries
            .push(NovaOsFlightLogActiveObjective {
                id: objective.id.clone(),
                entry_index,
            });
    }

    log.previous_active = objectives.objectives.clone();
}

/// Write a line to the flight log for every combat lock the upkeep let go of.
///
/// "Sometimes the ship loses radar focus on locked enemies" was unanswerable
/// from a shipped run: the drop had a reason, and the reason reached a `debug!`
/// nobody reads. The log is where a player asks that question afterwards, so
/// the reason goes where the question is asked - no new instrument, and nothing
/// on screen mid-fight.
pub(crate) fn log_combat_lock_drops(
    mut drops: MessageReader<CombatLockDropped>,
    mut log: ResMut<NovaOsFlightLog>,
) {
    for drop in drops.read() {
        log.entries.push(NovaOsFlightLogEntry {
            kind: NovaOsFlightLogEntryKind::System,
            objective_id: None,
            speaker: None,
            message: combat_lock_drop_line(drop),
            icon: None,
        });
    }
}

/// The one-line reason, phrased as the ship reporting rather than as the
/// enum naming itself.
fn combat_lock_drop_line(drop: &CombatLockDropped) -> String {
    match drop.reason {
        CombatLockDrop::TargetGone => "Combat lock lost: target is gone.".to_string(),
        CombatLockDrop::OutOfRange => "Combat lock lost: target out of lock range.".to_string(),
        CombatLockDrop::AllegianceFlip => {
            "Combat lock released: target is no longer hostile.".to_string()
        }
        CombatLockDrop::Occluded => "Combat lock lost: target behind cover.".to_string(),
    }
}
