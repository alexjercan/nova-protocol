//! The pure text layer: what the status line, breadcrumb, log rows, objective
//! rows and section table SAY, given a snapshot of the world.
//!
//! No `Commands` and no queries past the snapshot, so every string is testable
//! without an app.
//!
//! Touch this module when changing the wording or shape of terminal output.

use bevy::prelude::*;
use nova_gameplay::{objectives::GameObjectives, prelude::*};
use nova_os::prelude::*;
use nova_ship::prelude::*;

use super::components::*;
use crate::ship::prelude::SectionCode;

// Order and summaries mirror `nova_os_terminal_poc.html`'s command list. `map`
// and `ship viewer` from the PoC stay out until their stretch app tasks land.

pub(crate) fn nova_os_ship_name(name: Option<&Name>) -> String {
    name.map(|name| name.as_str().to_uppercase())
        .unwrap_or_else(|| "UNKNOWN".to_string())
}

/// The FPS number formatted to a FIXED width so the topbar does not reflow when
/// the reading changes digit count (owner playtest: 100 -> 99 must not shift the
/// layout). Right-aligned to 3 chars in the monospace topbar font, so ` 99` and
/// `100` occupy the same width; `--` before the first reading pads the same way.
pub(crate) fn nova_os_fps_segment(fps: Option<u32>) -> String {
    match fps {
        Some(fps) => format!("{fps:>3}"),
        None => format!("{:>3}", "--"),
    }
}

/// The NOVA OS topbar status line: ship + link, plus a live FPS segment. The FPS
/// is rehomed here from the flight status bar, which hides while the computer is
/// open; `fps` is the smoothed frame rate rounded to a
/// whole number, or `None` before the diagnostic has a reading (shown as `--`).
pub(crate) fn nova_os_status_text(ship_name: &str, fps: Option<u32>) -> String {
    let fps = nova_os_fps_segment(fps);
    format!("SHIP: {ship_name}     LINK: LOCAL     FPS: {fps}")
}

/// The header brand/breadcrumb line for the active surface. The terminal reads
/// `NOVA OS <ver> // SHELL`; a launched app reads `NOVA OS <ver> // APPS / <ID>`
/// where `<ID>` is the app's launch word upper-cased - NOT its `title()`, which
/// may itself contain a `/` (the map's title is "MAP / LOCAL SPACE").
pub(crate) fn nova_os_header_breadcrumb(mode: TerminalMode) -> String {
    let ver = nova_os_version_label();
    match mode {
        TerminalMode::Prompt => format!("NOVA OS {ver} // SHELL"),
        TerminalMode::App { id } => format!("NOVA OS {ver} // APPS / {}", id.to_uppercase()),
    }
}

impl NovaOsFlightLog {
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.active_objective_entries.clear();
        self.previous_active.clear();
        self.seen_story = 0;
    }
}

pub(crate) fn terminal_snapshot_from_world(
    log: &NovaOsFlightLog,
    objectives: &GameObjectives,
    ship_name: Option<&str>,
    ship_sections: &[ShipSectionStatus],
    seen_events: usize,
) -> TerminalCommandSnapshot {
    let unread_events = log.entries.len().saturating_sub(seen_events);
    // Keyed by command name so `submit` can look each up generically. `map view`
    // is filled by the caller (it needs live contact queries this pure builder
    // lacks), so it is absent here and prints nothing until populated.
    TerminalCommandSnapshot {
        unread_events,
        unread_hook: nova_os_unread_hook(log, seen_events),
        ..Default::default()
    }
    .with_output("log", terminal_log_rows(log))
    .with_output("objectives", terminal_objective_rows(objectives))
    // Bare `ship` launches the schematic viewer app; the status summary is the
    // `ship view` CLI subcommand.
    .with_output("ship view", terminal_ship_rows(ship_name, ship_sections))
}

