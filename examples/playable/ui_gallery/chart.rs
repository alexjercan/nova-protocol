//! The Chart page: the MAP pane in its chart view. The 3D chart scene fills
//! the left, the side panel reads the selected sector or, with none, the
//! objectives over the discovery log, and the footer carries the legend and
//! the keys.

use bevy::prelude::*;
use nova_ui::{prelude::*, theme::UiColor};

use crate::{
    chart3d::{spawn_chart_scene, CellView, ChartView, MarkView, SceneAssets, SceneKind},
    chrome::*,
    data::{
        chart_cell, job, thousands, Content, Mark, StationKind, CHART, CHART_REACH, DISCOVERY_LOG,
        FIRST_MILESTONE, LAST_SEEN, PLAYER_CELL,
    },
    icons::{icon, GalleryIcons, IconType},
    ChartState, Click, JobsState, CHART_VIEWPORT, MAP_CHART, MAP_REFRAME,
};

const TABS: [&str; 5] = ["Map", "Ship", "Inventory", "Jobs", "Station"];
const PLAY_CLOCK: &str = "02:14:07";

/// The chart view of the mock world: every charted cell with its contents
/// and marks, the player at the centre.
pub fn world_view(selected: Option<(i32, i32)>) -> ChartView {
    let cells = CHART
        .iter()
        .map(|cell| {
            let mut contents = Vec::new();
            let mut marks = Vec::new();
            for mark in cell.marks {
                match *mark {
                    Mark::Field(content, _) => contents.push(content),
                    Mark::Station(kind, code, _) => marks.push(MarkView::Station {
                        kind,
                        code: code.to_string(),
                    }),
                    Mark::Wreck(code) => marks.push(MarkView::Wreck(code.to_string())),
                    Mark::Job(id) => marks.push(MarkView::Job(format!("JOB-{id}"))),
                }
            }
            CellView {
                at: cell.at,
                visited: cell.entered.is_some(),
                contents,
                marks,
            }
        })
        .collect();
    ChartView {
        player: PLAYER_CELL,
        reach: CHART_REACH,
        cells,
        selected,
        label_cells: true,
        you: "YOU",
    }
}

pub fn chart_page(
    body: &mut ChildSpawnerCommands,
    icons: &GalleryIcons,
    assets: &mut SceneAssets,
    chart: &ChartState,
    jobs: &JobsState,
) {
    interface_card(body, "Map", &TABS, "Map", Some(PLAY_CLOCK), |body| {
        body.spawn(view_split()).with_children(|split| {
            let view = world_view(chart.selected);
            spawn_chart_scene(
                split,
                assets,
                SceneKind::Chart,
                &view,
                viewport_node(),
                CHART_VIEWPORT,
            );
            split
                .spawn(side_panel(Overflow::scroll_y()))
                .with_children(|side| match chart.selected {
                    Some(at) => sector_panel(side, icons, at, jobs),
                    None => {
                        objectives(side, icons, jobs);
                        side.spawn(divider());
                        discovery_log(side, icons);
                    }
                });
        });
        pane_footer(
            body,
            |legend| {
                for (what, label) in [
                    (IconType::Asteroids, "Asteroids"),
                    (IconType::Rocks, "Rocks"),
                    (IconType::Planet, "Planet"),
                    (IconType::Derelicts, "Derelicts"),
                    (IconType::Station, "Station"),
                    (IconType::Wreck, "Wreck"),
                    (IconType::Job, "Job"),
                    (IconType::You, "You"),
                ] {
                    legend.spawn(legend_entry()).with_children(|e| {
                        e.spawn(icon(icons, what, 14.0));
                        e.spawn(text(label, 12.0, UiColor::Body));
                    });
                }
            },
            |controls| {
                controls.spawn((
                    Name::new(MAP_REFRAME),
                    acts(
                        button(ButtonSpec::new("Reframe").fit()),
                        Click::Reframe(SceneKind::Chart),
                    ),
                ));
                controls.spawn((Name::new(MAP_CHART), button(ButtonSpec::new("Chart").fit())));
            },
            &[
                ("Q/E", "Turn"),
                ("R/F", "Tilt"),
                ("WASD", "Pan"),
                ("T", "Reframe"),
                ("[ ]", "Select"),
                ("C", "Chart"),
            ],
            &["Right-drag look", "Wheel zoom", "Click a sector"],
        );
    });
}

/// The OBJECTIVES block: the held jobs and the first milestone.
fn objectives(side: &mut ChildSpawnerCommands, icons: &GalleryIcons, jobs: &JobsState) {
    section(side, "Objectives");
    side.spawn(Node {
        column_gap: px(8),
        ..control_row(JustifyContent::FlexStart)
    })
    .with_children(|r| {
        r.spawn(icon(icons, IconType::Milestone, 16.0));
        r.spawn(grow_cell(FIRST_MILESTONE, 13.0, UiColor::Primary));
        r.spawn(tag(BadgeKind::Green, "MET"));
    });
    for &id in &jobs.held {
        let j = job(id);
        side.spawn(Node {
            column_gap: px(8),
            ..control_row(JustifyContent::FlexStart)
        })
        .with_children(|r| {
            r.spawn(icon(icons, j.kind.icon(), 16.0));
            r.spawn(grow_cell(j.title, 13.0, UiColor::Primary));
            r.spawn(text(
                &format!("{} / {}", j.have, j.need),
                12.0,
                UiColor::Body,
            ));
        });
    }
    if jobs.held.is_empty() {
        side.spawn(text("No job held.", 12.0, UiColor::Label));
    }
}

