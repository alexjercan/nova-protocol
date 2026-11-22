//! The map pane's components, contact panel layout and runtime state.
//!
//! Touch this module when changing what the map pane remembers between frames
//! or how its contact panel is laid out.

use bevy::{input::mouse::MouseWheel, picking::hover::Hovered, prelude::*};
use nova_events::prelude::{GeneratedBodyId, SectorCoord};
use nova_ui::{
    prelude::{
        button, control_row, inspector, metrics, preview_frame, spawn_kv_slot, text, verb_row,
        ButtonSpec, RowAlign, TextRole, VerbGate,
    },
    screen::drive_wheel_scroll,
};

use super::{
    on_map_cancel_button, on_map_goto_button, spawn_map_chart_log, spawn_map_chart_objectives,
    spawn_map_chart_sector, spawn_map_stance_block, ChartStationLive, MapContactKind,
};
use crate::{
    icons::{icon_node, BodyIconType, InterfaceIcons},
    pane::ActionCue,
};

/// The map pane's viewport node (holds the RTT image + blip children).
#[derive(Component)]
pub(crate) struct MapViewportMarker;

/// Which live text line of the contact panel a node is, so
/// [`update_map_panel`](super::update_map_panel) refreshes them all in place.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum MapPanelField {
    /// The selected contact's code; blank with nothing selected.
    Code,
    /// Its display name.
    Name,
    /// Its kind word, in its stance colour.
    Kind,
    /// Its range from the player ship; a chart mark's cell. The value of a
    /// [`MapPanelSlot`] row.
    Range,
    /// Its bearing and mark from the player ship; when a chart cell was
    /// entered, or a held job's reward. The value of a [`MapPanelSlot`] row.
    Bearing,
    /// A marked contact's objective and the held jobs that send the player
    /// to it.
    Note,
    /// The player ship's live GOTO destination, read from its autopilot and
    /// its waypoint, not from the selection. Shown in both views.
    Destination,
}

/// A kv row of the contact panel, [`MapPanelField::Range`] or
/// [`MapPanelField::Bearing`]: its label word follows what the row reads
/// (`Range` on the local map, `Cell` on a chart mark), and the row hides
/// whole while it reads nothing.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct MapPanelSlot(pub(crate) MapPanelField);

/// The label span of a [`MapPanelSlot`] row.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct MapPanelLabel(pub(crate) MapPanelField);

/// The selected contact's body icon in the panel head, hidden with nothing
/// selected.
#[derive(Component)]
pub(crate) struct MapPanelIcon;

/// The icon frame with the Code/Name/Kind fields beside it, then the
/// Range/Bearing and Note fields, as one node:
/// [`update_map_panel`](super::update_map_panel) hides it with nothing
/// selected, on local Map and in the chart overview alike.
#[derive(Component)]
pub(crate) struct MapPanelHeaderMarker;

/// The contact panel beside the map view: the scrolling
/// [`MapPanelBodyMarker`] over the destination line and the GOTO and Cancel
/// row, which stay in the panel however long the body runs. A wheel notch
/// over any of it scrolls the body.
#[derive(Component)]
pub(crate) struct MapPanelMarker;

/// The contact panel's scrolling body: a selected station's stock list
/// pushes the STANCES block past a 720p panel, and past a 1080p one in the
/// chart.
#[derive(Component)]
pub(crate) struct MapPanelBodyMarker;

/// The contact panel's GOTO button. Its [`VerbGate`] hides it on the docked
/// partner and, in the chart, unless a station or a cell is selected, and
/// disables it with no contact or the own ship selected;
/// [`update_map_panel`](super::update_map_panel) writes it.
#[derive(Component)]
pub(crate) struct MapGotoButton;

/// The contact panel's Cancel button. It is enabled while the player ship
/// flies a GOTO or a waypoint, even if its target left the local map; in the
/// chart it is hidden unless a station or a cell is selected.
/// [`update_map_panel`](super::update_map_panel) writes its [`VerbGate`].
#[derive(Component)]
pub(crate) struct MapCancelButton;

/// One pooled stroke of the predicted GOTO path, drawn under the blips. The
/// strokes are interchangeable; unused ones stay hidden. It takes no clicks.
#[derive(Component)]
pub(crate) struct MapRouteLine;

/// The `GOTO` text tag over the live GOTO target, or beside the waypoint's
/// label in the chart, so the destination does not rely on colour alone. It
/// takes no clicks.
#[derive(Component)]
pub(crate) struct MapGotoMarker;

