//! The map's chart view: the player's [`Chart`] drawn at cell scale.
//!
//! The view swaps the local scene for a grid of cells around the player's
//! cell, the visited cells filled in the colour of their first cluster type,
//! and the found stations and wrecks as marks in the cell that generated
//! them. Marks and cell tags are projected UI labels, like the local blips.
//! A label shows only a code minted from the chart's found order, never a
//! name: the open world names a ship by whose it is.
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
use nova_ui::{
    prelude::{button, ButtonSpec},
    theme::{ActiveUiTheme, UiColor},
    widget::{ThemedBorder, ThemedFill},
};

use super::{app::*, contacts::*, *};
use crate::{
    pane::{legend_entry, play_menu_select, themed_label, InterfacePaneType},
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

/// One station or wreck of the chart the player can select.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MapChartMarkType {
    /// A found station.
    Station(GeneratedBodyId),
    /// A found derelict hull.
    Wreck(GeneratedBodyId),
}

/// The chart view's scene, under the map scene root.
#[derive(Component)]
pub(crate) struct MapChartRoot;

/// One label of the chart view over the viewport: a mark's code, which
/// selects the mark, or a visited cell's cluster tag, which takes no clicks.
#[derive(Component)]
pub(crate) struct MapChartLabel {
    /// Where the label points, in chart space.
    pub(crate) anchor: Vec3,
    /// The mark the label selects.
    pub(crate) mark: Option<MapChartMarkType>,
}

/// The chart code of a mark: its 1-based place in the chart's found order.
pub(crate) fn chart_mark_code(chart: &Chart, mark: &MapChartMarkType) -> Option<String> {
    match mark {
        MapChartMarkType::Station(id) => chart
            .stations()
            .iter()
            .position(|station| station.id == *id)
            .map(|index| format!("STN-{}", index + 1)),
        MapChartMarkType::Wreck(id) => chart
            .wrecks()
            .iter()
            .position(|wreck| wreck == id)
            .map(|index| format!("WRK-{}", index + 1)),
    }
}

/// The colour a visited cell is filled with: its first cluster type's, or a
/// faint neutral for a cell with no cluster body.
pub(crate) fn chart_cell_color(clusters: &[ClusterType]) -> (UiColor, f32) {
    match clusters.first() {
        Some(ClusterType::AsteroidRich) => (UiColor::Accent, 0.35),
        Some(ClusterType::RockOnly) => (UiColor::Secondary, 0.35),
        Some(ClusterType::PlanetHeavy) => (UiColor::Info, 0.35),
        Some(ClusterType::DerelictField) => (UiColor::Body, 0.3),
        None => (UiColor::Label, 0.15),
    }
}

/// The two-letter tag a cell label and the legend name a cluster type by.
pub(crate) const fn chart_cluster_tag(cluster: ClusterType) -> &'static str {
    match cluster {
        ClusterType::AsteroidRich => "AR",
        ClusterType::RockOnly => "RO",
        ClusterType::PlanetHeavy => "PH",
        ClusterType::DerelictField => "DF",
    }
}

/// The colour of a station mark: living or dead.
pub(crate) const fn chart_station_color(derelict: bool) -> UiColor {
    if derelict {
        UiColor::Danger
    } else {
        UiColor::Nominal
    }
}

/// The colour of a wreck mark.
pub(crate) const MAP_CHART_WRECK_COLOR: UiColor = UiColor::AccentHigh;

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
/// drops the chart selection and reframes the view it enters.
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
        runtime.goto_note = Some(("NO CHART: NO OPEN WORLD".to_string(), 2.5));
        return;
    }
    runtime.chart = !runtime.chart;
    runtime.chart_selected = None;
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

