//! The MAP app's runtime seam: its id, title, hints, body spawn, and the CLI
//! verbs (`goto`, selection) it registers with the terminal.
//!
//! Touch this module when changing what the map app is called, shows on entry,
//! or accepts as a command.

use bevy::prelude::*;
use nova_os::prelude::*;
use nova_ship::prelude::*;

use super::{contacts::*, *};
use crate::terminal::{nova_os_text_font, DRAWER_LINE_FONT_PX, NOVA_OS_PHOSPHOR_MUTED};

/// Keep the terminal's arg-completion set in sync with the live contact labels,
/// so `map goto <TAB>` offers them. Only writes on a real change.
pub(crate) fn sync_map_arg_completions(
    contacts: MapContacts,
    mut runtime: ResMut<MapRuntime>,
    mut terminal: ResMut<NovaOsTerminal>,
) {
    let labels = contacts.labels();
    if labels == runtime.completion_labels {
        return;
    }
    runtime.completion_labels = labels.clone();
    terminal.merge_arg_completions([("map goto", labels)]);
}

/// Drain the arg-bearing `map goto` verb the terminal queued on submit, resolve
/// the label to a contact, and set a flight [`Autopilot`] GOTO on the player ship
/// (the same seam the in-app `G` key uses). Peeks the shared pending slot first so
/// it never swallows a `ship ...` verb.
pub(crate) fn apply_map_cli_commands(
    mut commands: Commands,
    mut terminal: ResMut<NovaOsTerminal>,
    contacts: MapContacts,
) {
    // Only consume an invocation this handler owns; leave `ship ...` for its system
    // (`cross-app-invocation-peek-before-take`).
    let owns = terminal
        .peek_pending_invocation()
        .is_some_and(|inv| inv.name == "map goto");
    if !owns {
        return;
    }
    let Some(invocation) = terminal.take_pending_invocation() else {
        return;
    };
    let Some(label) = invocation.args.first() else {
        terminal.extend_scrollback([TerminalRow {
            kind: TerminalRowKind::Error,
            text: format!("{}: expected a contact label", invocation.name),
        }]);
        return;
    };
    let Some((player, _, _)) = contacts.player_frame() else {
        terminal.extend_scrollback([TerminalRow {
            kind: TerminalRowKind::Error,
            text: "no live player ship".to_string(),
        }]);
        return;
    };

    let rows = match contacts.resolve(label) {
        Some(contact) if contact.kind == MapContactKind::OwnShip => vec![TerminalRow {
            kind: TerminalRowKind::Dim,
            text: format!("goto: {} is your own ship", contact.code),
        }],
        Some(contact) => {
            commands
                .entity(player)
                .insert(Autopilot::engage(AutopilotAction::Goto {
                    target: contact.entity,
                }));
            vec![TerminalRow {
                kind: TerminalRowKind::Info,
                text: format!(
                    "goto {} ({}): autopilot engaged, range {}",
                    contact.code,
                    contact.name,
                    nova_ui::units::distance(contact.range),
                ),
            }]
        }
        None => {
            let labels = contacts.labels();
            let mut rows = vec![TerminalRow {
                kind: TerminalRowKind::Error,
                text: format!("no such contact: {label}"),
            }];
            if !labels.is_empty() {
                rows.push(TerminalRow {
                    kind: TerminalRowKind::Dim,
                    text: format!("contacts: {}", labels.join("   ")),
                });
            }
            rows
        }
    };
    terminal.extend_scrollback(rows);
}

/// The `map` app: identity + static body shell. All interaction runs in this
/// module's systems (the trait gives no mouse and only discrete keys).
pub(crate) struct MapApp;

impl NovaOsAppRuntime for MapApp {
    fn id(&self) -> &'static str {
        MAP_APP_ID
    }
    fn title(&self) -> &'static str {
        "MAP / LOCAL SPACE"
    }
    fn hints(&self) -> &'static [&'static str] {
        MAP_HINTS
    }
    fn spawn_body(&self, body: &mut ChildSpawnerCommands, font: Handle<Font>) {
        // The viewport shows the offscreen map image (patched in once the RTT
        // exists); blips are added over it as absolutely-positioned children.
        body.spawn((
            MapViewportMarker,
            Node {
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                position_type: PositionType::Relative,
                overflow: Overflow::clip(),
                ..default()
            },
            ImageNode {
                image: Handle::default(),
                ..default()
            },
            BackgroundColor(MAP_VIEW_BG),
        ));
        body.spawn((
            MapReadoutMarker,
            Text::new("Select a contact for range and bearing."),
            nova_os_text_font(DRAWER_LINE_FONT_PX, font),
            TextColor(NOVA_OS_PHOSPHOR_MUTED),
            Node {
                min_height: Val::Px(26.0),
                ..default()
            },
        ));
    }
}

/// Dark fill behind the map image so it reads as a recessed screen.
pub(crate) const MAP_VIEW_BG: Color = Color::srgb_u8(0, 6, 3);

/// The app body's map viewport node (holds the RTT image + blip children).
#[derive(Component)]
pub(crate) struct MapViewportMarker;

/// The contact readout line under the viewport.
#[derive(Component)]
pub(crate) struct MapReadoutMarker;

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

/// The camera eye offset from the focus for a given orbit, on a Y-up sphere.
pub(crate) fn orbit_eye(radius: f32, theta: f32, phi: f32) -> Vec3 {
    let horizontal = radius * phi.cos();
    Vec3::new(
        horizontal * theta.sin(),
        radius * phi.sin(),
        horizontal * theta.cos(),
    )
}

/// The parent of every spawned map-scene entity (camera + proxy meshes), so the
/// whole scene tears down with one `despawn`.
#[derive(Component)]
pub(crate) struct MapSceneRoot;

/// Holds the distance rings + hub; its transform tracks the orbit center so the
/// scale reference surrounds the focused object (`map_focus_follow`).
#[derive(Component)]
pub(crate) struct MapFocusAnchor;

/// A projected contact blip (a clickable UI marker over the viewport image).
#[derive(Component)]
pub(crate) struct MapBlip {
    pub(crate) contact: Entity,
}

/// Live state of the running map app.
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
    /// The labels last pushed as `map goto` arg-completions, so the terminal is
    /// only marked changed when the set changes.
    pub(crate) completion_labels: Vec<String>,
}
