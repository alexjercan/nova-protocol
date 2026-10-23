//! The Jobs page: the JOBS pane in the interface shell. Docked, the station
//! board sits over the player's own jobs; undocked, only the own jobs show.
//! The inspector at the right reads the selected job.

use bevy::{prelude::*, ui::InteractionDisabled};
use nova_ui::{prelude::*, theme::UiColor};

use crate::{
    chrome::*,
    data::{distance_hint, job, thousands, JobCard, JobKind},
    icons::{icon, GalleryIcons},
    BoardTier, Click, DockState, JobsState, ACCEPT_PREFIX, EMPTY_STATE, HAND_IN_PREFIX,
    HELD_ROW_PREFIX, POSTED_ROW_PREFIX, SHOW_ON_MAP, TIER_PREFIX,
};

const TABS_DOCKED: [&str; 5] = ["Map", "Ship", "Inventory", "Jobs", "Station"];
const TABS_UNDOCKED: [&str; 4] = ["Map", "Ship", "Inventory", "Jobs"];

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
                .spawn(side_panel(Overflow::clip()))
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
        panel.spawn(text(
            "Job kinds are a proof of concept; the set changes.",
            11.0,
            UiColor::Label,
        ));
        table_head(
            panel,
            &[
                (28.0, ""),
                (0.0, "Job"),
                (90.0, "Reward"),
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
                r.spawn((
                    Node {
                        width: px(120),
                        flex_shrink: 0.0,
                        align_items: AlignItems::Center,
                        column_gap: px(8),
                        ..default()
                    },
                    children![
                        bare_meter(72.0, j.minutes_left as f32 / j.window.max(1) as f32),
                        text_nowrap(&format!("{} min", j.minutes_left), 12.0, UiColor::Body),
                    ],
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

/// Why a taken job cannot be handed in here, if it cannot, in the game's
/// order and words: the dock, the station, then the goods or the work.
fn hand_in_refusal(j: &JobCard, docked: bool) -> Option<String> {
    if !docked {
        return Some(format!("dock at {} to hand in", j.hand_in));
    }
    if !j.hand_in_here {
        return Some(format!("hand in at {}", j.hand_in));
    }
    if j.done {
        return None;
    }
    Some(match j.kind {
        JobKind::Bring => format!(
            "the hold carries {} of {}",
            j.have,
            j.title.trim_start_matches("Bring ")
        ),
        _ => "not done yet".to_string(),
    })
}

fn held_panel(
    col: &mut ChildSpawnerCommands,
    icons: &GalleryIcons,
    jobs: &JobsState,
    docked: bool,
) {
    col.spawn(column_panel()).with_children(|panel| {
        panel.spawn(text("Your jobs", 15.0, UiColor::Primary));
        table_head(
            panel,
            &[
                (28.0, ""),
                (0.0, "Job"),
                (130.0, "Progress"),
                (90.0, "Hand-in"),
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
                    width: px(130.0),
                    flex_shrink: 0.0,
                    ..default()
                })
                .with_children(|c| {
                    c.spawn(meter(72.0, j.have, j.need));
                });
                r.spawn(Node {
                    width: px(90.0),
                    flex_shrink: 0.0,
                    ..default()
                })
                .with_children(|c| {
                    if docked && j.hand_in_here {
                        c.spawn(tag(BadgeKind::Green, "HAND IN HERE"));
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
                children![text("No jobs taken.", 12.0, UiColor::Label)],
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
        return;
    };
    side.spawn(Node {
        column_gap: px(8),
        ..control_row(JustifyContent::FlexStart)
    })
    .with_children(|head| {
        head.spawn(icon(icons, j.kind.icon(), 24.0));
        head.spawn(text(j.title, 15.0, UiColor::Primary));
        if j.done {
            head.spawn(tag(BadgeKind::Green, "DONE"));
        }
    });
    side.spawn(divider());
    section(side, "Task");
    side.spawn(text(j.task, 12.0, UiColor::Body));
    side.spawn(divider());
    // The live inspector's wrapping Target row; `chrome::kv` stays one line
    // for its other pages.
    side.spawn(Node {
        column_gap: px(8),
        align_items: AlignItems::FlexStart,
        ..control_row(JustifyContent::FlexStart)
    })
    .with_children(|r| {
        r.spawn(cell(84.0, "Target", 12.0, UiColor::Label, false));
        r.spawn((
            Node {
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                ..default()
            },
            children![text(
                &format!(
                    "{}\n({}, {}), {}",
                    j.target_name,
                    j.target.0,
                    j.target.1,
                    distance_hint(j.target)
                ),
                13.0,
                UiColor::Primary,
            )],
        ));
    });
    kv(side, "Hand in", j.hand_in);
    kv(side, "Reward", &format!("{} cr", thousands(j.reward)));
    let taken = jobs.held.contains(&j.id);
    if !taken {
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
    }
    if taken {
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
