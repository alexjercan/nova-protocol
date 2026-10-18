//! The map's chart view: the player's [`Chart`] drawn at cell scale.
//!
//! The view swaps the local scene for a grid of cells around the player's
//! cell. A visited cell is a textured tile: one layer per cluster type, each
//! the type's texture ([`ChartTileTextures`]) in the type's colour, or a
//! faint grid for a cell with no cluster body; a cell never entered is an
//! outline. The found stations and wrecks stand as marks in the cell that
//! generated them, and every held job ([`Jobs`]) as a mark in its target's
//! cell. A station mark reads its state: a living station in the living
//! colour, a dead one faint in the danger colour, the player's own in the
//! ally colour, a ring under one that refuses the player a dock right now,
//! and a `n JOBS` tag over one whose board posts jobs the player does not
//! hold. Marks and cell tags are projected UI labels, like the local blips,
//! and each selects what it names. A mark's label shows only a code minted
//! from the chart's found order (or the jobs' accept order), never a name:
//! the open world names a ship by whose it is.
//!
//! The side panel carries the SECTOR block while a find is selected: the
//! find's cell, the route to it from the player's cell, its contents as icon
//! rows, when the player entered it, the finds there, the stock last seen at
//! a station docked there and the jobs held or posted there. With nothing
//! selected it carries the CHARTER block over the discovery log: the chart's
//! own finds ([`Chart::log`]) newest first, each an icon, a label, a cell and
//! how long ago, and each a row that selects its find.
//!
//! Chart space is its own frame on the map layer: the player's cell centre
//! sits at the origin and one cell is [`MAP_CHART_CELL`] units across. The
//! local rings, hub, blips and route hide while it shows, so the two frames
//! never draw over each other.
//!
//! Touch this module when changing what the chart view draws or says.

use bevy::{camera::visibility::RenderLayers, prelude::*, ui_widgets::Activate};
use nova_events::prelude::{GeneratedBodyId, SectorCoord};
use nova_gameplay::prelude::*;
use nova_ship::prelude::DockAdmission;
use nova_ui::{
    prelude::{button, ButtonSpec},
    theme::{ActiveUiTheme, UiColor},
    widget::{ThemedBorder, ThemedFill},
};

use super::{app::*, contacts::*, tiles::ChartTileTextures, *};
use crate::{
    icons::{icon_node, ChartIconType, InterfaceIcons},
    inventory::{category_color, cr_text},
    pane::{control_row, divider, legend_entry, play_menu_select, themed_label, InterfacePaneType},
    terminal::NovaOsAppInput,
    viewer::unlit,
};

/// Width of one cell in chart space, in map units.
pub(crate) const MAP_CHART_CELL: f32 = 60.0;
/// How many cells the grid reaches out from the player's cell on each
/// horizontal axis: a 7 by 7 grid.
pub(crate) const MAP_CHART_REACH: i32 = 3;
/// The orbit radius the chart opens at: the whole grid in the default fov.
pub(crate) const MAP_CHART_RADIUS: f32 =
    MAP_CHART_CELL * (MAP_CHART_REACH as f32 + 0.5) * MAP_FRAMING_MARGIN;
/// How far the wheel may pull back on the chart. The map camera keeps the
/// default 1000 unit far plane, and the far grid corner must stay inside it.
pub(crate) const MAP_CHART_RADIUS_MAX: f32 = 680.0;
/// Spacing of two marks found in one cell, in map units.
const MAP_CHART_MARK_SPACING: f32 = 14.0;
/// Height of a mark over its cell's floor, in map units.
const MAP_CHART_MARK_LIFT: f32 = 8.0;
/// Vertical step between the labels of marks that share a cell: a 14 unit
/// sideways shift is narrower than a label, so stacked labels never overlap.
const MAP_CHART_LABEL_STACK: f32 = 16.0;
/// Side of a tile, as a share of its cell: a gap keeps the grid readable.
const MAP_CHART_TILE_SHARE: f32 = 0.92;

/// One find of the chart the player can select: a visited cell, a station,
/// a wreck or a held job.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MapChartMarkType {
    /// A visited cell.
    Cell(SectorCoord),
    /// A found station.
    Station(GeneratedBodyId),
    /// A found derelict hull.
    Wreck(GeneratedBodyId),
    /// A held job, marked in its target's cell.
    Job(JobId),
}

/// The live facts of a found station that stands in the streamed window,
/// which the chart draws beyond its record. A station out of the window
/// reads the default: it neither refuses nor posts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ChartStationLive {
    /// It refuses the player a dock right now ([`DockAdmission`]).
    pub(crate) refusing: bool,
    /// How many jobs its board posts that the player neither holds nor has
    /// completed.
    pub(crate) posting: u32,
}

/// The chart view's scene, under the map scene root.
#[derive(Component)]
pub(crate) struct MapChartRoot;

/// One label of the chart view over the viewport: a mark's code or a
/// visited cell's coordinate, which selects what it names, or a tag that
/// takes no clicks: the own-ship tag and a station's job count.
#[derive(Component)]
pub(crate) struct MapChartLabel {
    /// Where the label points, in chart space.
    pub(crate) anchor: Vec3,
    /// The mark the label selects.
    pub(crate) mark: Option<MapChartMarkType>,
}

/// The SECTOR block of the side panel, shown in the chart view while a find
/// is selected.
#[derive(Component)]
pub(crate) struct MapChartSectorMarker;

/// The CHARTER block over the discovery log, shown in the chart view while
/// the role has a goal.
#[derive(Component)]
pub(crate) struct MapChartCharterMarker;

/// The rows of the discovery log in the side panel, shown in the chart view
/// with nothing selected.
#[derive(Component)]
pub(crate) struct MapChartLogMarker;

/// One row that selects a find: a discovery log row or a FINDS row of the
/// SECTOR block.
#[derive(Component)]
pub(crate) struct MapChartLogRow(pub(crate) MapChartMarkType);

/// How many finds the discovery log lists, newest first.
pub(crate) const MAP_CHART_LOG_ROWS: usize = 12;

/// The chart code of a mark: a cell's coordinate, a station's or wreck's
/// 1-based place in the chart's found order, or a held job's in the accept
/// order.
pub(crate) fn chart_mark_code(
    chart: &Chart,
    jobs: &Jobs,
    mark: &MapChartMarkType,
) -> Option<String> {
    match mark {
        MapChartMarkType::Cell(coord) => Some(coord.to_string()),
        MapChartMarkType::Station(id) => chart
            .stations()
            .iter()
            .position(|station| station.id == *id)
            .map(|index| format!("STN-{}", index + 1)),
        MapChartMarkType::Wreck(id) => chart
            .wrecks()
            .iter()
            .position(|wreck| wreck.id == *id)
            .map(|index| format!("WRK-{}", index + 1)),
        MapChartMarkType::Job(id) => jobs
            .held()
            .iter()
            .position(|job| job.id == *id)
            .map(|index| format!("JOB-{}", index + 1)),
    }
}

/// The cell a mark stands in: the cell itself, the cell that generated a
/// station or a wreck, or a held job's target cell. A job no longer held has
/// no cell.
pub(crate) fn chart_mark_cell(jobs: &Jobs, mark: &MapChartMarkType) -> Option<SectorCoord> {
    match mark {
        MapChartMarkType::Cell(coord) => Some(*coord),
        MapChartMarkType::Station(id) | MapChartMarkType::Wreck(id) => Some(id.origin),
        MapChartMarkType::Job(id) => jobs.job(id).map(job_cell),
    }
}

/// The cell a held job is marked in: the cell of the body it is about, or
/// the poster's for a procurement, which is done at the hand-in.
pub(crate) fn job_cell(job: &Job) -> SectorCoord {
    job.located_body()
        .map_or(job.target.origin, |body| body.origin)
}

/// The one line a job is called by: its title from the catalog, or by its
/// kind and its item id while no catalog is loaded.
pub(crate) fn job_title(job: &Job, items: Option<&GameItems>) -> String {
    items.map_or_else(
        || match &job.kind {
            JobType::Procure { item, count } => format!("Bring {count} {item}"),
            JobType::Recover {
                wreck_name,
                item,
                count,
                ..
            } => format!("Recover {count} {item} from {wreck_name}"),
            JobType::Escort { .. } | JobType::Distress { .. } | JobType::Neutralize { .. } => {
                job.title(&GameItems::default())
            }
        },
        |items| job.title(items),
    )
}

