//! The Map chart's STANCES block: where every civilization the player has
//! met stands, and the fine that clears an Enemy.
//!
//! With nothing selected the block lists one entry per civilization in
//! [`CivilizationStances::entries`] or charted as a living station's owner:
//! its name, seeded advancement, table stance and compact detail labels
//! ([`stance_entry`]). A Station mark selected whose civilization is known
//! adds that civilization's entry, then a row per Enemy neighbour of it
//! ([`enemy_neighbours`]) with its fine and a Pay fine button; the row's
//! tooltip says what paying does or why the button is greyed, and one line
//! under the rows says where to pay while undocked. The button is enabled
//! only docked at the selected station with the fine in hand. A click raises
//! [`StanceSettleCommand`], which
//! [`apply_station_settle_commands`](crate::station::apply_station_settle_commands)
//! applies at the docked station.
//!
//! Touch this module when changing what the STANCES block shows.

use std::collections::BTreeSet;

use bevy::{prelude::*, ui::InteractionDisabled, ui_widgets::Activate};
use nova_events::prelude::GeneratedBodyId;
use nova_gameplay::prelude::*;
use nova_ui::{
    prelude::{badge, BadgeKind, ButtonSpec, Tooltip},
    theme::UiColor,
};
use nova_world_base::prelude::{enemy_neighbours, standing, CivilizationField};

use super::{chart_mark_code, MapChartMarkType, MapRuntime};
use crate::{
    pane::{compact_button, control_row, divider, themed_label},
    station::{settle_refusal_text, SettleDock, StanceSettleCommand},
};

/// The STANCES block's container in the Map side panel, shown only while the
/// chart view is on.
#[derive(Component)]
pub(crate) struct MapStanceBlock;

/// The rows built inside [`MapStanceBlock`], and what they were last built
/// from, so a rebuild with nothing changed costs nothing.
#[derive(Component, Default)]
pub(crate) struct MapStanceRows(Vec<MapStanceRowView>);

/// What the STANCES rows were last computed for, so they are computed again
/// only on a difference: the view, the selection, the docked station, the
/// player's balance, the stance table and the play second the countdowns
/// read in. Each computation walks the seeded neighbourhood of every charted
/// station, too much for every frame. The table is held by value:
/// `expire_stances` writes it every frame it holds a record, so its change
/// tick says nothing.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MapStanceBuild {
    shown: bool,
    stances: CivilizationStances,
    selected: Option<MapChartMarkType>,
    docked_at: Option<Entity>,
    own_credits: u32,
    second: u64,
}

/// One row of the STANCES block, as it last drew.
#[derive(Clone, Debug, PartialEq)]
enum MapStanceRowView {
    /// A civilization's standing: its name, its table stance and the
    /// compact detail labels that apply, with explanations on hover.
    Entry {
        name: String,
        advancement: String,
        table: StanceType,
        details: Vec<(String, Option<&'static str>)>,
    },
    /// An Enemy neighbour of the selected station's civilization: its
    /// fine, its Pay fine button's state and the hint its row shows on
    /// hover: what paying does, or why the button is greyed.
    Settle {
        civilization: CivilizationId,
        fine: u32,
        enabled: bool,
        hint: String,
    },
    /// Where to pay, under the Settle rows, while the player is not
    /// docked at the selected station.
    Dock(String),
}

/// The badge text of `table`.
const fn stance_word(table: StanceType) -> &'static str {
    match table {
        StanceType::Ally => "ALLY",
        StanceType::Neutral => "NEUTRAL",
        StanceType::Enemy => "ENEMY",
    }
}

/// The badge family `table` reads in: the Map's contact colours.
const fn stance_badge_kind(table: StanceType) -> BadgeKind {
    match table {
        StanceType::Ally => BadgeKind::Blue,
        StanceType::Neutral => BadgeKind::Mute,
        StanceType::Enemy => BadgeKind::Red,
    }
}

/// The chart mark codes of every charted living, not-owned station that
/// admits the player at `now` and whose civilization's Enemy neighbours name
/// `enemy`, and the fine those stations take: the same figure for each,
/// since the fine is a function of `enemy` alone. A station whose own
/// civilization reads Enemy refuses the dock, so it takes no fine.
fn fine_takers(
    chart: &Chart,
    jobs: &Jobs,
    stances: &CivilizationStances,
    enemy: CivilizationId,
    now: f64,
) -> (Vec<String>, Option<u32>) {
    let mut codes = Vec::new();
    let mut fine = None;
    for station in chart.stations() {
        if station.derelict || station.owned {
            continue;
        }
        let Some(civilization) = station.civilization else {
            continue;
        };
        if standing(stances, &civilization, now).effective == StanceType::Enemy {
            continue;
        }
        let Some((_, found)) = enemy_neighbours(civilization, stances)
            .into_iter()
            .find(|(id, _)| *id == enemy)
        else {
            continue;
        };
        fine = Some(found);
        codes.push(
            chart_mark_code(chart, jobs, &MapChartMarkType::Station(station.id.clone()))
                .expect("a charted station has a mark code"),
        );
    }
    (codes, fine)
}

