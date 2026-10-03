//! The map pane's components, contact panel layout and runtime state.
//!
//! Touch this module when changing what the map pane remembers between frames
//! or how its contact panel is laid out.

use bevy::{prelude::*, ui::InteractionDisabled};
use nova_ui::{
    prelude::{button, ButtonSpec},
    theme::UiColor,
    widget::ThemedFill,
};

use super::{on_map_goto_button, MapContactKind};
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

/// The contact panel's GOTO button. It is disabled with no contact or the own
/// ship selected; [`update_map_panel`](super::update_map_panel) toggles it.
#[derive(Component)]
pub(crate) struct MapGotoButton;

/// The route line from the player ship to its live GOTO target, drawn under
/// the blips. It takes no clicks.
#[derive(Component)]
pub(crate) struct MapRouteLine;

/// The `GOTO` text tag over the live GOTO target, so the destination does not
/// rely on colour alone. It takes no clicks.
#[derive(Component)]
pub(crate) struct MapGotoMarker;

/// Build the contact panel beside the map view: the selection's icon over its
/// code, name and kind; a rule; its range and bearing; a rule; its note; a
/// rule; and the live GOTO destination over the GOTO button.
/// [`update_map_panel`](super::update_map_panel) fills it.
pub(crate) fn spawn_map_panel(parent: &mut ChildSpawnerCommands, icons: &InterfaceIcons) {
    parent.spawn(side_panel()).with_children(|panel| {
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
}