/// The colour a cluster type's tile layer, icon and word read in.
pub(crate) const fn chart_cell_color(cluster: ClusterType) -> UiColor {
    ChartIconType::of(cluster).color()
}

/// The alpha of a cluster type's tile layer.
const MAP_CHART_TILE_ALPHA: f32 = 0.9;
/// The fill under a visited cell's layers: the player's cell in the own-ship
/// colour, every other visited cell faint.
const MAP_CHART_PLAYER_CELL_FILL: (UiColor, f32) = (UiColor::Primary, 0.16);
const MAP_CHART_VISITED_CELL_FILL: (UiColor, f32) = (UiColor::Secondary, 0.14);
/// The grid of a visited cell with no cluster body.
const MAP_CHART_EMPTY_CELL: (UiColor, f32) = (UiColor::Label, 0.5);
/// The outline of a cell the player never entered.
const MAP_CHART_UNVISITED_CELL: (UiColor, f32) = (UiColor::Secondary, 0.6);
/// The alpha a dead station's mark fades to.
const MAP_CHART_DEAD_STATION_ALPHA: f32 = 0.3;

/// The word a cell label, the panel and the legend name a cluster type by:
/// what the player finds there.
pub(crate) const fn chart_cluster_word(cluster: ClusterType) -> &'static str {
    match cluster {
        ClusterType::AsteroidRich => "ASTEROIDS",
        ClusterType::RockOnly => "ROCKS",
        ClusterType::PlanetHeavy => "PLANETS",
        ClusterType::DerelictField => "DERELICTS",
    }
}

/// What a visited cell holds, in words: its cluster types joined, or empty.
fn chart_cell_words(cell: &ChartCell) -> String {
    if cell.clusters.is_empty() {
        "EMPTY".to_string()
    } else {
        cell.clusters
            .iter()
            .map(|cluster| chart_cluster_word(*cluster))
            .collect::<Vec<_>>()
            .join(" + ")
    }
}

/// The icon of a visited cell: its first cluster type's, or the bare cell.
fn chart_cell_icon(cell: &ChartCell) -> ChartIconType {
    cell.clusters
        .first()
        .map_or(ChartIconType::Sector, |cluster| ChartIconType::of(*cluster))
}

/// What a station mark is, as the panel's kind line names it, and its
/// colour: dead, the player's (in the Map's ally colour) or a civilization's.
/// A dead station is dead whoever owned it.
pub(crate) const fn chart_station_kind(derelict: bool, owned: bool) -> (&'static str, UiColor) {
    match (derelict, owned) {
        (true, _) => ("DEAD STATION", UiColor::Danger),
        (false, true) => ("OWNED STATION", UiColor::Info),
        (false, false) => ("LIVING STATION", UiColor::Nominal),
    }
}

/// The colour of a wreck mark.
pub(crate) const MAP_CHART_WRECK_COLOR: UiColor = ChartIconType::Wreck.color();
/// The colour of a held job's mark and label.
pub(crate) const MAP_CHART_JOB_COLOR: UiColor = ChartIconType::Job.color();

/// How long ago a play-time stamp was, read against the clock: `just now`
/// under a minute, else whole minutes, else whole hours. A stamp ahead of
/// the clock reads `just now`.
pub(crate) fn relative_time_text(now: f64, at: f64) -> String {
    let elapsed = now - at;
    if !elapsed.is_finite() || elapsed < 60.0 {
        "just now".to_string()
    } else if elapsed < 3600.0 {
        format!("{} min ago", (elapsed / 60.0).floor())
    } else {
        format!("{} h ago", (elapsed / 3600.0).floor())
    }
}

/// The route from the player's cell to `to`, one leg per axis: `2 cells +X,
/// 1 cell -Z`, or `This cell`.
pub(crate) fn route_hint(from: SectorCoord, to: SectorCoord) -> String {
    let legs = [
        (to.x - from.x, 'X'),
        (to.y - from.y, 'Y'),
        (to.z - from.z, 'Z'),
    ];
    let parts: Vec<String> = legs
        .iter()
        .filter(|(delta, _)| *delta != 0)
        .map(|(delta, axis)| {
            let cells = if delta.abs() == 1 { "cell" } else { "cells" };
            let sign = if *delta > 0 { '+' } else { '-' };
            format!("{} {cells} {sign}{axis}", delta.abs())
        })
        .collect();
    if parts.is_empty() {
        "This cell".to_string()
    } else {
        parts.join(", ")
    }
}

/// Where a cell sits in chart space, from the player's cell.
fn chart_cell_position(cell: SectorCoord, player_cell: SectorCoord) -> Vec3 {
    Vec3::new(
        (cell.x - player_cell.x) as f32,
        (cell.y - player_cell.y) as f32,
        (cell.z - player_cell.z) as f32,
    ) * MAP_CHART_CELL
}

/// Put the camera on the chart's opening framing: the player's cell at the
/// centre of the whole grid.
pub(crate) fn frame_chart(orbit: &mut MapOrbit) {
    orbit.center = Vec3::ZERO;
    orbit.radius = MAP_CHART_RADIUS;
    orbit.theta = MAP_THETA_DEFAULT;
    orbit.phi = MAP_PHI_DEFAULT;
}

/// Switch between the local view and the chart view on `map_chart` or the
/// Chart button, while the map owns the screen. A scene with no streamed
/// world has no chart: the switch writes a note and stays local. Each switch
/// drops the chart selection and reframes the view it enters; a switch into
/// the chart that a Show on Map asked for selects its mark instead
/// ([`MapRuntime::chart_show`]).
pub(crate) fn toggle_map_chart(
    input: NovaOsAppInput,
    mut runtime: ResMut<MapRuntime>,
    edge: Option<Res<ChartSectorEdge>>,
    contacts: MapContacts,
    mut q_camera: Query<&mut MapOrbit, With<MapCameraMarker>>,
) {
    let button = std::mem::take(&mut runtime.chart_requested);
    if !input.app_is_active(InterfacePaneType::Map) || !(button || input.just_pressed("map_chart"))
    {
        return;
    }
    if edge.is_none() {
        runtime.chart_show = None;
        runtime.goto_note = Some(("NO CHART: NO OPEN WORLD".to_string(), 2.5));
        return;
    }
    runtime.chart = !runtime.chart;
    runtime.chart_selected = runtime.chart_show.take().filter(|_| runtime.chart);
    if let Ok(mut orbit) = q_camera.single_mut() {
        if runtime.chart {
            frame_chart(&mut orbit);
        } else {
            reframe_map(&mut orbit, &contacts, &mut runtime);
        }
    }
}

/// Request the chart switch when the Chart button is activated.
/// [`toggle_map_chart`] takes it with the `map_chart` key.
pub(crate) fn on_map_chart_button(_activate: On<Activate>, mut runtime: ResMut<MapRuntime>) {
    runtime.chart_requested = true;
}

/// The live facts of every found station standing in the streamed window,
/// in id order: whether it refuses `player` a dock and how many jobs its
/// board posts that `jobs` neither holds nor has completed.
fn chart_station_live(
    chart: &Chart,
    jobs: &Jobs,
    player: Option<Entity>,
    stations: &Query<(Entity, &GeneratedBodyId, Option<&JobBoard>), With<StationMarker>>,
    admission: &DockAdmission,
) -> Vec<(GeneratedBodyId, ChartStationLive)> {
    let mut live: Vec<(GeneratedBodyId, ChartStationLive)> = stations
        .iter()
        .filter(|(_, id, _)| chart.station(id).is_some())
        .map(|(entity, id, board)| {
            let posting = board.map_or(0, |board| {
                board
                    .0
                    .iter()
                    .filter(|job| !jobs.holds(&job.id) && !jobs.is_completed(&job.id))
                    .count() as u32
            });
            let refusing = player.is_some_and(|player| admission.refusal(player, entity).is_some());
            (id.clone(), ChartStationLive { refusing, posting })
        })
        .collect();
    live.sort_by(|a, b| a.0.cmp(&b.0));
    live
}

