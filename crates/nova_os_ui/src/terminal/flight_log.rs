//! The NOVA OS combined flight-log model, derived from the story feed and the
//! active objective list.
//!
//! Nothing here paints rows. The monitor has no permanent panes; the log
//! reaches the player through the `log` / `objectives` terminal commands, the
//! boot banner's unread-events count and the live completion announcements.
//!
//! Touch this module when changing how logged events are recorded or announced.

use bevy::prelude::*;
use nova_gameplay::{
    objectives::{GameObjectives, Objective},
    PauseStates,
};
use nova_hud::prelude::*;
use nova_os::prelude::*;

use super::{components::*, content::*};

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

/// Announce objective flips into the LIVE terminal scrollback while the computer
/// is open at the prompt (PoC `checkObjectives` pushes an `OBJ x ...` line the
/// moment an objective completes, so the player sees it without typing `log`).
/// Only completions that happen while open are announced; ones that flipped while
/// the computer was closed stay in the flight log (counted by the boot banner's
/// unread-events line instead of dumping on open).
pub(crate) fn announce_objectives_in_terminal(
    log: Res<NovaOsFlightLog>,
    pause: Res<State<PauseStates>>,
    mut terminal: ResMut<NovaOsTerminal>,
    mut announced: Local<Option<usize>>,
) {
    let total = log.entries.len();
    // `None` on the first run (and `min` if the log was cleared) means we start
    // from "everything already seen" - nothing is announced retroactively.
    let from = announced.unwrap_or(total).min(total);
    let open =
        *pause.get() == PauseStates::NovaOs && terminal.active_mode() == TerminalMode::Prompt;
    if open {
        let fresh: Vec<TerminalRow> = log.entries[from..]
            .iter()
            .filter(|entry| entry.kind == NovaOsFlightLogEntryKind::ObjectiveCompleted)
            .map(|entry| TerminalRow {
                kind: TerminalRowKind::Info,
                text: nova_os_flight_log_text(entry),
            })
            .collect();
        // Only touch the scrollback (and so mark the terminal changed, forcing a
        // rebuild that snaps the view to the bottom) when there is actually
        // something to announce - most objective-change frames have no completion.
        if !fresh.is_empty() {
            terminal.extend_scrollback(fresh);
        }
    }
    *announced = Some(total);
}
