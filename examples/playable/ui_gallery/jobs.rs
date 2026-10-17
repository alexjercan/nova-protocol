//! The Jobs page: the JOBS pane in the interface shell. Docked, the station
//! board sits over the player's own jobs; undocked, only the own jobs show
//! and each held row says how far its hand-in is. The inspector at the right
//! reads the selected job.

use bevy::{prelude::*, ui::InteractionDisabled};
use nova_ui::{prelude::*, theme::UiColor};

use crate::{
    chrome::*,
    data::{distance_hint, job, offset_hint, thousands, JobCard, JobKind},
    icons::{icon, GalleryIcons},
    BoardTier, Click, DockState, JobsState, ACCEPT_PREFIX, EMPTY_STATE, HAND_IN_PREFIX,
    HELD_ROW_PREFIX, POSTED_ROW_PREFIX, SHOW_ON_MAP, TIER_PREFIX,
};

const TABS_DOCKED: [&str; 5] = ["Map", "Ship", "Inventory", "Jobs", "Station"];
const TABS_UNDOCKED: [&str; 4] = ["Map", "Ship", "Inventory", "Jobs"];

/// The note under the starter board's tag.
const STARTER_NOTE: &str = "Near the start a station posts only jobs your current hull can do; \
every job unlocks after the first upgrade, after 30 min of play, or at any station past 500 km.";

pub fn jobs_page(
    body: &mut ChildSpawnerCommands,
    icons: &GalleryIcons,
    jobs: &JobsState,
    dock: DockState,
    tier: BoardTier,
) {
    let docked = dock == DockState::Docked;
    let tabs: &[&str] = if docked { &TABS_DOCKED } else { &TABS_UNDOCKED };
    interface_card(body, "Jobs", tabs, "Jobs", None, |body| {
        body.spawn(view_split()).with_children(|split| {
            split.spawn(column()).with_children(|col| {
                if docked {
                    board_panel(col, icons, jobs, tier);
                }
                held_panel(col, icons, jobs, docked);
            });
            split
                .spawn(side_panel_wide(26.0, Overflow::scroll_y()))
                .with_children(|side| job_inspector(side, icons, jobs, docked));
        });
    });
}

/// What the board lists: a posted job still open, and under the starter
/// tier only one the current hull can do.
fn listed(j: &JobCard, tier: BoardTier) -> bool {
    j.minutes_left > 0 && (tier == BoardTier::Core || j.fits_role)
}