/// Build the chart scene and its labels while the chart view shows, rebuild
/// them when the chart, the player's cell or the theme changes, and tear
/// them down when the view goes. Hides the local rings and hub meanwhile.
#[expect(
    clippy::too_many_arguments,
    reason = "the chart, the scene assets and the viewport its labels sit on"
)]
pub(crate) fn manage_map_chart(
    mut commands: Commands,
    mut runtime: ResMut<MapRuntime>,
    chart: Res<Chart>,
    edge: Option<Res<ChartSectorEdge>>,
    contacts: MapContacts,
    theme: Res<ActiveUiTheme>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    q_viewport: Query<Entity, With<MapViewportMarker>>,
    mut q_anchor: Query<&mut Visibility, With<MapFocusAnchor>>,
    mut built_for: Local<Option<SectorCoord>>,
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
        *built_for = None;
        return;
    };
    let player_cell = SectorCoord::containing(Meters3::from_engine(contacts.focus()), edge.0);
    if runtime.chart_root.is_some()
        && *built_for == Some(player_cell)
        && !chart.is_changed()
        && !theme.is_changed()
    {
        return;
    }
    drop_map_chart(&mut commands, &mut runtime);
    *built_for = Some(player_cell);
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

    // Visited cells, filled in their first cluster type's colour.
    let fill = meshes.add(Cuboid::new(MAP_CHART_CELL * 0.9, 0.4, MAP_CHART_CELL * 0.9));
    let mut labels = Vec::new();
    for (cell, clusters) in chart.visited() {
        let (color, alpha) = chart_cell_color(clusters);
        let at = chart_cell_position(cell, player_cell);
        spawn_mesh(
            fill.clone(),
            materials.add(unlit(theme.color_alpha(color, alpha))),
            Transform::from_translation(at - Vec3::Y * 0.3),
        );
        if !clusters.is_empty() {
            let tag = clusters
                .iter()
                .map(|cluster| chart_cluster_tag(*cluster))
                .collect::<Vec<_>>()
                .join("+");
            let corner = Vec3::new(-0.38, 0.0, -0.38) * MAP_CHART_CELL;
            labels.push((at + corner, None, tag, color));
        }
    }

    // Stations and wrecks, side by side in the cell that generated them.
    let marks: Vec<(MapChartMarkType, SectorCoord, UiColor)> = chart
        .stations()
        .iter()
        .map(|station| {
            (
                MapChartMarkType::Station(station.id.clone()),
                station.id.origin,
                chart_station_color(station.derelict),
            )
        })
        .chain(chart.wrecks().iter().map(|wreck| {
            (
                MapChartMarkType::Wreck(wreck.clone()),
                wreck.origin,
                MAP_CHART_WRECK_COLOR,
            )
        }))
        .collect();
    let station_mesh = meshes.add(Cuboid::new(9.0, 9.0, 9.0));
    let wreck_mesh = meshes.add(Tetrahedron::default());
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
        let material = materials.add(unlit(theme.color(*color)));
        match mark {
            MapChartMarkType::Station(_) => {
                spawn_mesh(
                    station_mesh.clone(),
                    material,
                    Transform::from_translation(at),
                );
            }
            MapChartMarkType::Wreck(_) => {
                spawn_mesh(
                    wreck_mesh.clone(),
                    material,
                    Transform::from_translation(at).with_scale(Vec3::splat(10.0)),
                );
            }
        }
        if let Some(code) = chart_mark_code(&chart, mark) {
            labels.push((at, Some(mark.clone()), code, *color));
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
        "SELF".to_string(),
        MapContactKind::OwnShip.color(),
    ));

    for (anchor, mark, text, color) in labels {
        let label = spawn_chart_label(&mut commands, viewport, anchor, mark, &text, color);
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
/// mark's label is a button that selects the mark.
fn spawn_chart_label(
    commands: &mut Commands,
    viewport: Entity,
    anchor: Vec3,
    mark: Option<MapChartMarkType>,
    text: &str,
    color: UiColor,
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
        children![(
            themed_label(text, if clickable { 12.0 } else { 11.0 }, color),
            Pickable::IGNORE
        )],
    ));
    if clickable {
        label
            .insert((
                bevy::ui_widgets::Button,
                Name::new(format!("MapChartMark{text}")),
            ))
            .observe(on_map_chart_label_click);
    } else {
        label.insert(Pickable::IGNORE);
    }
    label.id()
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
    if runtime.chart_selected.as_ref() != Some(mark) {
        runtime.chart_selected = Some(mark.clone());
        play_menu_select(&mut commands, bank.as_deref());
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

/// What the contact panel says in the chart view, field by field: the
/// selected mark's code, its last-seen name, what it is, its cell, the
/// services and stock seen on the last dock, or a station seen but never
/// docked as having no stock. With nothing selected it counts the chart.
pub(crate) fn chart_panel_field(
    chart: &Chart,
    selected: Option<&MapChartMarkType>,
    field: MapPanelField,
) -> (String, UiColor) {
    let stacks = |stacks: &[(ItemDesignId, u32)]| {
        if stacks.is_empty() {
            "none".to_string()
        } else {
            stacks
                .iter()
                .map(|(item, count)| format!("{item} x{count}"))
                .collect::<Vec<_>>()
                .join(", ")
        }
    };
    let station = |id: &GeneratedBodyId| chart.station(id);
    match (selected, field) {
        (None, MapPanelField::Code) => ("No mark".to_string(), UiColor::Label),
        (None, MapPanelField::Range) => (
            format!("Cells visited {}", chart.visited().count()),
            UiColor::Primary,
        ),
        (None, MapPanelField::Bearing) => (
            format!(
                "Stations {}  Wrecks {}",
                chart.stations().len(),
                chart.wrecks().len()
            ),
            UiColor::Primary,
        ),
        (None, MapPanelField::Note) => (
            "Click a station or wreck to read it.".to_string(),
            UiColor::Label,
        ),
        (None, _) => (String::new(), UiColor::Body),
        (Some(mark), MapPanelField::Code) => (
            chart_mark_code(chart, mark).unwrap_or_default(),
            UiColor::Primary,
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
            let derelict = station(id).is_some_and(|station| station.derelict);
            let word = if derelict {
                "DEAD STATION"
            } else {
                "LIVING STATION"
            };
            (word.to_string(), chart_station_color(derelict))
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
            match station(id).and_then(|station| station.summary.as_ref()) {
                Some(summary) => (
                    format!(
                        "Last seen docked.\nStock: {}\nStored: {}",
                        stacks(&summary.stock),
                        stacks(&summary.stored)
                    ),
                    UiColor::Label,
                ),
                None => ("Not docked: no stock seen.".to_string(), UiColor::Label),
            }
        }
        (Some(MapChartMarkType::Wreck(_)), MapPanelField::Note) => {
            ("Found on sensors.".to_string(), UiColor::Label)
        }
        (Some(_), _) => (String::new(), UiColor::Body),
    }
}

/// The legend the chart view shows: each cluster type's fill and tag, the
/// station and wreck marks, and the own ship.
pub(crate) fn spawn_chart_legend(legend: &mut ChildSpawnerCommands) {
    let swatch = |entry: &mut ChildSpawnerCommands, color: UiColor, alpha: f32| {
        entry.spawn((
            Node {
                width: px(12),
                height: px(12),
                ..default()
            },
            BackgroundColor(Color::NONE),
            ThemedFill::alpha(color, alpha),
        ));
    };
    for cluster in ClusterType::ALL {
        legend.spawn(legend_entry()).with_children(|entry| {
            let (color, _) = chart_cell_color(&[cluster]);
            swatch(entry, color, 1.0);
            entry.spawn(themed_label(
                &format!("{} {}", chart_cluster_tag(cluster), cluster.label()),
                12.0,
                UiColor::Body,
            ));
        });
    }
    for (color, word) in [
        (chart_station_color(false), "Living station"),
        (chart_station_color(true), "Dead station"),
        (MAP_CHART_WRECK_COLOR, "Wreck"),
        (MapContactKind::OwnShip.color(), "Own ship"),
    ] {
        legend.spawn(legend_entry()).with_children(|entry| {
            swatch(entry, color, 1.0);
            entry.spawn(themed_label(word, 12.0, UiColor::Body));
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