/// The live facts of the found station `id`, or the default off the window.
fn live_of(live: &[(GeneratedBodyId, ChartStationLive)], id: &GeneratedBodyId) -> ChartStationLive {
    live.iter()
        .find(|(each, _)| each == id)
        .map_or_else(ChartStationLive::default, |(_, facts)| *facts)
}

/// What the chart scene was last built for, beyond the chart, the jobs and
/// the theme, whose change ticks say so themselves.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ChartSceneBuild {
    player_cell: SectorCoord,
    selected: Option<MapChartMarkType>,
    live: Vec<(GeneratedBodyId, ChartStationLive)>,
}

/// Build the chart scene and its labels while the chart view shows, rebuild
/// them when the chart, the jobs, the selection, the player's cell, a
/// station's live facts or the theme changes, and tear them down when the
/// view goes. Hides the local rings and hub meanwhile.
#[expect(
    clippy::too_many_arguments,
    reason = "the chart, the live stations, the scene assets and the viewport its labels sit on"
)]
pub(crate) fn manage_map_chart(
    mut commands: Commands,
    mut runtime: ResMut<MapRuntime>,
    chart: Res<Chart>,
    jobs: Res<Jobs>,
    edge: Option<Res<ChartSectorEdge>>,
    contacts: MapContacts,
    admission: DockAdmission,
    q_stations: Query<(Entity, &GeneratedBodyId, Option<&JobBoard>), With<StationMarker>>,
    theme: Res<ActiveUiTheme>,
    tiles: Res<ChartTileTextures>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    q_viewport: Query<Entity, With<MapViewportMarker>>,
    mut q_anchor: Query<&mut Visibility, With<MapFocusAnchor>>,
    mut built_for: Local<Option<ChartSceneBuild>>,
) {
    let local = if runtime.chart {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    for mut visibility in &mut q_anchor {
        visibility.set_if_neq(local);
    }
    let wanted = runtime.active && runtime.chart && runtime.scene_root.is_some();
    let (Some(edge), Some(mut meshes), Some(mut materials), Ok(viewport), true) =
        (edge, meshes, materials, q_viewport.single(), wanted)
    else {
        if runtime.chart_root.is_some() || !runtime.chart_labels.is_empty() {
            drop_map_chart(&mut commands, &mut runtime);
        }
        if runtime.chart_player_cell.is_some() || !runtime.chart_live.is_empty() {
            runtime.chart_player_cell = None;
            runtime.chart_live.clear();
        }
        *built_for = None;
        return;
    };
    let player_cell = SectorCoord::containing(Meters3::from_engine(contacts.focus()), edge.0);
    let player = contacts.player_frame().map(|(entity, _, _)| entity);
    let build = ChartSceneBuild {
        player_cell,
        selected: runtime.chart_selected.clone(),
        live: chart_station_live(&chart, &jobs, player, &q_stations, &admission),
    };
    if runtime.chart_root.is_some()
        && built_for.as_ref() == Some(&build)
        && !chart.is_changed()
        && !jobs.is_changed()
        && !theme.is_changed()
    {
        return;
    }
    drop_map_chart(&mut commands, &mut runtime);
    runtime.chart_player_cell = Some(player_cell);
    if runtime.chart_live != build.live {
        runtime.chart_live.clone_from(&build.live);
    }
    let selected_cell = build
        .selected
        .as_ref()
        .and_then(|mark| chart_mark_cell(&jobs, mark));
    *built_for = Some(build);
    let Some(scene_root) = runtime.scene_root else {
        return;
    };
    let root = commands
        .spawn((
            MapChartRoot,
            Name::new("NovaOsMapChart"),
            Transform::default(),
            Visibility::Visible,
            ChildOf(scene_root),
        ))
        .id();
    runtime.chart_root = Some(root);
    let mut spawn_mesh = |mesh: Handle<Mesh>, material: Handle<StandardMaterial>, at: Transform| {
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            at,
            RenderLayers::layer(MAP_LAYER),
            ChildOf(root),
        ));
    };
    let textured = |texture: Handle<Image>, color: Color| StandardMaterial {
        base_color: color,
        base_color_texture: Some(texture),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    };

    // The grid: cell borders on the player's layer.
    let span = MAP_CHART_CELL * (2 * MAP_CHART_REACH + 1) as f32;
    let line_material = materials.add(unlit(theme.color_alpha(UiColor::Secondary, 0.5)));
    let along_x = meshes.add(Cuboid::new(span, 0.6, 0.6));
    let along_z = meshes.add(Cuboid::new(0.6, 0.6, span));
    for line in 0..=(2 * MAP_CHART_REACH + 1) {
        let offset = (line as f32 - MAP_CHART_REACH as f32 - 0.5) * MAP_CHART_CELL;
        spawn_mesh(
            along_x.clone(),
            line_material.clone(),
            Transform::from_xyz(0.0, 0.0, offset),
        );
        spawn_mesh(
            along_z.clone(),
            line_material.clone(),
            Transform::from_xyz(offset, 0.0, 0.0),
        );
    }

    // The tiles: every cell of the grid on the player's layer, plus every
    // visited cell off it. A visited cell is a fill under one textured layer
    // per cluster type, or the empty grid, with its coordinate at its
    // corner; a cell never entered is an outline.
    let tile = meshes.add(Plane3d::default().mesh().size(
        MAP_CHART_CELL * MAP_CHART_TILE_SHARE,
        MAP_CHART_CELL * MAP_CHART_TILE_SHARE,
    ));
    let mut cells: Vec<SectorCoord> = chart.visited().map(|(coord, _)| coord).collect();
    for dx in -MAP_CHART_REACH..=MAP_CHART_REACH {
        for dz in -MAP_CHART_REACH..=MAP_CHART_REACH {
            let coord = SectorCoord::new(player_cell.x + dx, player_cell.y, player_cell.z + dz);
            if !cells.contains(&coord) {
                cells.push(coord);
            }
        }
    }
    let mut labels = Vec::new();
    for coord in cells {
        let at = chart_cell_position(coord, player_cell);
        match chart.cell(coord) {
            Some(cell) => {
                let (color, alpha) = if coord == player_cell {
                    MAP_CHART_PLAYER_CELL_FILL
                } else {
                    MAP_CHART_VISITED_CELL_FILL
                };
                spawn_mesh(
                    tile.clone(),
                    materials.add(unlit(theme.color_alpha(color, alpha))),
                    Transform::from_translation(at - Vec3::Y * 0.3),
                );
                if cell.clusters.is_empty() {
                    let (color, alpha) = MAP_CHART_EMPTY_CELL;
                    spawn_mesh(
                        tile.clone(),
                        materials.add(textured(
                            tiles.grid.clone(),
                            theme.color_alpha(color, alpha),
                        )),
                        Transform::from_translation(at),
                    );
                }
                for (index, cluster) in cell.clusters.iter().enumerate() {
                    spawn_mesh(
                        tile.clone(),
                        materials.add(textured(
                            tiles.cluster(*cluster),
                            theme.color_alpha(chart_cell_color(*cluster), MAP_CHART_TILE_ALPHA),
                        )),
                        Transform::from_translation(at + Vec3::Y * (0.2 + index as f32 * 0.2)),
                    );
                }
                let corner = Vec3::new(-0.38, 0.0, -0.38) * MAP_CHART_CELL;
                labels.push((
                    at + corner,
                    Some(MapChartMarkType::Cell(coord)),
                    coord.to_string(),
                    coord.to_string(),
                    UiColor::Label,
                    10.0,
                ));
            }
            None => {
                let (color, alpha) = MAP_CHART_UNVISITED_CELL;
                spawn_mesh(
                    tile.clone(),
                    materials.add(textured(
                        tiles.outline.clone(),
                        theme.color_alpha(color, alpha),
                    )),
                    Transform::from_translation(at),
                );
            }
        }
        if selected_cell == Some(coord) {
            spawn_mesh(
                tile.clone(),
                materials.add(textured(
                    tiles.outline.clone(),
                    theme.color(UiColor::Accent),
                )),
                Transform::from_translation(at + Vec3::Y * 1.2),
            );
        }
    }

    // Stations and wrecks, side by side in the cell that generated them,
    // and held jobs in their target's cell.
    let marks: Vec<(MapChartMarkType, SectorCoord, UiColor)> = chart
        .stations()
        .iter()
        .map(|station| {
            (
                MapChartMarkType::Station(station.id.clone()),
                station.id.origin,
                chart_station_kind(station.derelict, station.owned).1,
            )
        })
        .chain(chart.wrecks().iter().map(|wreck| {
            (
                MapChartMarkType::Wreck(wreck.id.clone()),
                wreck.id.origin,
                MAP_CHART_WRECK_COLOR,
            )
        }))
        .chain(jobs.held().iter().map(|job| {
            (
                MapChartMarkType::Job(job.id.clone()),
                job_cell(job),
                MAP_CHART_JOB_COLOR,
            )
        }))
        .collect();
    let station_mesh = meshes.add(Cuboid::new(9.0, 9.0, 9.0));
    let refusal_ring = meshes.add(Torus::new(8.0, 9.5));
    let wreck_mesh = meshes.add(Tetrahedron::default());
    let job_mesh = meshes.add(Cuboid::new(4.0, 14.0, 4.0));
    for (mark, cell, color) in &marks {
        let in_cell: Vec<&MapChartMarkType> = marks
            .iter()
            .filter(|(_, other, _)| other == cell)
            .map(|(other, _, _)| other)
            .collect();
        let slot = in_cell.iter().position(|other| *other == mark).unwrap_or(0);
        let shift = (slot as f32 - (in_cell.len() - 1) as f32 * 0.5) * MAP_CHART_MARK_SPACING;
        let at =
            chart_cell_position(*cell, player_cell) + Vec3::new(shift, MAP_CHART_MARK_LIFT, 0.0);
        let label_at = at + Vec3::Y * (slot as f32 * MAP_CHART_LABEL_STACK);
        match mark {
            MapChartMarkType::Station(id) => {
                let dead = chart.station(id).is_some_and(|station| station.derelict);
                let alpha = if dead {
                    MAP_CHART_DEAD_STATION_ALPHA
                } else {
                    1.0
                };
                spawn_mesh(
                    station_mesh.clone(),
                    materials.add(unlit(theme.color_alpha(*color, alpha))),
                    Transform::from_translation(at),
                );
                let live = live_of(&runtime.chart_live, id);
                if live.refusing {
                    spawn_mesh(
                        refusal_ring.clone(),
                        materials.add(unlit(theme.color(UiColor::Danger))),
                        Transform::from_translation(at - Vec3::Y * 4.0),
                    );
                }
                if live.posting > 0 {
                    if let Some(code) = chart_mark_code(&chart, &jobs, mark) {
                        labels.push((
                            // Above every stacked label of the cell.
                            at + Vec3::Y * (in_cell.len() as f32 * MAP_CHART_LABEL_STACK + 4.0),
                            None,
                            format!("{} JOBS", live.posting),
                            format!("{code} jobs"),
                            MAP_CHART_JOB_COLOR,
                            10.0,
                        ));
                    }
                }
            }
            MapChartMarkType::Wreck(_) => {
                spawn_mesh(
                    wreck_mesh.clone(),
                    materials.add(unlit(theme.color(*color))),
                    Transform::from_translation(at).with_scale(Vec3::splat(10.0)),
                );
            }
            MapChartMarkType::Job(_) => {
                spawn_mesh(
                    job_mesh.clone(),
                    materials.add(unlit(theme.color(*color))),
                    Transform::from_translation(at + Vec3::Y * 3.0),
                );
            }
            MapChartMarkType::Cell(_) => unreachable!("a cell is drawn as a tile, not a mark"),
        }
        if let Some(code) = chart_mark_code(&chart, &jobs, mark) {
            labels.push((
                label_at,
                Some(mark.clone()),
                code.clone(),
                code,
                *color,
                12.0,
            ));
        }
    }

    // The own ship, where it stands in its cell.
    let within = (Meters3::from_engine(contacts.focus()).0 - player_cell.centre(edge.0).0)
        / edge.0.get()
        * MAP_CHART_CELL;
    spawn_mesh(
        meshes.add(Sphere::new(3.5)),
        materials.add(unlit(theme.color(MapContactKind::OwnShip.color()))),
        Transform::from_translation(within + Vec3::Y * MAP_CHART_MARK_LIFT * 0.5),
    );
    labels.push((
        within + Vec3::Y * MAP_CHART_MARK_LIFT * 0.5,
        None,
        "YOU".to_string(),
        "YOU".to_string(),
        MapContactKind::OwnShip.color(),
        11.0,
    ));

    for (anchor, mark, text, name, color, size) in labels {
        let label = spawn_chart_label(
            &mut commands,
            viewport,
            anchor,
            mark,
            &text,
            &name,
            color,
            size,
        );
        runtime.chart_labels.push(label);
    }
}