/// The discovery log, grouped by session, newest first.
fn discovery_log(side: &mut ChildSpawnerCommands, icons: &GalleryIcons) {
    section(side, "Discovery log");
    let mut session: Option<bool> = None;
    for (what, code, at, when, this_session) in DISCOVERY_LOG {
        if session != Some(this_session) {
            session = Some(this_session);
            side.spawn(text(
                if this_session {
                    "This session"
                } else {
                    "Earlier"
                },
                11.0,
                UiColor::Label,
            ));
        }
        side.spawn(Node {
            column_gap: px(8),
            ..control_row(JustifyContent::FlexStart)
        })
        .with_children(|r| {
            r.spawn(icon(icons, what, 16.0));
            r.spawn(grow_cell(code, 13.0, UiColor::Primary));
            r.spawn(text(&format!("({},{})", at.0, at.1), 12.0, UiColor::Body));
            r.spawn(cell(72.0, when, 11.0, UiColor::Label, true));
        });
    }
}

/// The SECTOR panel for a clicked cell.
fn sector_panel(
    side: &mut ChildSpawnerCommands,
    icons: &GalleryIcons,
    at: (i32, i32),
    jobs: &JobsState,
) {
    side.spawn(control_row(JustifyContent::SpaceBetween))
        .with_children(|head| {
            head.spawn(text(
                &format!("SECTOR ({},{})", at.0, at.1),
                15.0,
                UiColor::Primary,
            ));
            head.spawn(text(&crate::data::offset_hint(at), 12.0, UiColor::Label));
        });
    let Some(cell_data) = chart_cell(at) else {
        side.spawn(text(
            "Uncharted. Fly there to chart its contents.",
            12.0,
            UiColor::Body,
        ));
        return;
    };
    side.spawn(divider());
    section(side, "Contents");
    let mut listed = false;
    for mark in cell_data.marks {
        let (what, label, count): (IconType, String, Option<u8>) = match *mark {
            Mark::Field(content, n) => (content.icon(), content.label().to_string(), Some(n)),
            Mark::Station(kind, code, name) => (
                IconType::Station,
                format!("{code} {name}, {}", kind.state()),
                None,
            ),
            Mark::Wreck(code) => (IconType::Wreck, format!("{code} wreck"), None),
            Mark::Job(_) => continue,
        };
        listed = true;
        side.spawn(Node {
            column_gap: px(8),
            ..control_row(JustifyContent::FlexStart)
        })
        .with_children(|r| {
            r.spawn(icon(icons, what, 16.0));
            r.spawn(grow_cell(&label, 13.0, UiColor::Primary));
            if let Some(n) = count {
                r.spawn(text(&format!("x{n}"), 12.0, UiColor::Body));
            }
        });
    }
    if !listed {
        side.spawn(text("Empty space.", 12.0, UiColor::Body));
    }
    side.spawn(divider());
    match cell_data.entered {
        Some(entered) => kv(side, "Entered", &format!("play time {entered}")),
        None => kv(side, "Entered", "never; known from a job"),
    }
    if !cell_data.finds.is_empty() {
        kv(side, "Finds", &cell_data.finds.join(", "));
    }
    let docked_station = cell_data.marks.iter().find_map(|m| match m {
        Mark::Station(kind, code, name) => Some((*kind, *code, *name)),
        _ => None,
    });
    if let Some((kind, _, name)) = docked_station {
        side.spawn(divider());
        section(side, &format!("{name} last seen"));
        match kind {
            StationKind::Dead => {
                side.spawn(text("Dead. Nothing trades here.", 12.0, UiColor::Body));
            }
            _ => {
                kv(side, "Docked", "play time 00:03");
                for (ware, price, stock) in LAST_SEEN {
                    side.spawn(Node {
                        column_gap: px(8),
                        ..control_row(JustifyContent::FlexStart)
                    })
                    .with_children(|r| {
                        r.spawn(grow_cell(ware, 13.0, UiColor::Primary));
                        r.spawn(text(&format!("{stock} t"), 12.0, UiColor::Body));
                        r.spawn(cell(
                            60.0,
                            &format!("{} cr", thousands(price)),
                            12.0,
                            UiColor::Accent,
                            true,
                        ));
                    });
                }
            }
        }
    }
    let posted: Vec<u32> = cell_data
        .marks
        .iter()
        .filter_map(|m| match m {
            Mark::Job(id) => Some(*id),
            _ => None,
        })
        .collect();
    if !posted.is_empty() {
        side.spawn(divider());
        section(side, "Jobs here");
        for id in posted {
            let j = job(id);
            side.spawn(Node {
                column_gap: px(8),
                ..control_row(JustifyContent::FlexStart)
            })
            .with_children(|r| {
                r.spawn(icon(icons, j.kind.icon(), 16.0));
                r.spawn(grow_cell(j.title, 13.0, UiColor::Primary));
                r.spawn(text(
                    if jobs.held.contains(&id) {
                        "held"
                    } else {
                        "open"
                    },
                    12.0,
                    UiColor::Label,
                ));
            });
        }
    }
    let contents = cell_data
        .marks
        .iter()
        .filter_map(|m| match m {
            Mark::Field(c, _) => Some(*c),
            _ => None,
        })
        .collect::<Vec<Content>>();
    if contents.len() > 1 {
        side.spawn(text(
            "Several contents: the tile stacks one layer per content.",
            11.0,
            UiColor::Label,
        ));
    }
}
