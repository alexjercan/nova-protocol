//! The Load page: the main menu's LOAD panel. The saved worlds sort by last
//! played with Continue at the top; the details at the right preview the
//! save's chart in 3D, list the fleet it holds and its open objectives.

use bevy::prelude::*;
use nova_ui::{prelude::*, theme::UiColor};

use crate::{
    chart3d::{spawn_chart_scene, CellView, ChartView, MarkView, SceneAssets, SceneKind},
    chrome::*,
    data::{hash, thousands, Content, SaveCard, StationKind, ROLES, RUNNING_VERSION, SAVES},
    icons::{icon, GalleryIcons},
    Click, LoadState, CONTINUE, FLEET_ESCORTS, FLEET_OWNED_SHIPS, FLEET_OWNED_STATIONS, FLEET_SHIP,
    LOAD_DELETE, LOAD_LOAD, LOAD_PREVIEW, NO_OPEN_OBJECTIVES, SAVE_ROW_PREFIX,
};

/// A seeded chart around a start cell: every cell within `core` visited,
/// `explored` cells visited in all, the rest within `reach` known as
/// outlines. The same seed always draws the same chart, so Randomize visibly
/// changes the New Game preview.
pub fn seeded_view(
    seed: u64,
    core: i32,
    explored: u32,
    reach: i32,
    you: &'static str,
) -> ChartView {
    let mut visited = Vec::new();
    for x in -core..=core {
        for z in -core..=core {
            visited.push((x, z));
        }
    }
    let mut k = 1;
    while visited.len() < explored as usize && k < 200 {
        let h = hash(seed, k);
        let span = (2 * reach + 1) as u64;
        let at = ((h % span) as i32 - reach, ((h >> 16) % span) as i32 - reach);
        if !visited.contains(&at) {
            visited.push(at);
        }
        k += 1;
    }
    let mut cells = Vec::new();
    for x in -reach..=reach {
        for z in -reach..=reach {
            let at = (x, z);
            let h = hash(seed, 1000 + ((x + reach) * 16 + (z + reach)) as u64);
            let mut contents = Vec::new();
            match h % 6 {
                0 | 1 => contents.push(Content::Asteroids),
                2 => contents.push(Content::Rocks),
                3 => {
                    contents.push(Content::Asteroids);
                    contents.push(Content::Rocks);
                }
                4 => contents.push(Content::Derelicts),
                _ => {}
            }
            if (h >> 20) % 11 == 0 {
                contents.push(Content::Planet);
            }
            let mut marks = Vec::new();
            if at == (0, 0) {
                marks.push(MarkView::Station {
                    kind: StationKind::Living,
                    code: "STN-1".into(),
                });
            } else if (h >> 28) % 9 == 0 {
                marks.push(MarkView::Station {
                    kind: if (h >> 36) % 3 == 0 {
                        StationKind::Dead
                    } else {
                        StationKind::Living
                    },
                    code: format!("STN-{}", (h >> 40) % 9 + 2),
                });
            } else if (h >> 28) % 13 == 0 {
                marks.push(MarkView::Wreck(format!("WRK-{}", (h >> 44) % 9 + 1)));
            }
            cells.push(CellView {
                at,
                visited: visited.contains(&at),
                contents,
                marks,
            });
        }
    }
    ChartView {
        player: (0, 0),
        reach,
        cells,
        selected: None,
        label_cells: false,
        you,
    }
}

fn save_view(save: &SaveCard) -> ChartView {
    seeded_view(save.seed, 0, save.explored, 2, "YOU")
}

/// The row's objectives cell: how many are still open.
fn open_summary(save: &SaveCard) -> String {
    match save.open_count() {
        0 => "none open".to_string(),
        n => format!("{n} open"),
    }
}

pub fn load_page(
    body: &mut ChildSpawnerCommands,
    icons: &GalleryIcons,
    assets: &mut SceneAssets,
    load: &LoadState,
) {
    menu_card(body, "Load", "SAVED WORLDS", |body| {
        body.spawn(view_split()).with_children(|split| {
            split.spawn(column()).with_children(|list| {
                // Continue: the most recently played world, one click.
                let latest = &SAVES[0];
                list.spawn(Node {
                    column_gap: px(12),
                    padding: UiRect::axes(px(13), px(6)),
                    ..control_row(JustifyContent::FlexStart)
                })
                .with_children(|row| {
                    row.spawn((
                        Name::new(CONTINUE),
                        acts(
                            button(ButtonSpec::new("Continue").fit().primary()),
                            Click::Save(0),
                        ),
                    ));
                    row.spawn(text(
                        &format!("{}, saved {}", latest.world, latest.saved_rel),
                        13.0,
                        UiColor::Body,
                    ));
                });
                table_head(
                    list,
                    &[
                        (24.0, ""),
                        (0.0, "World"),
                        (90.0, "Play time"),
                        (90.0, "Saved"),
                        (0.0, "Open"),
                    ],
                );
                for (i, save) in SAVES.iter().enumerate() {
                    let role = &ROLES[save.role];
                    spawn_row(
                        list,
                        Click::Save(i),
                        format!("{SAVE_ROW_PREFIX} {i}"),
                        load.selected == i,
                    )
                    .with_children(|r| {
                        r.spawn(icon(icons, role.icon, 18.0));
                        r.spawn(grow_cell(save.world, 13.0, UiColor::Primary));
                        r.spawn(cell(90.0, save.play_time, 12.0, UiColor::Body, false));
                        r.spawn(cell(90.0, save.saved_rel, 12.0, UiColor::Body, false));
                        r.spawn(grow_cell(&open_summary(save), 12.0, UiColor::Label));
                    });
                }
            });
            split
                .spawn(side_panel_wide(34.0, Overflow::scroll_y()))
                .with_children(|side| details(side, icons, assets, &SAVES[load.selected]));
        });
        body.spawn(control_row(JustifyContent::FlexStart))
            .with_children(|footer| {
                footer.spawn(button(ButtonSpec::new("Back").fit().key("Esc")));
            });
    });
}