/// `civilization`'s table stance and the compact detail labels that apply,
/// in order: a live provocation, an Enemy's decay or seeded note, an Enemy's
/// fine and where it is taken, offences, merits. Explanations remain on hover.
fn stance_entry(
    chart: &Chart,
    jobs: &Jobs,
    stances: &CivilizationStances,
    civilization: CivilizationId,
    now: f64,
) -> (StanceType, Vec<(String, Option<&'static str>)>) {
    let standing = standing(stances, &civilization, now);
    let mut details = Vec::new();
    if let Some(left) = standing.provoked_left {
        details.push((
            format!("HOSTILE {}", play_time_text(left)),
            Some("Hostile after you fired on it."),
        ));
    }
    if standing.table == StanceType::Enemy {
        match standing.decay_left {
            Some(left) => details.push((
                format!("CLEAR {}", play_time_text(left)),
                Some("Clears if no new offence occurs."),
            )),
            None => details.push((
                "SEEDED".to_string(),
                Some("Enemy from the start; only a fine clears it."),
            )),
        }
        let (codes, fine) = fine_takers(chart, jobs, stances, civilization, now);
        details.push(match fine {
            Some(fine) => (
                format!("FINE {} @ {}", cr_text(u64::from(fine)), codes.join(", ")),
                Some("Pay at a listed station to return this civilization to Neutral."),
            ),
            None => (
                "FINE NONE".to_string(),
                Some("No charted station takes this fine."),
            ),
        });
    }
    if standing.offences > 0 {
        details.push((
            format!("OFFENCES {}/{OFFENCE_STEP}", standing.offences),
            Some("Offences in the last hour."),
        ));
    }
    if standing.merits > 0 {
        details.push((
            format!("MERITS {}/{MERIT_STEP}", standing.merits),
            Some("Jobs toward the next step up."),
        ));
    }
    (standing.table, details)
}

/// The overview rows: one entry per civilization in the union of
/// [`CivilizationStances::entries`] and the civilization of every charted,
/// living station, in [`CivilizationId`] order.
fn overview_rows(
    chart: &Chart,
    jobs: &Jobs,
    stances: &CivilizationStances,
    now: f64,
) -> Vec<MapStanceRowView> {
    let mut civs: BTreeSet<CivilizationId> = stances.entries().map(|(id, _)| *id).collect();
    for station in chart.stations() {
        if !station.derelict {
            if let Some(civilization) = station.civilization {
                civs.insert(civilization);
            }
        }
    }
    civs.into_iter()
        .map(|civilization| {
            let (table, details) = stance_entry(chart, jobs, stances, civilization, now);
            let advancement = CivilizationField::new(civilization.world_seed)
                .civilization(civilization.node)
                .advancement;
            assert!(advancement.is_finite(), "seeded advancement must be finite");
            MapStanceRowView::Entry {
                name: civilization.name(),
                advancement: format!(
                    "ADV {}%",
                    (advancement.clamp(0.0, 1.0) * 100.0).round() as u32
                ),
                table,
                details,
            }
        })
        .collect()
}

/// The rows for a selected Station mark whose civilization is `civilization`:
/// its own entry, then a Pay fine row per Enemy neighbour, then the Dock line
/// while the player is not docked here. A station whose civilization reads
/// Enemy refuses the dock, so its hints and Dock line say so, and the hints
/// name the charted stations that take the fine instead.
#[expect(
    clippy::too_many_arguments,
    reason = "the records the rows read and the live dock facts the button reads"
)]
fn station_rows(
    chart: &Chart,
    jobs: &Jobs,
    stances: &CivilizationStances,
    civilization: CivilizationId,
    now: f64,
    station_code: &str,
    docked_here: bool,
    own_credits: u32,
) -> Vec<MapStanceRowView> {
    let (table, details) = stance_entry(chart, jobs, stances, civilization, now);
    let refuses = standing(stances, &civilization, now).effective == StanceType::Enemy;
    let advancement = CivilizationField::new(civilization.world_seed)
        .civilization(civilization.node)
        .advancement;
    assert!(advancement.is_finite(), "seeded advancement must be finite");
    let mut rows = vec![MapStanceRowView::Entry {
        name: civilization.name(),
        advancement: format!(
            "ADV {}%",
            (advancement.clamp(0.0, 1.0) * 100.0).round() as u32
        ),
        table,
        details,
    }];
    for (enemy, fine) in enemy_neighbours(civilization, stances) {
        let name = enemy.name();
        let enabled = docked_here && own_credits >= fine;
        let hint = if enabled {
            format!(
                "Pay {}: {name} returns to NEUTRAL.",
                cr_text(u64::from(fine))
            )
        } else if refuses {
            let (codes, _) = fine_takers(chart, jobs, stances, enemy, now);
            if codes.is_empty() {
                format!("{station_code} refuses docking; no charted station takes it")
            } else {
                format!(
                    "{station_code} refuses docking; pay at {}",
                    codes.join(", ")
                )
            }
        } else if !docked_here {
            format!("Dock at {station_code} to pay: {name} returns to NEUTRAL.")
        } else {
            settle_refusal_text(&name, fine, own_credits)
        };
        rows.push(MapStanceRowView::Settle {
            civilization: enemy,
            fine,
            enabled,
            hint,
        });
    }
    // With no Settle row there is no fine to pay, so no line says where.
    if !docked_here && rows.len() > 1 {
        rows.push(MapStanceRowView::Dock(if refuses {
            format!("{station_code} REFUSED")
        } else {
            format!("DOCK {station_code}")
        }));
    }
    rows
}