/// Despawn the chart scene and its labels, if they stand.
pub(crate) fn drop_map_chart(commands: &mut Commands, runtime: &mut MapRuntime) {
    if let Some(root) = runtime.chart_root.take() {
        commands.entity(root).try_despawn();
    }
    for label in runtime.chart_labels.drain(..) {
        commands.entity(label).try_despawn();
    }
}

/// Spawn one chart label under the viewport, hidden until projected. A
/// mark's label is a button named `MapChartMark{name}` that selects the
/// mark; a tag takes no clicks.
#[expect(
    clippy::too_many_arguments,
    reason = "a label's place, mark, text, name, colour and size under the viewport"
)]
fn spawn_chart_label(
    commands: &mut Commands,
    viewport: Entity,
    anchor: Vec3,
    mark: Option<MapChartMarkType>,
    text: &str,
    name: &str,
    color: UiColor,
    size: f32,
) -> Entity {
    let clickable = mark.is_some();
    let mut label = commands.spawn((
        MapChartLabel { anchor, mark },
        Node {
            position_type: PositionType::Absolute,
            padding: UiRect::axes(Val::Px(4.0), Val::Px(1.0)),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        Visibility::Hidden,
        ZIndex(i32::from(clickable)),
        BackgroundColor(Color::NONE),
        ThemedFill::alpha(UiColor::Void, 0.8),
        BorderColor::all(Color::NONE),
        ThemedBorder::alpha(UiColor::Secondary, if clickable { 0.3 } else { 0.0 }),
        ChildOf(viewport),
        children![(themed_label(text, size, color), Pickable::IGNORE)],
    ));
    if clickable {
        label
            .insert((
                bevy::ui_widgets::Button,
                Name::new(format!("MapChartMark{name}")),
            ))
            .observe(on_map_chart_label_click);
    } else {
        label.insert(Pickable::IGNORE);
    }
    label.id()
}

/// Select `mark` in the chart view, with one click if the selection changes.
fn select_chart_mark(
    runtime: &mut MapRuntime,
    mark: &MapChartMarkType,
    bank: Option<&SoundBank<UiSfx>>,
    commands: &mut Commands,
) {
    if runtime.chart_selected.as_ref() != Some(mark) {
        runtime.chart_selected = Some(mark.clone());
        play_menu_select(commands, bank);
    }
}

/// Select a chart mark when its label is activated, with one click if the
/// selection changes.
fn on_map_chart_label_click(
    activate: On<Activate>,
    q_label: Query<&MapChartLabel>,
    mut runtime: ResMut<MapRuntime>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    let Ok(MapChartLabel {
        mark: Some(mark), ..
    }) = q_label.get(activate.entity)
    else {
        return;
    };
    select_chart_mark(&mut runtime, mark, bank.as_deref(), &mut commands);
}

/// Select a find when its discovery log row or FINDS row is activated.
fn on_map_chart_log_row_click(
    activate: On<Activate>,
    q_row: Query<&MapChartLogRow>,
    mut runtime: ResMut<MapRuntime>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    let Ok(MapChartLogRow(mark)) = q_row.get(activate.entity) else {
        return;
    };
    select_chart_mark(&mut runtime, mark, bank.as_deref(), &mut commands);
}

/// A section heading of a chart panel block.
fn block_heading(parent: &mut ChildSpawnerCommands, text: &str) {
    parent.spawn((themed_label(text, 11.0, UiColor::Label), Pickable::IGNORE));
}

/// A `label  value` line of a chart panel block.
fn block_fact(parent: &mut ChildSpawnerCommands, label: &str, value: &str) {
    parent
        .spawn(Node {
            column_gap: px(8),
            ..control_row(JustifyContent::FlexStart)
        })
        .with_children(|row| {
            row.spawn((
                Node {
                    width: px(72),
                    flex_shrink: 0.0,
                    ..default()
                },
                children![(themed_label(label, 12.0, UiColor::Label), Pickable::IGNORE)],
            ));
            row.spawn((
                themed_label(value, 13.0, UiColor::Primary),
                Pickable::IGNORE,
            ));
        });
}

/// A note line of a chart panel block.
fn block_note(parent: &mut ChildSpawnerCommands, text: &str) {
    parent.spawn((themed_label(text, 12.0, UiColor::Body), Pickable::IGNORE));
}

/// One icon row of a chart panel block: an icon in its tint, a label that
/// takes the width and clips, and the trailing cells in the Label colour.
/// Returns the row for the caller to make a button of.
fn spawn_icon_row<'a>(
    parent: &'a mut ChildSpawnerCommands,
    icon: Option<(Handle<Image>, UiColor)>,
    label: &str,
    label_color: UiColor,
    trailing: &[(&str, f32, UiColor)],
) -> EntityCommands<'a> {
    let mut row = parent.spawn(Node {
        column_gap: px(8),
        padding: UiRect::axes(px(4), px(1)),
        border: UiRect::all(px(1)),
        ..control_row(JustifyContent::FlexStart)
    });
    row.with_children(|row| {
        if let Some((image, tint)) = icon {
            row.spawn(icon_node(image, tint, 16.0));
        }
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
                themed_label(label, 13.0, label_color),
                TextLayout::new(Justify::Left, LineBreak::NoWrap),
                Pickable::IGNORE
            )],
        ));
        for (text, size, color) in trailing {
            row.spawn((
                themed_label(text, *size, *color),
                TextLayout::new(Justify::Right, LineBreak::NoWrap),
                Pickable::IGNORE,
            ));
        }
    });
    row
}