fn details(
    side: &mut ChildSpawnerCommands,
    icons: &GalleryIcons,
    assets: &mut SceneAssets,
    save: &SaveCard,
) {
    let role = &ROLES[save.role];
    side.spawn(Node {
        column_gap: px(8),
        ..control_row(JustifyContent::SpaceBetween)
    })
    .with_children(|head| {
        head.spawn(Node {
            column_gap: px(8),
            ..control_row(JustifyContent::FlexStart)
        })
        .with_children(|left| {
            left.spawn(icon(icons, role.icon, 20.0));
            left.spawn(text(save.world, 15.0, UiColor::Primary));
        });
        if save.version != RUNNING_VERSION {
            head.spawn(tag(BadgeKind::Amber, save.version));
        }
    });
    spawn_chart_scene(
        side,
        assets,
        SceneKind::LoadPreview,
        &save_view(save),
        Node {
            height: px(220),
            flex_shrink: 0.0,
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        LOAD_PREVIEW,
    );
    side.spawn(text(
        "Right-drag turns, wheel zooms, T reframes.",
        11.0,
        UiColor::Label,
    ));
    kv(side, "Seed", &save.seed.to_string());
    kv(side, "Sector", save.sector);
    kv(side, "Credits", &format!("{} cr", thousands(save.credits)));
    kv(side, "Play time", save.play_time);
    kv(
        side,
        "Saved",
        &format!("{} ({})", save.saved_at, save.saved_rel),
    );
    side.spawn(divider());
    section(side, "Fleet");
    fleet_line(side, FLEET_SHIP, "Ship", save.hull.display());
    for (name, label, n) in [
        (FLEET_OWNED_SHIPS, "Owned ships", save.owned_ships),
        (FLEET_ESCORTS, "Escorts", save.escorts),
        (FLEET_OWNED_STATIONS, "Owned stations", save.owned_stations),
    ] {
        if n > 0 {
            fleet_line(side, name, label, &n.to_string());
        }
    }
    side.spawn(divider());
    section(side, "Open objectives");
    let mut any = false;
    for objective in save.objectives.iter().filter(|o| o.open()) {
        any = true;
        side.spawn(Node {
            column_gap: px(8),
            ..control_row(JustifyContent::FlexStart)
        })
        .with_children(|r| {
            r.spawn(icon(icons, objective.icon, 16.0));
            r.spawn(grow_cell(objective.title, 13.0, UiColor::Primary));
            r.spawn(text(
                &format!("{} / {}", objective.have, objective.need),
                12.0,
                UiColor::Body,
            ));
        });
    }
    if !any {
        side.spawn((
            Name::new(NO_OPEN_OBJECTIVES),
            text("No open objectives", 12.0, UiColor::Label),
        ));
    }
    side.spawn(Node {
        flex_grow: 1.0,
        ..default()
    });
    side.spawn(control_row(JustifyContent::FlexStart))
        .with_children(|row| {
            row.spawn((
                Name::new(LOAD_LOAD),
                button(ButtonSpec::new("Load").fit().primary().key("Enter")),
            ));
            row.spawn((
                Name::new(LOAD_DELETE),
                button(ButtonSpec::new("Delete").fit().danger()),
            ));
        });
}

/// One named `label  value` line of the FLEET block, so the smoke can see
/// which lines a save shows.
fn fleet_line(side: &mut ChildSpawnerCommands, name: &str, label: &str, value: &str) {
    side.spawn((
        Name::new(name.to_string()),
        Node {
            column_gap: px(8),
            ..control_row(JustifyContent::FlexStart)
        },
    ))
    .with_children(|r| {
        r.spawn(cell(84.0, label, 12.0, UiColor::Label, false));
        r.spawn(grow_cell(value, 13.0, UiColor::Primary));
    });
}
