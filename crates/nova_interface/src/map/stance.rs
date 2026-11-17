//! The Map chart's STANCES block: where every civilization the player has
//! met stands, and the fine that clears an Enemy.
//!
//! With nothing selected the block lists one entry per civilization in
//! [`CivilizationStances::entries`] or charted as a living station's owner:
//! its name, seeded advancement, table stance and compact detail labels
//! ([`stance_entry`]). A Station mark selected whose civilization is known
//! adds that civilization's entry, then a row per Enemy neighbour of it
//! ([`enemy_neighbours`]) with its fine and a Pay fine button, and one line
//! under the rows says where to pay while undocked. The button shows only
//! docked at the selected station, and is disabled with the fine painted red
//! while the balance is short of it. A click raises
//! [`StanceSettleCommand`], which
//! [`apply_station_settle_commands`](crate::station::apply_station_settle_commands)
//! applies at the docked station.
//!
//! Touch this module when changing what the STANCES block shows.

use std::collections::BTreeSet;

use bevy::{prelude::*, ui_widgets::Activate};
use nova_events::prelude::GeneratedBodyId;
use nova_gameplay::prelude::*;
use nova_ui::{
    prelude::{
        button, control_row, metrics, spawn_block, tag, text, ButtonCueType, ButtonSpec, Short,
        TextRole, VerbGate,
    },
    theme::UiColorType,
};
use nova_world_base::prelude::{enemy_neighbours, standing, CivilizationField};

use super::{chart_mark_code, MapChartMarkType, MapRuntime};
use crate::station::{SettleDock, StanceSettleCommand};

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
    /// compact detail labels that apply.
    Entry {
        name: String,
        advancement: String,
        table: StanceType,
        details: Vec<String>,
    },
    /// An Enemy neighbour of the selected station's civilization: its fine,
    /// whether the balance is short of it, and its Pay fine button's gate.
    Settle {
        civilization: CivilizationId,
        fine: u32,
        short: bool,
        gate: VerbGate,
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
const fn stance_badge_kind(table: StanceType) -> UiColorType {
    match table {
        StanceType::Ally => UiColorType::Info,
        StanceType::Neutral => UiColorType::TextSecondary,
        StanceType::Enemy => UiColorType::Danger,
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
/// fine and where it is taken, offences, merits.
fn stance_entry(
    chart: &Chart,
    jobs: &Jobs,
    stances: &CivilizationStances,
    civilization: CivilizationId,
    now: f64,
) -> (StanceType, Vec<String>) {
    let standing = standing(stances, &civilization, now);
    let mut details = Vec::new();
    if let Some(left) = standing.provoked_left {
        details.push(format!("HOSTILE {}", play_time_text(left)));
    }
    if standing.table == StanceType::Enemy {
        details.push(match standing.decay_left {
            Some(left) => format!("CLEAR {}", play_time_text(left)),
            None => "SEEDED".to_string(),
        });
        let (codes, fine) = fine_takers(chart, jobs, stances, civilization, now);
        details.push(match fine {
            Some(fine) => format!("FINE {} @ {}", cr_text(u64::from(fine)), codes.join(", ")),
            None => "FINE NONE".to_string(),
        });
    }
    if standing.offences > 0 {
        details.push(format!("OFFENCES {}/{OFFENCE_STEP}", standing.offences));
    }
    if standing.merits > 0 {
        details.push(format!("MERITS {}/{MERIT_STEP}", standing.merits));
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
/// Enemy refuses the dock, so its Dock line says so.
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
        let short = own_credits < fine;
        let gate = match (docked_here, short) {
            (false, _) => VerbGate::Hidden,
            (true, true) => VerbGate::Disabled,
            (true, false) => VerbGate::Ready,
        };
        rows.push(MapStanceRowView::Settle {
            civilization: enemy,
            fine,
            short,
            gate,
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
    let mut block = spawn_block(
        panel,
        "Stances",
        (MapStanceBlock, Name::new("MapStanceBlock")),
    );
    block
        .entry::<Node>()
        .and_modify(|mut node| node.display = Display::None);
    block.with_children(|block| {
        block.spawn((
            MapStanceRows::default(),
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(metrics::GAP_S),
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
                            column_gap: px(metrics::GAP),
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
                                    text(name, TextRole::Value),
                                    TextLayout::new(Justify::Left, LineBreak::NoWrap),
                                    Pickable::IGNORE
                                )],
                            ));
                            row.spawn((
                                tag(stance_word(*table), stance_badge_kind(*table)),
                                Pickable::IGNORE,
                            ));
                            row.spawn((
                                tag(advancement, UiColorType::TextSecondary),
                                Pickable::IGNORE,
                            ));
                        });
                        for detail in details {
                            list.spawn(text(detail, TextRole::Label));
                        }
                    }
                    MapStanceRowView::Settle {
                        civilization,
                        fine,
                        short,
                        gate,
                    } => {
                        list.spawn((
                            Name::new(format!("MapStanceRow-{index}")),
                            Node {
                                column_gap: px(metrics::GAP),
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
                                    text(&civilization.name(), TextRole::Value),
                                    TextLayout::new(Justify::Left, LineBreak::NoWrap),
                                    Pickable::IGNORE
                                )],
                            ));
                            row.spawn((
                                tag(
                                    stance_word(StanceType::Enemy),
                                    stance_badge_kind(StanceType::Enemy),
                                ),
                                Pickable::IGNORE,
                            ));
                            row.spawn((
                                text(cr_text(u64::from(*fine)), TextRole::Body),
                                Short(*short),
                                TextLayout::new(Justify::Right, LineBreak::NoWrap),
                                Pickable::IGNORE,
                            ));
                            row.spawn((
                                Name::new(format!("MapStancePayFine-{index}")),
                                button(ButtonSpec::control("Pay fine")),
                                ButtonCueType::Owned,
                                gate.clone(),
                            ))
                            .observe(on_pay_fine_button(*civilization));
                        });
                    }
                    MapStanceRowView::Dock(line) => {
                        list.spawn(text(line, TextRole::Label));
                    }
                }
            }
        });
    built.0 = rows;
}
