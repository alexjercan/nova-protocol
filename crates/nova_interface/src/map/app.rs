//! The map pane's components and runtime state.
//!
//! Touch this module when changing what the map pane remembers between frames.

use bevy::prelude::*;

use super::MapContactKind;
use crate::icons::BodyIconType;

/// The map pane's viewport node (holds the RTT image + blip children).
#[derive(Component)]
pub(crate) struct MapViewportMarker;

/// The contact readout line under the viewport.
#[derive(Component)]
pub(crate) struct MapReadoutMarker;

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
    /// A transient "GOTO SET" note shown in the readout for a short time.
    pub(crate) goto_note: Option<(String, f32)>,
}