/// Make `row` a button named `name` that selects `mark`, born with its
/// edge: the accent when it is the selection, so a rebuild under a selected
/// row never shows it plain for a frame.
fn select_row(row: &mut EntityCommands, name: String, mark: MapChartMarkType, selected: bool) {
    let (edge, alpha) = chart_log_row_edge(selected);
    row.insert((
        MapChartLogRow(mark),
        bevy::ui_widgets::Button,
        Name::new(name),
        BackgroundColor(Color::NONE),
        BorderColor::all(Color::NONE),
        ThemedBorder::alpha(edge, alpha),
    ))
    .observe(on_map_chart_log_row_click);
}

/// The SECTOR block's slot in the side panel, over the CHARTER block, hidden
/// off the chart view and with nothing selected.
/// [`update_map_chart_sector`] fills it.
pub(crate) fn spawn_map_chart_sector(panel: &mut ChildSpawnerCommands) {
    panel.spawn((
        MapChartSectorMarker,
        Name::new("MapChartSector"),
        Node {
            display: Display::None,
            flex_direction: FlexDirection::Column,
            row_gap: px(4),
            ..default()
        },
    ));
}

/// What a chart panel block was last built for, so it rebuilds only on a
/// difference: the selection, the player's cell, the stations' live facts
/// and the play minute every relative time reads in.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ChartPanelBuild {
    selected: Option<MapChartMarkType>,
    player_cell: Option<SectorCoord>,
    live: Vec<(GeneratedBodyId, ChartStationLive)>,
    minute: u64,
}

impl ChartPanelBuild {
    fn of(runtime: &MapRuntime, clock: &WorldClock) -> Self {
        Self {
            selected: runtime.chart_selected.clone(),
            player_cell: runtime.chart_player_cell,
            live: runtime.chart_live.clone(),
            minute: (clock.play_seconds.max(0.0) / 60.0).floor() as u64,
        }
    }
}

/// The name a found station is called by in a row: its code, then the name
/// seen on the last dock if the player docked there.
fn station_row_label(chart: &Chart, jobs: &Jobs, station: &ChartStation) -> String {
    let code = chart_mark_code(chart, jobs, &MapChartMarkType::Station(station.id.clone()))
        .unwrap_or_default();
    match &station.summary {
        Some(summary) => format!("{code} {}", summary.name),
        None => code,
    }
}