/// A short lead-in for the most recent unread flight-log entry, used by the boot
/// banner's unread-events line. `None` when nothing is unread.
pub(crate) fn nova_os_unread_hook(log: &NovaOsFlightLog, seen_events: usize) -> Option<String> {
    log.entries
        .get(seen_events..)
        .and_then(|unread| unread.last())
        .map(|entry| entry.message.clone())
}

pub(crate) fn terminal_log_rows(log: &NovaOsFlightLog) -> Vec<TerminalRow> {
    if log.entries.is_empty() {
        return vec![TerminalRow {
            kind: TerminalRowKind::Dim,
            text: "Flight log is empty.".to_string(),
        }];
    }
    // HTML-style log: each entry gets a 4-digit sequential index prefix
    // (`0001 COMMS ... > ...`, `0003 OBJ + ...`) with no separate header.
    log.entries
        .iter()
        .enumerate()
        .map(|(index, entry)| TerminalRow {
            kind: match entry.kind {
                NovaOsFlightLogEntryKind::Comms => TerminalRowKind::Output,
                NovaOsFlightLogEntryKind::ObjectivePosted => TerminalRowKind::Warn,
                NovaOsFlightLogEntryKind::ObjectiveCompleted => TerminalRowKind::Info,
            },
            text: format!("{:04} {}", index + 1, nova_os_flight_log_text(entry)),
        })
        .collect()
}

pub(crate) fn terminal_objective_rows(objectives: &GameObjectives) -> Vec<TerminalRow> {
    if objectives.objectives.is_empty() {
        return vec![TerminalRow {
            kind: TerminalRowKind::Dim,
            text: "No active objectives.".to_string(),
        }];
    }
    // HTML-style objectives: one `OBJ + <message>` row each, no header.
    objectives
        .objectives
        .iter()
        .map(|objective| TerminalRow {
            kind: TerminalRowKind::Warn,
            text: format!("OBJ + {}", objective.message),
        })
        .collect()
}

#[derive(Debug, Clone)]
pub(crate) struct ShipSectionStatus {
    /// The short section code (`HULL-1`, `PDC-1`, ...) - the LABEL shown in
    /// `ship view` and the id the `ship <verb> <id>` commands take. Falls back to
    /// the uppercase kind label when a section has no code minted yet.
    pub(crate) code: String,
    pub(crate) kind: SectionClass,
    pub(crate) health: Option<Health>,
    pub(crate) inactive: bool,
    pub(crate) zero_health: bool,
    pub(crate) ammo: Option<SectionAmmo>,
}

pub(crate) fn terminal_ship_rows(
    ship_name: Option<&str>,
    sections: &[ShipSectionStatus],
) -> Vec<TerminalRow> {
    if sections.is_empty() {
        return vec![
            TerminalRow {
                kind: TerminalRowKind::Info,
                text: format!("SHIP {}", terminal_ship_name(ship_name)),
            },
            TerminalRow {
                kind: TerminalRowKind::Dim,
                text: "No live player ship sections detected.".to_string(),
            },
        ];
    }

    let mut rows = vec![
        TerminalRow {
            kind: TerminalRowKind::Info,
            text: format!("SHIP {}", terminal_ship_name(ship_name)),
        },
        TerminalRow {
            kind: TerminalRowKind::Dim,
            text: format!("Sections: {}", sections.len()),
        },
    ];

    // Column widths: pad KIND and LABEL to the widest cell (header included) so the
    // monospace terminal lines the columns up. INFO is last, so it needs no pad.
    const KIND_HEADER: &str = "KIND";
    const LABEL_HEADER: &str = "LABEL";
    const GUTTER: &str = "  ";
    let w_kind = sections
        .iter()
        .map(|s| section_kind_label(s.kind).len())
        .chain([KIND_HEADER.len()])
        .max()
        .unwrap_or(KIND_HEADER.len());
    let w_label = sections
        .iter()
        .map(|s| s.code.len())
        .chain([LABEL_HEADER.len()])
        .max()
        .unwrap_or(LABEL_HEADER.len());

    rows.push(TerminalRow {
        kind: TerminalRowKind::Dim,
        text: format!("{KIND_HEADER:<w_kind$}{GUTTER}{LABEL_HEADER:<w_label$}{GUTTER}INFO"),
    });
    for section in sections {
        let status = section_status_label(section);
        // INFO = health + ammo, plus the status word when the section is not
        // nominal (the separate status sub-row is gone - the word + row colour carry it).
        let mut info = format!(
            "{}{}",
            section_health_text(section.health.as_ref()),
            section_ammo_suffix(section.ammo.as_ref())
        );
        if status != "nominal" {
            info.push_str(&format!("{GUTTER}[{status}]"));
        }
        rows.push(TerminalRow {
            kind: section_status_row_kind(section),
            text: format!(
                "{kind:<w_kind$}{GUTTER}{label:<w_label$}{GUTTER}{info}",
                kind = section_kind_label(section.kind),
                label = section.code,
            ),
        });
    }
    rows
}