/// Build the STANCES block: a header over the rows
/// [`update_map_stance_block`] fills, hidden off the chart view and with no
/// row to show.
pub(crate) fn spawn_map_stance_block(panel: &mut ChildSpawnerCommands) {
    panel
        .spawn((
            MapStanceBlock,
            Name::new("MapStanceBlock"),
            Node {
                display: Display::None,
                flex_direction: FlexDirection::Column,
                row_gap: px(4),
                ..default()
            },
        ))
        .with_children(|block| {
            block.spawn(divider());
            block.spawn((
                themed_label("STANCES", 11.0, UiColor::Label),
                Pickable::IGNORE,
            ));
            block.spawn((
                MapStanceRows::default(),
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4),
                    ..default()
                },
            ));
        });
}

/// Raise a [`StanceSettleCommand`] for `civilization` when its Pay fine
/// button is activated. The press is silent: the Map notice's outcome cue
/// is the one sound.
fn on_pay_fine_button(
    civilization: CivilizationId,
) -> impl FnMut(On<Activate>, MessageWriter<StanceSettleCommand>) {
    move |_activate, mut settles| {
        settles.write(StanceSettleCommand { civilization });
    }
}

/// Show the STANCES block while the chart view shows with no selection or a
/// Station mark whose civilization is known, and rebuild its rows whenever
/// what they show changes; hide it otherwise. A click on an enabled Pay fine
/// button raises [`StanceSettleCommand`]; a disabled one takes no click.
#[expect(
    clippy::too_many_arguments,
    reason = "the records the rows read and the live dock facts the button reads"
)]
pub(crate) fn update_map_stance_block(
    mut commands: Commands,
    runtime: Res<MapRuntime>,
    chart: Res<Chart>,
    jobs: Res<Jobs>,
    clock: Res<WorldClock>,
    stances: Res<CivilizationStances>,
    dock: SettleDock,
    q_body_id: Query<&GeneratedBodyId>,
    q_credits: Query<&ShipCredits>,
    mut q_block: Query<(Entity, &mut Node), With<MapStanceBlock>>,
    mut q_rows: Query<(Entity, &mut MapStanceRows)>,
    mut computed: Local<Option<MapStanceBuild>>,
) {
    let Ok((_block, mut node)) = q_block.single_mut() else {
        return;
    };
    let Ok((rows_entity, mut built)) = q_rows.single_mut() else {
        return;
    };
    let docked = dock.docked();
    let build = MapStanceBuild {
        shown: runtime.active && runtime.chart,
        stances: stances.clone(),
        selected: runtime.chart_selected.clone(),
        docked_at: docked.as_ref().map(|(_, station, _, _)| *station),
        own_credits: docked
            .as_ref()
            .and_then(|(player, ..)| q_credits.get(*player).ok())
            .map_or(0, |credits| credits.0),
        second: clock.play_seconds.max(0.0).floor() as u64,
    };
    if computed.as_ref() == Some(&build) && !chart.is_changed() && !jobs.is_changed() {
        return;
    }
    let rows: Vec<MapStanceRowView> = if !build.shown {
        Vec::new()
    } else {
        match &runtime.chart_selected {
            None => overview_rows(&chart, &jobs, &stances, clock.play_seconds),
            Some(MapChartMarkType::Station(id)) => chart
                .station(id)
                .filter(|station| !station.derelict && !station.owned)
                .and_then(|station| station.civilization)
                .map_or_else(Vec::new, |civilization| {
                    let station_code =
                        chart_mark_code(&chart, &jobs, &MapChartMarkType::Station(id.clone()))
                            .expect("a charted station has a mark code");
                    let docked_here = build
                        .docked_at
                        .is_some_and(|station| q_body_id.get(station) == Ok(id));
                    station_rows(
                        &chart,
                        &jobs,
                        &stances,
                        civilization,
                        clock.play_seconds,
                        &station_code,
                        docked_here,
                        build.own_credits,
                    )
                }),
            Some(_) => Vec::new(),
        }
    };
    *computed = Some(build);
    let display = if rows.is_empty() {
        Display::None
    } else {
        Display::Flex
    };
    if node.display != display {
        node.display = display;
    }
    if built.0 == rows {
        return;
    }
    commands
        .entity(rows_entity)
        .despawn_related::<Children>()
        .with_children(|list| {
            for (index, row) in rows.iter().enumerate() {
                match row {
                    MapStanceRowView::Entry {
                        name,
                        advancement,
                        table,
                        details,
                    } => {
                        list.spawn(Node {
                            column_gap: px(8),
                            ..control_row(JustifyContent::FlexStart)
                        })
                        .with_children(|row| {
                            row.spawn((
                                Node {
                                    flex_grow: 1.0,
                                    flex_basis: px(0),
                                    min_width: px(0),
                                    overflow: Overflow::clip(),
                                    ..default()
                                },
                                Pickable::IGNORE,
                                children![(
                                    themed_label(name, 13.0, UiColor::Primary),
                                    TextLayout::new(Justify::Left, LineBreak::NoWrap),
                                    Pickable::IGNORE
                                )],
                            ));
                            row.spawn((
                                badge(stance_badge_kind(*table), stance_word(*table)),
                                Pickable::IGNORE,
                            ))
                            .insert(Node {
                                align_items: AlignItems::Center,
                                flex_shrink: 0.0,
                                ..default()
                            });
                            row.spawn((
                                badge(BadgeKind::Mute, advancement),
                                Tooltip("Civilization advancement: seeded generation potential, not per-ship technology or progression.".to_string()),
                            ))
                            .insert(Node {
                                align_items: AlignItems::Center,
                                flex_shrink: 0.0,
                                ..default()
                            });
                        });
                        for (detail, hint) in details {
                            let label = themed_label(detail, 12.0, UiColor::Label);
                            if let Some(hint) = hint {
                                // A Tooltip spawns its panel as a child, and a
                                // text node with a child lays out as a container
                                // with no text height: the hint goes on a wrapper.
                                list.spawn((Node::default(), Tooltip((*hint).to_string())))
                                    .with_child(label);
                            } else {
                                list.spawn(label);
                            }
                        }
                    }
                    MapStanceRowView::Settle {
                        civilization,
                        fine,
                        enabled,
                        hint,
                    } => {
                        list.spawn((
                            Name::new(format!("MapStanceRow-{index}")),
                            Tooltip(hint.clone()),
                            Node {
                                column_gap: px(8),
                                ..control_row(JustifyContent::FlexStart)
                            },
                        ))
                        .with_children(|row| {
                            // The name cell takes the slack and clips, so a
                            // long name never pushes the button out.
                            row.spawn((
                                Node {
                                    flex_grow: 1.0,
                                    flex_basis: px(0),
                                    min_width: px(0),
                                    overflow: Overflow::clip(),
                                    ..default()
                                },
                                Pickable::IGNORE,
                                children![(
                                    themed_label(&civilization.name(), 13.0, UiColor::Primary),
                                    TextLayout::new(Justify::Left, LineBreak::NoWrap),
                                    Pickable::IGNORE
                                )],
                            ));
                            row.spawn((
                                badge(
                                    stance_badge_kind(StanceType::Enemy),
                                    stance_word(StanceType::Enemy),
                                ),
                                Pickable::IGNORE,
                            ));
                            row.spawn((
                                themed_label(&cr_text(u64::from(*fine)), 12.0, UiColor::Body),
                                TextLayout::new(Justify::Right, LineBreak::NoWrap),
                                Pickable::IGNORE,
                            ));
                            let mut button = row.spawn((
                                Name::new(format!("MapStancePayFine-{index}")),
                                compact_button(ButtonSpec::new("Pay fine").fit().primary()),
                            ));
                            button.observe(on_pay_fine_button(*civilization));
                            if !enabled {
                                button.insert(InteractionDisabled);
                            }
                        });
                    }
                    MapStanceRowView::Dock(line) => {
                        list.spawn(themed_label(line, 12.0, UiColor::Label));
                    }
                }
            }
        });
    built.0 = rows;
}
