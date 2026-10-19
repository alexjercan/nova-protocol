//! The map pane's components, contact panel layout and runtime state.
//!
//! Touch this module when changing what the map pane remembers between frames
//! or how its contact panel is laid out.

use bevy::{
    input::mouse::MouseWheel, picking::hover::Hovered, prelude::*, ui::InteractionDisabled,
};
use nova_events::prelude::{GeneratedBodyId, SectorCoord};
use nova_ui::{
    prelude::{button, ButtonSpec},
    screen::drive_wheel_scroll,
    theme::UiColor,
    widget::ThemedFill,
};

use super::{
    on_map_cancel_button, on_map_goto_button, spawn_map_chart_log, spawn_map_chart_objectives,
    spawn_map_chart_sector, spawn_map_stance_block, ChartStationLive, MapContactKind,
};
use crate::{
    icons::{icon_node, BodyIconType, InterfaceIcons},
    pane::{panel_preview_frame, side_panel, themed_label, PANEL_PREVIEW_PX},
};

/// The map pane's viewport node (holds the RTT image + blip children).
#[derive(Component)]
pub(crate) struct MapViewportMarker;

/// Which live text line of the contact panel a node is, so
/// [`update_map_panel`](super::update_map_panel) refreshes them all in place.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum MapPanelField {
    /// The selected contact's code, or `No contact`.
    Code,
    /// Its display name.
    Name,
    /// Its kind word, in its stance colour.
    Kind,
    /// Its range from the player ship.
    Range,
    /// Its bearing and mark from the player ship.
    Bearing,
    /// A GOTO result while it shows, else what the contact is.
    Note,
    /// The player ship's live GOTO destination, read from its autopilot, not
    /// from the selection.
    Destination,
}

/// The selected contact's body icon in the panel head, hidden with nothing
/// selected.
#[derive(Component)]
pub(crate) struct MapPanelIcon;

/// The contact panel beside the map view. It scrolls: a selected station's
/// stock list pushes the STANCES block past a 720p panel.
#[derive(Component)]
pub(crate) struct MapPanelMarker;

/// The contact panel's GOTO button. It is disabled with no contact or the own
/// ship selected; [`update_map_panel`](super::update_map_panel) toggles it.
#[derive(Component)]
pub(crate) struct MapGotoButton;

/// The contact panel's CANCEL button. It is enabled while the player ship
/// flies a GOTO, in both views and whether or not its target is still on the
/// map; [`update_map_panel`](super::update_map_panel) toggles it.
#[derive(Component)]
pub(crate) struct MapCancelButton;

/// One pooled stroke of the predicted GOTO path, drawn under the blips. The
/// strokes are interchangeable; unused ones stay hidden. It takes no clicks.
#[derive(Component)]
pub(crate) struct MapRouteLine;

/// The `GOTO` text tag over the live GOTO target, so the destination does not
/// rely on colour alone. It takes no clicks.
#[derive(Component)]
pub(crate) struct MapGotoMarker;

/// Wheel-scroll the contact panel while the pointer is over it, through the
/// shared driver; `map_input` leaves the view's zoom alone then. A wheel
/// notch away from the panel is the view's.
pub(crate) fn scroll_map_panel(
    mut wheel: MessageReader<MouseWheel>,
    q_panel: Query<
        (&mut ScrollPosition, Option<&ComputedNode>, Option<&Hovered>),
        With<MapPanelMarker>,
    >,
) {
    if !q_panel
        .iter()
        .any(|(_, _, hovered)| hovered.is_some_and(Hovered::get))
    {
        wheel.clear();
        return;
    }
    drive_wheel_scroll(wheel, q_panel);
}

/// Build the contact panel beside the map view: the selection's icon, in a
/// frame the chart view hides, over its code, name and kind; a rule; its range
/// and bearing; a rule; its note; a rule; the SECTOR block, shown in the chart
/// view with a find selected; the OBJECTIVES block and the discovery log under
/// it, shown in the chart view with nothing selected; the STANCES block, shown
/// in the chart view with nothing selected or a living, unowned Station mark
/// selected; and the live GOTO destination over the GOTO and CANCEL buttons.
/// [`update_map_panel`](super::update_map_panel) fills it.
pub(crate) fn spawn_map_panel(parent: &mut ChildSpawnerCommands, icons: &InterfaceIcons) {
    parent
        .spawn((
            MapPanelMarker,
            Name::new("MapPanel"),
            side_panel(Overflow::scroll_y()),
            ScrollPosition::default(),
            Hovered::default(),
        ))
        .with_children(|panel| {
            panel.spawn(panel_preview_frame()).with_children(|frame| {
                frame.spawn((
                    MapPanelIcon,
                    icon_node(
                        icons.body(BodyIconType::Ship),
                        MapContactKind::OwnShip.color(),
                        PANEL_PREVIEW_PX * 0.75,
                    ),
                    Visibility::Hidden,
                ));
            });
            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4),
                    ..default()
                })
                .with_children(|identity| {
                    for (field, size, color) in [
                        (MapPanelField::Code, 18.0, UiColor::Primary),
                        (MapPanelField::Name, 13.0, UiColor::Body),
                        (MapPanelField::Kind, 12.0, UiColor::Body),
                    ] {
                        identity.spawn((field, themed_label("", size, color)));
                    }
                });
            panel.spawn(panel_rule());
            for field in [MapPanelField::Range, MapPanelField::Bearing] {
                panel.spawn((field, themed_label("", 13.0, UiColor::Primary)));
            }
            panel.spawn(panel_rule());
            panel.spawn((MapPanelField::Note, themed_label("", 12.0, UiColor::Label)));
            panel.spawn(panel_rule());
            spawn_map_chart_sector(panel);
            spawn_map_chart_objectives(panel);
            spawn_map_chart_log(panel);
            spawn_map_stance_block(panel);
            panel.spawn((
                MapPanelField::Destination,
                themed_label("", 12.0, UiColor::Label),
            ));
            // Disabled at spawn: the map opens with nothing selected.
            panel
                .spawn((
                    MapGotoButton,
                    button(ButtonSpec::new("GOTO").fit()),
                    InteractionDisabled,
                    Name::new("MapGoto"),
                ))
                .observe(on_map_goto_button);
            panel
                .spawn((
                    MapCancelButton,
                    button(ButtonSpec::new("CANCEL").fit()),
                    InteractionDisabled,
                    Name::new("MapCancel"),
                ))
                .observe(on_map_cancel_button);
        });
}

/// A one-pixel rule between groups of the contact panel.
fn panel_rule() -> impl Bundle {
    (
        Node {
            height: px(1),
            flex_shrink: 0.0,
            ..default()
        },
        BackgroundColor(Color::NONE),
        ThemedFill::alpha(UiColor::Secondary, 0.35),
    )
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
    /// A transient GOTO result shown on the panel note for a short time.
    pub(crate) goto_note: Option<(String, f32)>,
    /// The GOTO button was activated. [`map_input`](super::map_input) takes it
    /// on its next run and validates it with the `map_goto` key.
    pub(crate) goto_requested: bool,
    /// The CANCEL button was activated. [`map_input`](super::map_input) takes
    /// it on its next run.
    pub(crate) cancel_requested: bool,
    /// The chart view shows instead of the local scene.
    pub(crate) chart: bool,
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
}