pub(crate) fn terminal_ship_name(name: Option<&str>) -> String {
    name.map(str::to_uppercase)
        .unwrap_or_else(|| "UNKNOWN".to_string())
}

pub(crate) fn section_kind_label(kind: SectionClass) -> &'static str {
    match kind {
        SectionClass::Hull => "HULL",
        SectionClass::Thruster => "THRUSTER",
        SectionClass::Controller => "CONTROLLER",
        SectionClass::Turret => "TURRET",
        SectionClass::Torpedo => "TORPEDO",
    }
}

pub(crate) fn section_health_text(health: Option<&Health>) -> String {
    match health {
        Some(health) if health.max > 0.0 => {
            format!("{:.0}/{:.0} HP", health.current.max(0.0), health.max)
        }
        Some(health) => format!("{:.0} HP", health.current.max(0.0)),
        None => "HP unknown".to_string(),
    }
}

pub(crate) fn section_ammo_suffix(ammo: Option<&SectionAmmo>) -> String {
    ammo.map(|ammo| format!("; ammo {}/{}", ammo.rounds, ammo.capacity))
        .unwrap_or_default()
}

pub(crate) fn section_status_label(section: &ShipSectionStatus) -> &'static str {
    if section.inactive || section.zero_health {
        return "neutralized";
    }
    let Some(health) = section.health.as_ref() else {
        return "nominal";
    };
    if health.max > 0.0 && health.current / health.max <= 0.25 {
        "critical"
    } else {
        "nominal"
    }
}