fn board_panel(
    col: &mut ChildSpawnerCommands,
    icons: &GalleryIcons,
    jobs: &JobsState,
    tier: BoardTier,
) {
    col.spawn(column_panel()).with_children(|panel| {
        panel
            .spawn(control_row(JustifyContent::SpaceBetween))
            .with_children(|head| {
                head.spawn(Node {
                    column_gap: px(10),
                    ..control_row(JustifyContent::FlexStart)
                })
                .with_children(|left| {
                    left.spawn(text("Dust Gate board", 15.0, UiColor::Primary));
                    let (kind, label) = match tier {
                        BoardTier::Starter => (BadgeKind::Amber, "STARTER BOARD"),
                        BoardTier::Core => (BadgeKind::Blue, "CORE BOARD"),
                    };
                    left.spawn(tag(kind, label));
                });
                head.spawn(segmented_container()).with_children(|seg| {
                    for (value, label) in [
                        (BoardTier::Starter, "Starter station"),
                        (BoardTier::Core, "Core station"),
                    ] {
                        let mut option = seg.spawn((
                            segmented_option_compact(label),
                            ButtonValue(value),
                            Name::new(format!("{TIER_PREFIX} {value:?}")),
                        ));
                        if value == tier {
                            option.insert(Selected);
                        }
                    }
                });
            });
        if tier == BoardTier::Starter {
            panel.spawn(text(STARTER_NOTE, 12.0, UiColor::Body));
        }
        table_head(
            panel,
            &[
                (28.0, ""),
                (0.0, "Job"),
                (90.0, "Reward"),
                (190.0, "Target"),
                (120.0, "Time left"),
                (78.0, ""),
            ],
        );
        let mut any = false;
        for &id in &jobs.posted {
            let j = job(id);
            if !listed(j, tier) {
                continue;
            }
            any = true;
            spawn_row(
                panel,
                Click::SelectJob(id),
                format!("{POSTED_ROW_PREFIX} {id}"),
                jobs.selected == Some(id),
            )
            .with_children(|r| {
                r.spawn(icon(icons, j.kind.icon(), 18.0));
                r.spawn(grow_cell(j.title, 13.0, UiColor::Primary));
                r.spawn(cell(
                    90.0,
                    &format!("{} cr", thousands(j.reward)),
                    13.0,
                    UiColor::Accent,
                    false,
                ));
                r.spawn(cell(
                    190.0,
                    &format!(
                        "{} ({},{}), {}",
                        j.target_name,
                        j.target.0,
                        j.target.1,
                        distance_hint(j.target)
                    ),
                    12.0,
                    UiColor::Body,
                    false,
                ));
                r.spawn(bare_meter(
                    120.0,
                    j.minutes_left as f32 / j.window.max(1) as f32,
                ));
                r.spawn((
                    Name::new(format!("{ACCEPT_PREFIX} {id}")),
                    acts(
                        compact_button(ButtonSpec::new("Accept").fit().primary()),
                        Click::Accept(id),
                    ),
                ));
            });
        }
        if !any {
            panel.spawn((
                Name::new(EMPTY_STATE),
                Node {
                    padding: UiRect::axes(px(13), px(10)),
                    ..default()
                },
                children![text(
                    "No open job on this board. Dock at a core station past 500 km for the full list.",
                    12.0,
                    UiColor::Label
                )],
            ));
        }
    });
}

/// Why a held job cannot be handed in here, if it cannot.
fn hand_in_refusal(j: &JobCard, docked: bool) -> Option<String> {
    if !j.done {
        return Some(format!(
            "Not done: {} of {} {}",
            j.have,
            j.need,
            match j.kind {
                JobKind::Bring => "t delivered",
                JobKind::Recover => "recovered",
                _ => "complete",
            }
        ));
    }
    if !docked {
        return Some(format!("Dock at {} to hand in", j.hand_in));
    }
    if !j.hand_in_here {
        return Some(format!(
            "Hand in at {}, {}",
            j.hand_in,
            offset_hint(j.target)
        ));
    }
    None
}

fn held_panel(
    col: &mut ChildSpawnerCommands,
    icons: &GalleryIcons,
    jobs: &JobsState,
    docked: bool,
) {
    col.spawn(column_panel()).with_children(|panel| {
        panel
            .spawn(control_row(JustifyContent::SpaceBetween))
            .with_children(|head| {
                head.spawn(text("Your jobs", 15.0, UiColor::Primary));
                head.spawn(text(
                    &format!("{} held", jobs.held.len()),
                    12.0,
                    UiColor::Label,
                ));
            });
        table_head(
            panel,
            &[
                (28.0, ""),
                (0.0, "Job"),
                (200.0, "Progress"),
                (190.0, if docked { "Hand-in" } else { "Hand-in at" }),
                (78.0, ""),
            ],
        );
        for &id in &jobs.held {
            let j = job(id);
            spawn_row(
                panel,
                Click::SelectJob(id),
                format!("{HELD_ROW_PREFIX} {id}"),
                jobs.selected == Some(id),
            )
            .with_children(|r| {
                r.spawn(icon(icons, j.kind.icon(), 18.0));
                r.spawn(grow_cell(j.title, 13.0, UiColor::Primary));
                r.spawn(Node {
                    width: px(200.0),
                    flex_shrink: 0.0,
                    ..default()
                })
                .with_children(|c| {
                    c.spawn(meter(110.0, j.have, j.need));
                });
                r.spawn(Node {
                    width: px(190.0),
                    flex_shrink: 0.0,
                    ..default()
                })
                .with_children(|c| {
                    if docked && j.hand_in_here {
                        c.spawn(tag(BadgeKind::Green, "HAND IN HERE"));
                    } else if docked {
                        c.spawn(text_nowrap(
                            &format!("hand in at {}", j.hand_in),
                            12.0,
                            UiColor::Body,
                        ));
                    } else {
                        c.spawn(text_nowrap(
                            &format!("{}, {}", j.hand_in, distance_hint(j.target)),
                            12.0,
                            UiColor::Body,
                        ));
                    }
                });
                let refusal = hand_in_refusal(j, docked);
                let mut button = r.spawn((
                    Name::new(format!("{HAND_IN_PREFIX} {id}")),
                    acts(
                        compact_button(ButtonSpec::new("Hand in").fit().primary()),
                        Click::HandIn(id),
                    ),
                ));
                if let Some(why) = refusal {
                    button.insert((InteractionDisabled, Tooltip(why)));
                }
            });
        }
        if jobs.held.is_empty() {
            panel.spawn((
                Node {
                    padding: UiRect::axes(px(13), px(10)),
                    ..default()
                },
                children![text("No job held.", 12.0, UiColor::Label)],
            ));
        }
    });
}