/// Wheel-scroll the contact panel while the pointer is over it, through the
/// shared driver; `map_input` leaves the view's zoom alone then. A wheel
/// notch away from the panel is the view's.
pub(crate) fn scroll_map_panel(
    mut wheel: MessageReader<MouseWheel>,
    q_panel: Query<&Hovered, With<MapPanelMarker>>,
    q_body: Query<
        (&mut ScrollPosition, Option<&ComputedNode>, Option<&Hovered>),
        With<MapPanelBodyMarker>,
    >,
) {
    if !q_panel.iter().any(Hovered::get) {
        wheel.clear();
        return;
    }
    drive_wheel_scroll(wheel, q_body);
}

/// Build the contact panel beside the map view: the
/// [`MapPanelHeaderMarker`] node (the selection's icon in a frame the chart
/// view hides, beside its code, name and kind; its range and bearing; its
/// note), hidden with nothing selected; the SECTOR block, shown in the chart
/// view with a find selected; the OBJECTIVES block and the discovery log
/// under it, shown in the chart view with nothing selected; the STANCES
/// block, shown in the chart view with nothing selected or a living, unowned
/// Station mark selected. All of these scroll in the
/// [`MapPanelBodyMarker`]; under it, outside the scroll, the GOTO
/// destination over the GOTO and Cancel pair.
/// [`update_map_panel`](super::update_map_panel) fills it.
pub(crate) fn spawn_map_panel(parent: &mut ChildSpawnerCommands, icons: &InterfaceIcons) {
    parent
        .spawn((
            MapPanelMarker,
            Name::new("MapPanel"),
            inspector(Overflow::clip()),
            Hovered::default(),
        ))
        .with_children(|frame| {
            frame
                .spawn((
                    MapPanelBodyMarker,
                    Name::new("MapPanelBody"),
                    Node {
                        flex_grow: 1.0,
                        min_height: px(0),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(metrics::GAP_L),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                    ScrollPosition::default(),
                ))
                .with_children(|panel| spawn_map_panel_body(panel, icons));
            frame.spawn((MapPanelField::Destination, text("", TextRole::Label)));
            // Disabled at spawn: the map opens with nothing selected.
            frame
                .spawn(Node {
                    width: percent(100),
                    flex_shrink: 0.0,
                    ..verb_row(RowAlign::End)
                })
                .with_children(|verbs| {
                    verbs
                        .spawn((
                            MapGotoButton,
                            button(ButtonSpec::control("GOTO").fill()),
                            VerbGate::Disabled,
                            Name::new("MapGoto"),
                        ))
                        .observe(on_map_goto_button);
                    verbs
                        .spawn((
                            MapCancelButton,
                            button(ButtonSpec::control("Cancel").fill()),
                            VerbGate::Disabled,
                            Name::new("MapCancel"),
                        ))
                        .observe(on_map_cancel_button);
                });
        });
}

/// The scrolling part of the contact panel: the header, the chart blocks and
/// the STANCES block.
fn spawn_map_panel_body(panel: &mut ChildSpawnerCommands, icons: &InterfaceIcons) {
    panel
        .spawn((
            MapPanelHeaderMarker,
            Node {
                display: Display::None,
                flex_direction: FlexDirection::Column,
                row_gap: px(metrics::GAP),
                ..default()
            },
        ))
        .with_children(|header| {
            header
                .spawn(control_row(JustifyContent::FlexStart))
                .with_children(|head| {
                    head.spawn(preview_frame()).with_children(|frame| {
                        frame.spawn((
                            MapPanelIcon,
                            icon_node(
                                icons.body(BodyIconType::Ship),
                                MapContactKind::OwnShip.color(),
                                metrics::PREVIEW * 0.75,
                            ),
                            Visibility::Hidden,
                        ));
                    });
                    head.spawn(Node {
                        flex_grow: 1.0,
                        min_width: px(0),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(metrics::GAP_S),
                        ..default()
                    })
                    .with_children(|identity| {
                        for (field, role) in [
                            (MapPanelField::Code, TextRole::Title),
                            (MapPanelField::Name, TextRole::Body),
                            (MapPanelField::Kind, TextRole::Body),
                        ] {
                            identity.spawn((field, text("", role)));
                        }
                    });
                });
            for field in [MapPanelField::Range, MapPanelField::Bearing] {
                spawn_kv_slot(
                    header,
                    MapPanelSlot(field),
                    MapPanelLabel(field),
                    (field, text("", TextRole::Value)),
                );
            }
            header.spawn((MapPanelField::Note, text("", TextRole::Label)));
        });
    spawn_map_chart_sector(panel);
    spawn_map_chart_objectives(panel);
    spawn_map_chart_log(panel);
    spawn_map_stance_block(panel);
}

/// The slot [`refresh_map_legend`](super::refresh_map_legend) fills with the
/// bodies and stances the map plots.
#[derive(Component)]
pub(crate) struct MapLegendMarker;

/// The map's 3D camera (renders the schematic scene to the offscreen image).
#[derive(Component)]
pub(crate) struct MapCameraMarker;

/// The map camera's orbit state, driven directly (we own the spherical math
/// rather than routing through the shared `SphereOrbit` plugin, whose smoothed
/// input path did not rotate this render-to-texture camera in practice).
/// `theta` is the azimuth, `phi` the elevation above the plane, `radius` the
/// distance from `center`, the focus point WASD pans and selection recenters.
#[derive(Component)]
pub(crate) struct MapOrbit {
    pub(crate) theta: f32,
    pub(crate) phi: f32,
    pub(crate) radius: f32,
    pub(crate) center: Vec3,
}

/// The parent of every spawned map-scene entity (camera + proxy meshes), so the
/// whole scene tears down with one `despawn`.
#[derive(Component)]
pub(crate) struct MapSceneRoot;

/// Holds the distance rings + hub; its transform tracks the orbit center so the
/// scale reference surrounds the focused object (`map_focus_follow`).
#[derive(Component)]
pub(crate) struct MapFocusAnchor;

/// The hub sphere marking the orbit centre; scaled to the focused contact.
#[derive(Component)]
pub(crate) struct MapFocusHub;

/// A projected contact blip (a clickable UI marker over the viewport image).
#[derive(Component)]
pub(crate) struct MapBlip {
    pub(crate) contact: Entity,
    /// What the blip's icon draws.
    pub(crate) body: BodyIconType,
    /// The stance its icon is tinted for.
    pub(crate) kind: MapContactKind,
    /// The code its label shows: a station's turns from `STN` to `STN-n`
    /// when the chart finds it.
    pub(crate) code: String,
}

/// A blip's code label. An asteroid's label shows only while it is the
/// selection: an open-world belt plots about two hundred rocks, and two
/// hundred labels bury each other and the ship labels in one pile of text.
#[derive(Component)]
pub(crate) struct MapBlipLabel;

/// Live state of the map pane.
#[derive(Resource, Default)]
pub(crate) struct MapRuntime {
    pub(crate) active: bool,
    pub(crate) image: Option<Handle<Image>>,
    pub(crate) camera: Option<Entity>,
    pub(crate) scene_root: Option<Entity>,
    pub(crate) blips: bevy::platform::collections::HashMap<Entity, Entity>,
    pub(crate) selected: Option<Entity>,
    /// The selection the focus last recentered on, so selecting a NEW contact
    /// snaps the map onto it once (without fighting WASD panning after).
    pub(crate) focused_on: Option<Entity>,
    /// The GOTO button was activated. [`map_input`](super::map_input) takes it
    /// on its next run and validates it with the `map_goto` key.
    pub(crate) goto_requested: bool,
    /// The CANCEL button was activated. [`map_input`](super::map_input) takes
    /// it on its next run.
    pub(crate) cancel_requested: bool,
    /// The chart view shows instead of the local scene.
    pub(crate) chart: bool,
    /// The Y coordinate of the chart's focused grid, in sector cells.
    pub(crate) chart_layer: i32,
    /// The Chart button was activated.
    /// [`toggle_map_chart`](super::toggle_map_chart) takes it on its next run.
    pub(crate) chart_requested: bool,
    /// The cell, station, wreck or held job selected in the chart view.
    pub(crate) chart_selected: Option<super::MapChartMarkType>,
    /// A mark to open the chart view on, selected, the next time the map
    /// owns the screen: the Jobs pane's Show on Map. Taken with
    /// [`chart_requested`](Self::chart_requested) by
    /// [`toggle_map_chart`](super::toggle_map_chart).
    pub(crate) chart_show: Option<super::MapChartMarkType>,
    /// The chart view's scene root, while it stands.
    pub(crate) chart_root: Option<Entity>,
    /// The chart view's labels over the viewport.
    pub(crate) chart_labels: Vec<Entity>,
    /// The player's cell the chart view was built around, while it stands.
    pub(crate) chart_player_cell: Option<SectorCoord>,
    /// The live facts of every found station standing in the streamed
    /// window, in id order, as the chart view last drew them.
    pub(crate) chart_live: Vec<(GeneratedBodyId, ChartStationLive)>,
    /// The last action result's cue: the STANCES block's Pay fine, settled
    /// or refused, and a GOTO key press the map refuses.
    pub(crate) cue: ActionCue,
}