pub(crate) fn section_status_row_kind(section: &ShipSectionStatus) -> TerminalRowKind {
    match section_status_label(section) {
        "neutralized" => TerminalRowKind::Error,
        "critical" => TerminalRowKind::Warn,
        _ => TerminalRowKind::Output,
    }
}
pub(crate) fn player_ship_snapshot(
    q_player: &Query<
        (Entity, Option<&Name>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    q_sections: &Query<
        (
            &ChildOf,
            Option<&Health>,
            Option<&SectionClass>,
            Has<SectionInactiveMarker>,
            Has<HealthZeroMarker>,
            Has<HullSectionMarker>,
            Has<ControllerSectionMarker>,
            Has<ThrusterSectionMarker>,
            Has<TurretSectionMarker>,
            Has<TorpedoSectionMarker>,
            Option<&SectionAmmo>,
            Option<&SectionCode>,
        ),
        With<SectionMarker>,
    >,
) -> (Option<String>, Vec<ShipSectionStatus>) {
    let Ok((ship, ship_name)) = q_player.single() else {
        return (None, Vec::new());
    };
    let mut sections: Vec<ShipSectionStatus> = q_sections
        .iter()
        .filter(|(ChildOf(parent), ..)| *parent == ship)
        .filter_map(
            |(
                _,
                health,
                class,
                inactive,
                zero_health,
                hull,
                controller,
                thruster,
                turret,
                torpedo,
                ammo,
                code,
            )| {
                let kind =
                    section_kind_from_markers(class, hull, controller, thruster, turret, torpedo)?;
                Some(ShipSectionStatus {
                    // The label shown in `ship view`; fall back to the kind label
                    // (uppercase, matching the column style) when no code is minted.
                    code: code
                        .map(|code| code.0.clone())
                        .unwrap_or_else(|| section_kind_label(kind).to_string()),
                    kind,
                    health: health.cloned(),
                    inactive,
                    zero_health,
                    ammo: ammo.copied(),
                })
            },
        )
        .collect();
    // Sort by the displayed columns: kind, then the code label.
    sections.sort_by(|a, b| {
        section_kind_label(a.kind)
            .cmp(section_kind_label(b.kind))
            .then_with(|| a.code.cmp(&b.code))
    });
    (ship_name.map(|name| name.as_str().to_string()), sections)
}

pub(crate) fn section_kind_from_markers(
    class: Option<&SectionClass>,
    hull: bool,
    controller: bool,
    thruster: bool,
    turret: bool,
    torpedo: bool,
) -> Option<SectionClass> {
    if let Some(class) = class {
        return Some(*class);
    }
    if hull {
        Some(SectionClass::Hull)
    } else if controller {
        Some(SectionClass::Controller)
    } else if thruster {
        Some(SectionClass::Thruster)
    } else if turret {
        Some(SectionClass::Turret)
    } else if torpedo {
        Some(SectionClass::Torpedo)
    } else {
        None
    }
}

/// The separator that fronts the FPS segment in the topbar status line. The drive
/// system rewrites everything from this marker on, leaving the `SHIP:`/`LINK:`
/// head (which never changes after spawn) untouched.
pub(crate) const NOVA_OS_TOPBAR_FPS_MARKER: &str = "     FPS: ";

/// The smoothed frame rate rounded to a whole number, or `None` before the
/// diagnostic has a reading. Reuses Bevy's `FrameTimeDiagnosticsPlugin::FPS`
/// smoothed value - the exact source the flight status bar's FPS item read
/// (bcs `status_fps_value_fn`) - so the number on the topbar matches the one the
/// hidden status bar would show.
pub(crate) fn nova_os_diagnostic_fps(
    diagnostics: &bevy::diagnostic::DiagnosticsStore,
) -> Option<u32> {
    diagnostics
        .get(&bevy::diagnostic::FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|fps| fps.smoothed())
        .map(|fps| fps.round() as u32)
}

/// Rewrite only the `FPS: <n>` tail of a topbar status line, preserving the
/// `SHIP:`/`LINK:` head. Falls back to appending the segment if a line somehow
/// lacks it (e.g. an older spawn), so the FPS never silently goes missing.
pub(crate) fn topbar_line_with_fps(current: &str, fps: Option<u32>) -> String {
    let head = current
        .split_once(NOVA_OS_TOPBAR_FPS_MARKER)
        .map(|(head, _)| head)
        .unwrap_or(current);
    let fps = nova_os_fps_segment(fps);
    format!("{head}{NOVA_OS_TOPBAR_FPS_MARKER}{fps}")
}
pub(crate) fn nova_os_flight_log_text(entry: &NovaOsFlightLogEntry) -> String {
    match entry.kind {
        NovaOsFlightLogEntryKind::Comms => format!(
            "COMMS {} > {}",
            entry.speaker.as_deref().unwrap_or("UNKNOWN").to_uppercase(),
            entry.message
        ),
        NovaOsFlightLogEntryKind::ObjectivePosted => format!("OBJ + {}", entry.message),
        NovaOsFlightLogEntryKind::ObjectiveCompleted => format!("OBJ x {}", entry.message),
    }
}