fn job_inspector(
    side: &mut ChildSpawnerCommands,
    icons: &GalleryIcons,
    jobs: &JobsState,
    docked: bool,
) {
    let Some(j) = jobs.selected.map(job) else {
        section(side, "Job");
        side.spawn(text(
            "Select a row to read the job, its poster and where it pays.",
            12.0,
            UiColor::Label,
        ));
        return;
    };
    side.spawn(Node {
        column_gap: px(8),
        ..control_row(JustifyContent::FlexStart)
    })
    .with_children(|head| {
        head.spawn(icon(icons, j.kind.icon(), 24.0));
        head.spawn(text(j.title, 15.0, UiColor::Primary));
    });
    side.spawn(Node {
        column_gap: px(6),
        ..control_row(JustifyContent::FlexStart)
    })
    .with_children(|tags| {
        tags.spawn(tag(BadgeKind::Mute, &j.kind.label().to_uppercase()));
        if jobs.held.contains(&j.id) {
            tags.spawn(tag(BadgeKind::Blue, "HELD"));
        }
        if j.done {
            tags.spawn(tag(BadgeKind::Green, "DONE"));
        }
    });
    side.spawn(divider());
    section(side, "Task");
    side.spawn(text(j.task, 12.0, UiColor::Body));
    side.spawn(divider());
    kv(side, "Poster", j.poster);
    kv(
        side,
        "Target",
        &format!(
            "{} ({},{}), {}",
            j.target_name,
            j.target.0,
            j.target.1,
            distance_hint(j.target)
        ),
    );
    kv(side, "Hand in", j.hand_in);
    kv(side, "Reward", &format!("{} cr", thousands(j.reward)));
    side.spawn(Node {
        column_gap: px(8),
        ..control_row(JustifyContent::FlexStart)
    })
    .with_children(|r| {
        r.spawn(cell(84.0, "Time left", 12.0, UiColor::Label, false));
        r.spawn(bare_meter(
            110.0,
            j.minutes_left as f32 / j.window.max(1) as f32,
        ));
        r.spawn(text(
            &format!("{} min", j.minutes_left),
            12.0,
            UiColor::Body,
        ));
    });
    if jobs.held.contains(&j.id) {
        side.spawn(Node {
            column_gap: px(8),
            ..control_row(JustifyContent::FlexStart)
        })
        .with_children(|r| {
            r.spawn(cell(84.0, "Progress", 12.0, UiColor::Label, false));
            r.spawn(meter(110.0, j.have, j.need));
        });
        if let Some(why) = hand_in_refusal(j, docked) {
            side.spawn(text(&why, 12.0, UiColor::Accent));
        }
    }
    side.spawn(divider());
    side.spawn(control_row(JustifyContent::FlexStart))
        .with_children(|row| {
            row.spawn((
                Name::new(SHOW_ON_MAP),
                acts(
                    compact_button(ButtonSpec::new("Show on Map").fit()),
                    Click::ShowOnMap(j.id),
                ),
            ));
        });
}