/// Show the SECTOR block while the chart view shows with a find selected,
/// rebuild it whenever the selection, the chart, the jobs, the player's
/// cell, a station's live facts or the play minute changes, and hide it
/// otherwise.
#[expect(
    clippy::too_many_arguments,
    reason = "the selection, the records it reads and the icons the rows draw"
)]
pub(crate) fn update_map_chart_sector(
    mut commands: Commands,
    runtime: Res<MapRuntime>,
    chart: Res<Chart>,
    jobs: Res<Jobs>,
    items: Option<Res<GameItems>>,
    clock: Res<WorldClock>,
    icons: Res<InterfaceIcons>,
    mut q_block: Query<(Entity, &mut Node), With<MapChartSectorMarker>>,
    mut built: Local<Option<ChartPanelBuild>>,
) {
    let Ok((block, mut node)) = q_block.single_mut() else {
        return;
    };
    let selected_cell = runtime
        .chart_selected
        .as_ref()
        .and_then(|mark| chart_mark_cell(&jobs, mark));
    let shown = runtime.active && runtime.chart && selected_cell.is_some();
    let display = if shown { Display::Flex } else { Display::None };
    if node.display != display {
        node.display = display;
    }
    let (Some(coord), Some(player_cell), true) = (selected_cell, runtime.chart_player_cell, shown)
    else {
        if built.is_some() {
            commands.entity(block).despawn_related::<Children>();
            *built = None;
        }
        return;
    };
    let build = ChartPanelBuild::of(&runtime, &clock);
    if built.as_ref() == Some(&build) && !chart.is_changed() && !jobs.is_changed() {
        return;
    }
    *built = Some(build);
    let selected = runtime.chart_selected.as_ref();
    commands
        .entity(block)
        .despawn_related::<Children>()
        .with_children(|block| {
            block
                .spawn(control_row(JustifyContent::SpaceBetween))
                .with_children(|head| {
                    head.spawn((
                        themed_label(&format!("SECTOR {coord}"), 15.0, UiColor::Primary),
                        Pickable::IGNORE,
                    ));
                    head.spawn((
                        themed_label(&route_hint(player_cell, coord), 12.0, UiColor::Label),
                        Pickable::IGNORE,
                    ));
                });
            block.spawn(divider());
            block_heading(block, "CONTENTS");
            match chart.cell(coord) {
                Some(cell) if cell.clusters.is_empty() => block_note(block, "Empty space."),
                Some(cell) => {
                    for cluster in &cell.clusters {
                        let icon = ChartIconType::of(*cluster);
                        spawn_icon_row(
                            block,
                            Some((icons.chart(icon), icon.color())),
                            icon.label(),
                            UiColor::Primary,
                            &[],
                        );
                    }
                }
                None => block_note(block, "Not entered yet: contents unknown."),
            }
            if let Some(cell) = chart.cell(coord) {
                block.spawn(divider());
                block_fact(
                    block,
                    "Entered",
                    &format!(
                        "{}, {}",
                        play_time_text(cell.entered_at),
                        relative_time_text(clock.play_seconds, cell.entered_at)
                    ),
                );
            }

            // The finds: the stations and wrecks the cell generated.
            let stations: Vec<&ChartStation> = chart
                .stations()
                .iter()
                .filter(|station| station.id.origin == coord)
                .collect();
            let wrecks: Vec<&ChartWreck> = chart
                .wrecks()
                .iter()
                .filter(|wreck| wreck.id.origin == coord)
                .collect();
            if !stations.is_empty() || !wrecks.is_empty() {
                block.spawn(divider());
                block_heading(block, "FINDS");
            }
            for station in &stations {
                let mark = MapChartMarkType::Station(station.id.clone());
                let code = chart_mark_code(&chart, &jobs, &mark).unwrap_or_default();
                let (_, color) = chart_station_kind(station.derelict, station.owned);
                let live = live_of(&runtime.chart_live, &station.id);
                let mut facts: Vec<String> = Vec::new();
                if station.derelict {
                    facts.push("dead".to_string());
                } else if station.owned {
                    facts.push("yours".to_string());
                }
                if live.refusing {
                    facts.push("refuses docking".to_string());
                }
                if live.posting > 0 {
                    facts.push(format!("posts {} jobs", live.posting));
                }
                if facts.is_empty() {
                    facts.push("living".to_string());
                }
                let facts = facts.join(", ");
                let mut row = spawn_icon_row(
                    block,
                    Some((icons.chart(ChartIconType::Station), color)),
                    &station_row_label(&chart, &jobs, station),
                    color,
                    &[(&facts, 12.0, UiColor::Label)],
                );
                select_row(
                    &mut row,
                    format!("MapChartFind{code}"),
                    mark.clone(),
                    selected == Some(&mark),
                );
            }
            for wreck in &wrecks {
                let mark = MapChartMarkType::Wreck(wreck.id.clone());
                let code = chart_mark_code(&chart, &jobs, &mark).unwrap_or_default();
                let found = format!(
                    "found {}",
                    relative_time_text(clock.play_seconds, wreck.found_at)
                );
                let mut row = spawn_icon_row(
                    block,
                    Some((icons.chart(ChartIconType::Wreck), MAP_CHART_WRECK_COLOR)),
                    &code,
                    MAP_CHART_WRECK_COLOR,
                    &[(&found, 12.0, UiColor::Label)],
                );
                select_row(
                    &mut row,
                    format!("MapChartFind{code}"),
                    mark.clone(),
                    selected == Some(&mark),
                );
            }

            // What the player last saw at each station docked at here.
            for station in &stations {
                let Some(summary) = &station.summary else {
                    continue;
                };
                block.spawn(divider());
                block_heading(block, &format!("{} LAST SEEN", summary.name.to_uppercase()));
                if station.derelict {
                    block_note(block, "Dead. Nothing trades here.");
                    continue;
                }
                let services = if summary.services.is_empty() {
                    "none".to_string()
                } else {
                    summary
                        .services
                        .iter()
                        .map(|service| format!("{service:?}").to_lowercase())
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                block_fact(block, "Services", &services);
                if summary.stock.is_empty() {
                    block_note(block, "No stock seen.");
                }
                for (item, count) in &summary.stock {
                    stock_row(block, &icons, items.as_deref(), item, *count);
                }
                if !summary.stored.is_empty() {
                    block_heading(block, "STORED");
                    for (item, count) in &summary.stored {
                        stock_row(block, &icons, items.as_deref(), item, *count);
                    }
                }
            }

            // The jobs here: held jobs about this cell, and the boards that
            // post.
            let held: Vec<&Job> = jobs
                .held()
                .iter()
                .filter(|job| job_cell(job) == coord)
                .collect();
            let posting: Vec<(&ChartStation, u32)> = stations
                .iter()
                .map(|station| (*station, live_of(&runtime.chart_live, &station.id).posting))
                .filter(|(_, posting)| *posting > 0)
                .collect();
            if !held.is_empty() || !posting.is_empty() {
                block.spawn(divider());
                block_heading(block, "JOBS HERE");
            }
            for job in held {
                let mark = MapChartMarkType::Job(job.id.clone());
                let code = chart_mark_code(&chart, &jobs, &mark).unwrap_or_default();
                let state = if jobs.is_done(&job.id) {
                    "done"
                } else {
                    "held"
                };
                let mut row = spawn_icon_row(
                    block,
                    Some((icons.chart(ChartIconType::Job), MAP_CHART_JOB_COLOR)),
                    &job_title(job, items.as_deref()),
                    UiColor::Primary,
                    &[(state, 12.0, UiColor::Label)],
                );
                select_row(
                    &mut row,
                    format!("MapChartFind{code}"),
                    mark.clone(),
                    selected == Some(&mark),
                );
            }
            for (station, count) in posting {
                let code = chart_mark_code(
                    &chart,
                    &jobs,
                    &MapChartMarkType::Station(station.id.clone()),
                )
                .unwrap_or_default();
                spawn_icon_row(
                    block,
                    Some((icons.chart(ChartIconType::Job), MAP_CHART_JOB_COLOR)),
                    &format!("{code} posts {count} jobs"),
                    UiColor::Primary,
                    &[("open", 12.0, UiColor::Label)],
                );
            }
        });
}

/// One stock row: the item's category icon and name from the catalog, or
/// its id with no icon while no catalog is loaded, and its count.
fn stock_row(
    block: &mut ChildSpawnerCommands,
    icons: &InterfaceIcons,
    items: Option<&GameItems>,
    item: &ItemDesignId,
    count: u32,
) {
    let design = items.and_then(|items| items.get(item));
    let icon = design.map(|design| {
        (
            icons.category(design.category),
            category_color(design.category),
        )
    });
    let name = design.map_or_else(|| item.to_string(), |design| design.name.clone());
    spawn_icon_row(
        block,
        icon,
        &name,
        UiColor::Primary,
        &[(&format!("x{count}"), 12.0, UiColor::Body)],
    );
}

/// The CHARTER block's slot in the side panel, over the discovery log,
/// hidden off the chart view. [`update_map_chart_charter`] fills it.
pub(crate) fn spawn_map_chart_charter(panel: &mut ChildSpawnerCommands) {
    panel.spawn((
        MapChartCharterMarker,
        Name::new("MapChartCharter"),
        Node {
            display: Display::None,
            flex_direction: FlexDirection::Column,
            row_gap: px(2),
            ..default()
        },
    ));
}

/// The CHARTER block's lines: a heading, the goal's title, one
/// `label  have / need` row per line coloured met or open, and a last line
/// naming the play time the goal was met, or that it stands.
pub(crate) fn charter_block_lines(
    progress: &GoalProgress,
    record: &GoalRecord,
) -> Vec<(String, UiColor)> {
    let mut lines = vec![
        ("CHARTER".to_string(), UiColor::Label),
        (progress.title.clone(), UiColor::Label),
    ];
    for line in &progress.lines {
        let color = if line.have >= line.need {
            UiColor::Accent
        } else {
            UiColor::Secondary
        };
        lines.push((
            format!("{}  {} / {}", line.label, line.have, line.need),
            color,
        ));
    }
    lines.push(match record.met_at {
        Some(at) => (format!("Met at {}", play_time_text(at)), UiColor::Label),
        None => ("Standing".to_string(), UiColor::Label),
    });
    lines
}

/// Show the CHARTER block while the chart view shows and the role has a
/// goal, rebuild its lines whenever the progress or the record changes, and
/// hide it off the chart view or with no goal.
pub(crate) fn update_map_chart_charter(
    mut commands: Commands,
    runtime: Res<MapRuntime>,
    progress: Res<GoalProgress>,
    record: Res<GoalRecord>,
    mut q_block: Query<(Entity, &mut Node), With<MapChartCharterMarker>>,
    mut built: Local<bool>,
) {
    let Ok((block, mut node)) = q_block.single_mut() else {
        return;
    };
    let shown = runtime.active && runtime.chart && !progress.is_empty();
    let display = if shown { Display::Flex } else { Display::None };
    if node.display != display {
        node.display = display;
    }
    if !shown {
        if *built {
            commands.entity(block).despawn_related::<Children>();
            *built = false;
        }
        return;
    }
    if !*built || progress.is_changed() || record.is_changed() {
        *built = true;
        commands
            .entity(block)
            .despawn_related::<Children>()
            .with_children(|block| {
                for (index, (text, color)) in charter_block_lines(&progress, &record)
                    .into_iter()
                    .enumerate()
                {
                    let size = if index == 0 { 12.0 } else { 11.0 };
                    block.spawn(themed_label(&text, size, color));
                }
            });
    }
}

/// The discovery log's slot in the side panel: a heading over the rows,
/// hidden off the chart view. [`update_map_chart_log`] fills the rows.
pub(crate) fn spawn_map_chart_log(panel: &mut ChildSpawnerCommands) {
    panel.spawn((
        MapChartLogMarker,
        Name::new("MapChartLog"),
        Node {
            display: Display::None,
            flex_direction: FlexDirection::Column,
            row_gap: px(2),
            ..default()
        },
    ));
}

/// One discovery log row, as the panel draws it.
struct ChartLogRowView {
    /// The find it selects.
    mark: MapChartMarkType,
    icon: ChartIconType,
    /// The icon's and the label's colour.
    color: UiColor,
    /// A cell's contents in words, a station's code and last-seen name, or
    /// a wreck's code.
    label: String,
    cell: SectorCoord,
}

/// What one discovery log row draws.
fn chart_log_row(chart: &Chart, jobs: &Jobs, entry: &ChartLogEntry) -> ChartLogRowView {
    match &entry.entry {
        ChartLogEntryType::Cell(coord) => {
            let cell = chart.cell(*coord);
            ChartLogRowView {
                mark: MapChartMarkType::Cell(*coord),
                icon: cell.map_or(ChartIconType::Sector, chart_cell_icon),
                color: UiColor::Primary,
                label: cell.map_or_else(|| "CELL".to_string(), chart_cell_words),
                cell: *coord,
            }
        }
        ChartLogEntryType::Station(id) => {
            let station = chart.station(id);
            ChartLogRowView {
                mark: MapChartMarkType::Station(id.clone()),
                icon: ChartIconType::Station,
                color: station.map_or(chart_station_kind(false, false).1, |station| {
                    chart_station_kind(station.derelict, station.owned).1
                }),
                label: station.map_or_else(
                    || id.to_string(),
                    |station| station_row_label(chart, jobs, station),
                ),
                cell: id.origin,
            }
        }
        ChartLogEntryType::Wreck(id) => {
            let mark = MapChartMarkType::Wreck(id.clone());
            ChartLogRowView {
                label: chart_mark_code(chart, jobs, &mark).unwrap_or_default(),
                mark,
                icon: ChartIconType::Wreck,
                color: MAP_CHART_WRECK_COLOR,
                cell: id.origin,
            }
        }
    }
}

/// The edge a find row wears: the accent when it is the selected find, else
/// the faint secondary the chart labels wear.
const fn chart_log_row_edge(selected: bool) -> (UiColor, f32) {
    if selected {
        (UiColor::Accent, 1.0)
    } else {
        (UiColor::Secondary, 0.3)
    }
}

/// Show the discovery log while the chart view shows with nothing selected,
/// rebuild its rows newest first whenever the chart or the play minute
/// changes, hide it otherwise, and edge the selected find's row in the
/// accent.
pub(crate) fn update_map_chart_log(
    mut commands: Commands,
    runtime: Res<MapRuntime>,
    chart: Res<Chart>,
    jobs: Res<Jobs>,
    clock: Res<WorldClock>,
    icons: Res<InterfaceIcons>,
    mut q_log: Query<(Entity, &mut Node), With<MapChartLogMarker>>,
    mut q_row: Query<(&MapChartLogRow, &mut ThemedBorder)>,
    mut built: Local<Option<u64>>,
) {
    let Ok((log, mut node)) = q_log.single_mut() else {
        return;
    };
    let shown = runtime.active && runtime.chart && runtime.chart_selected.is_none();
    let display = if shown { Display::Flex } else { Display::None };
    if node.display != display {
        node.display = display;
    }
    if !shown {
        if built.is_some() {
            commands.entity(log).despawn_related::<Children>();
            *built = None;
        }
        return;
    }
    let minute = (clock.play_seconds.max(0.0) / 60.0).floor() as u64;
    if *built != Some(minute) || chart.is_changed() {
        *built = Some(minute);
        let mut entries = chart.log();
        entries.reverse();
        entries.truncate(MAP_CHART_LOG_ROWS);
        commands
            .entity(log)
            .despawn_related::<Children>()
            .with_children(|log| {
                log.spawn(themed_label("DISCOVERY LOG", 12.0, UiColor::Label));
                if entries.is_empty() {
                    log.spawn(themed_label("Nothing found yet.", 11.0, UiColor::Label));
                }
                for (index, entry) in entries.iter().enumerate() {
                    let view = chart_log_row(&chart, &jobs, entry);
                    let cell = view.cell.to_string();
                    let when = relative_time_text(clock.play_seconds, entry.at);
                    let mut row = spawn_icon_row(
                        log,
                        Some((icons.chart(view.icon), view.color)),
                        &view.label,
                        view.color,
                        &[(&cell, 12.0, UiColor::Body), (&when, 11.0, UiColor::Label)],
                    );
                    select_row(
                        &mut row,
                        format!("MapChartLog{index}"),
                        view.mark.clone(),
                        runtime.chart_selected.as_ref() == Some(&view.mark),
                    );
                }
            });
        return;
    }
    for (row, mut border) in &mut q_row {
        let (color, alpha) = chart_log_row_edge(Some(&row.0) == runtime.chart_selected.as_ref());
        if border.color != color || border.alpha != alpha {
            border.color = color;
            border.alpha = alpha;
        }
    }
}

/// Place each chart label over its anchor through the map camera, hide the
/// ones off the viewport, and mark the selected one with the accent edge.
pub(crate) fn project_map_chart_labels(
    runtime: Res<MapRuntime>,
    q_camera: Query<(&Camera, &GlobalTransform), With<MapCameraMarker>>,
    q_viewport: Query<&ComputedNode, With<MapViewportMarker>>,
    mut q_label: Query<(
        &MapChartLabel,
        &mut Node,
        &mut Visibility,
        &mut ThemedBorder,
    )>,
) {
    if !runtime.active || !runtime.chart {
        return;
    }
    let (Ok((camera, cam_gt)), Ok(computed)) = (q_camera.single(), q_viewport.single()) else {
        return;
    };
    if camera.physical_target_size() != Some(computed.size().round().as_uvec2()) {
        return;
    }
    let to_logical = computed.inverse_scale_factor();
    let size = computed.size() * to_logical;
    for (label, mut node, mut visibility, mut border) in &mut q_label {
        let projected = camera
            .world_to_viewport(cam_gt, label.anchor)
            .ok()
            .map(|p| p * to_logical)
            .filter(|p| p.x >= 0.0 && p.y >= 0.0 && p.x <= size.x && p.y <= size.y);
        let Some(p) = projected else {
            visibility.set_if_neq(Visibility::Hidden);
            continue;
        };
        let (left, top) = (Val::Px(p.x + 6.0), Val::Px(p.y - 9.0));
        if node.left != left || node.top != top {
            node.left = left;
            node.top = top;
        }
        visibility.set_if_neq(Visibility::Inherited);
        if label.mark.is_some() {
            let selected = label.mark == runtime.chart_selected;
            let (color, alpha) = if selected {
                (UiColor::Accent, 1.0)
            } else {
                (UiColor::Secondary, 0.3)
            };
            if border.color != color || border.alpha != alpha {
                border.color = color;
                border.alpha = alpha;
            }
        }
    }
}

/// What the contact panel's head says in the chart view, field by field:
/// the selected mark's code, its last-seen name, what it is, its cell, the
/// services seen on the last dock, and when the player found it and whether
/// it was docked at; for a visited cell, what its clusters hold and when the
/// player entered it; for a held job, its title, reward and where it hands
/// in. The SECTOR block lists the cell's finds, stock and jobs. With nothing
/// selected it heads the discovery log and counts the chart.
pub(crate) fn chart_panel_field(
    chart: &Chart,
    jobs: &Jobs,
    items: Option<&GameItems>,
    selected: Option<&MapChartMarkType>,
    field: MapPanelField,
) -> (String, UiColor) {
    let station = |id: &GeneratedBodyId| chart.station(id);
    let station_name = |id: &GeneratedBodyId| {
        station(id)
            .and_then(|station| station.summary.as_ref())
            .map_or_else(|| id.to_string(), |summary| summary.name.clone())
    };
    let found = |at: f64| format!("Found {}.", play_time_text(at));
    match (selected, field) {
        (None, MapPanelField::Code) => ("DISCOVERY LOG".to_string(), UiColor::Label),
        (None, MapPanelField::Range) => (
            format!("Cells visited {}", chart.visited().count()),
            UiColor::Primary,
        ),
        (None, MapPanelField::Bearing) => (
            format!(
                "Stations {}  Wrecks {}  Jobs {}",
                chart.stations().len(),
                chart.wrecks().len(),
                jobs.held().len()
            ),
            UiColor::Primary,
        ),
        (None, MapPanelField::Note) => (
            "Click a log line, a cell or a mark to read it.".to_string(),
            UiColor::Label,
        ),
        (None, _) => (String::new(), UiColor::Body),
        (Some(mark), MapPanelField::Code) => (
            chart_mark_code(chart, jobs, mark).unwrap_or_default(),
            UiColor::Primary,
        ),
        (Some(MapChartMarkType::Cell(coord)), MapPanelField::Name) => (
            match chart.cell(*coord) {
                Some(cell) if !cell.clusters.is_empty() => chart_cell_words(cell),
                _ => "Empty space".to_string(),
            },
            UiColor::Body,
        ),
        (Some(MapChartMarkType::Cell(_)), MapPanelField::Kind) => {
            ("VISITED CELL".to_string(), UiColor::Primary)
        }
        (Some(MapChartMarkType::Cell(coord)), MapPanelField::Range) => {
            (format!("Cell {coord}"), UiColor::Primary)
        }
        (Some(MapChartMarkType::Cell(coord)), MapPanelField::Bearing) => (
            chart.cell(*coord).map_or_else(String::new, |cell| {
                format!("Entered {}", play_time_text(cell.entered_at))
            }),
            UiColor::Primary,
        ),
        (Some(MapChartMarkType::Cell(_)), MapPanelField::Note) => (String::new(), UiColor::Label),
        (Some(MapChartMarkType::Job(id)), MapPanelField::Name) => (
            jobs.job(id).map_or_else(
                || "Job no longer held".to_string(),
                |job| job_title(job, items),
            ),
            UiColor::Body,
        ),
        (Some(MapChartMarkType::Job(_)), MapPanelField::Kind) => {
            ("HELD JOB".to_string(), MAP_CHART_JOB_COLOR)
        }
        (Some(MapChartMarkType::Job(id)), MapPanelField::Range) => (
            jobs.job(id)
                .map_or_else(String::new, |job| format!("Cell {}", job_cell(job))),
            UiColor::Primary,
        ),
        (Some(MapChartMarkType::Job(id)), MapPanelField::Bearing) => (
            jobs.job(id).map_or_else(String::new, |job| {
                format!("Reward {}", cr_text(job.reward_cr))
            }),
            UiColor::Primary,
        ),
        (Some(MapChartMarkType::Job(id)), MapPanelField::Note) => (
            jobs.job(id).map_or_else(String::new, |job| {
                let state = if jobs.is_done(id) {
                    "Done".to_string()
                } else {
                    format!("To do: {}", job.task())
                };
                format!(
                    "Posted by {}.\n{state}.\nHand in at {} while docked.",
                    station_name(job.posted_by()),
                    station_name(&job.target)
                )
            }),
            UiColor::Label,
        ),
        (Some(MapChartMarkType::Station(id)), MapPanelField::Name) => (
            station(id)
                .and_then(|station| station.summary.as_ref())
                .map_or_else(
                    || "Unknown station".to_string(),
                    |summary| summary.name.clone(),
                ),
            UiColor::Body,
        ),
        (Some(MapChartMarkType::Station(id)), MapPanelField::Kind) => {
            let (word, color) = station(id).map_or(chart_station_kind(false, false), |station| {
                chart_station_kind(station.derelict, station.owned)
            });
            (word.to_string(), color)
        }
        (Some(MapChartMarkType::Wreck(_)), MapPanelField::Name) => {
            ("Derelict hull".to_string(), UiColor::Body)
        }
        (Some(MapChartMarkType::Wreck(_)), MapPanelField::Kind) => {
            ("WRECK".to_string(), MAP_CHART_WRECK_COLOR)
        }
        (
            Some(MapChartMarkType::Station(id) | MapChartMarkType::Wreck(id)),
            MapPanelField::Range,
        ) => (format!("Cell {}", id.origin), UiColor::Primary),
        (Some(MapChartMarkType::Station(id)), MapPanelField::Bearing) => {
            match station(id).and_then(|station| station.summary.as_ref()) {
                Some(summary) if summary.services.is_empty() => {
                    ("Services: none".to_string(), UiColor::Primary)
                }
                Some(summary) => (
                    format!(
                        "Services: {}",
                        summary
                            .services
                            .iter()
                            .map(|service| format!("{service:?}").to_lowercase())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    UiColor::Primary,
                ),
                None => (String::new(), UiColor::Primary),
            }
        }
        (Some(MapChartMarkType::Station(id)), MapPanelField::Note) => {
            let Some(station) = station(id) else {
                return (String::new(), UiColor::Label);
            };
            let seen = if station.summary.is_some() {
                "Last seen docked."
            } else {
                "Not docked: no stock seen."
            };
            (
                format!("{}\n{seen}", found(station.found_at)),
                UiColor::Label,
            )
        }
        (Some(MapChartMarkType::Wreck(id)), MapPanelField::Note) => (
            chart.wreck(id).map_or_else(String::new, |wreck| {
                format!("{}\nFound on sensors.", found(wreck.found_at))
            }),
            UiColor::Label,
        ),
        (Some(_), _) => (String::new(), UiColor::Body),
    }
}

/// The legend the chart view shows: one icon and word per content, the
/// station, wreck and job marks, and you.
pub(crate) fn spawn_chart_legend(legend: &mut ChildSpawnerCommands, icons: &InterfaceIcons) {
    for icon in ChartIconType::LEGEND {
        legend.spawn(legend_entry()).with_children(|entry| {
            entry.spawn(icon_node(icons.chart(icon), icon.color(), 16.0));
            entry.spawn(themed_label(icon.label(), 12.0, UiColor::Body));
        });
    }
}

/// The footer button that switches the chart view.
pub(crate) fn map_chart_button() -> impl Bundle {
    (
        button(ButtonSpec::new("Chart").fit()),
        Name::new("MapChart"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The CHARTER block reads a heading, the goal's title, a `have / need`
    /// row per line coloured met or open, and the met stamp or `Standing`.
    #[test]
    fn charter_block_lines_reads_heading_title_rows_and_the_met_stamp() {
        let progress = GoalProgress {
            title: "A kill list".to_string(),
            lines: vec![GoalLine::new("Hulls", 6, 6), GoalLine::new("Rings", 1, 2)],
        };
        let met = charter_block_lines(&progress, &GoalRecord { met_at: Some(90.0) });
        assert_eq!(
            met,
            vec![
                ("CHARTER".to_string(), UiColor::Label),
                ("A kill list".to_string(), UiColor::Label),
                ("Hulls  6 / 6".to_string(), UiColor::Accent),
                ("Rings  1 / 2".to_string(), UiColor::Secondary),
                ("Met at 0:01:30".to_string(), UiColor::Label),
            ]
        );
        let standing = charter_block_lines(&progress, &GoalRecord::default());
        assert_eq!(
            standing.last(),
            Some(&("Standing".to_string(), UiColor::Label))
        );
    }

    /// A find reads how long ago against the clock in whole minutes, then
    /// whole hours, and `just now` under a minute or ahead of the clock; the
    /// route to a cell reads one leg per axis the player must cross.
    #[test]
    fn the_sector_panel_reads_relative_times_and_routes() {
        assert_eq!(relative_time_text(100.0, 70.0), "just now");
        assert_eq!(relative_time_text(100.0, 130.0), "just now");
        assert_eq!(relative_time_text(300.0, 59.0), "4 min ago");
        assert_eq!(relative_time_text(7_300.0, 0.0), "2 h ago");
        assert_eq!(relative_time_text(f64::NAN, 0.0), "just now");

        let from = SectorCoord::new(1, 0, -2);
        assert_eq!(route_hint(from, from), "This cell");
        assert_eq!(
            route_hint(from, SectorCoord::new(3, 0, -3)),
            "2 cells +X, 1 cell -Z"
        );
        assert_eq!(route_hint(from, SectorCoord::new(1, -1, -2)), "1 cell -Y");
    }
}
