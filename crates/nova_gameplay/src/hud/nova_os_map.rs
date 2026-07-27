//! NOVA OS `map` app: a terminal-launched schematic 3D minimap of local space.
//!
//! This is the visual counterpart of the `map view` CLI built-in. Both read the
//! same [`MapContacts`] model (player, allies, enemies, asteroids, objective
//! markers with live range/bearing). The app renders a small schematic 3D scene
//! - concentric distance rings and a central hub - through a dedicated
//! [`Camera3d`] on its own [`RenderLayers`] into an offscreen image, shown in the
//! app body; the interactive CONTACTS ride on top as projected clickable UI
//! blips (a nested 3D mesh would not be pickable through the NOVA OS CRT
//! composite, but UI buttons are - see `tasks/20260724-102320/DECISION.md`).
//!
//! The camera is a [`MapOrbit`] you drive with the keyboard (Q/E turn, R/F tilt)
//! plus WASD (move the focus around) and the wheel (zoom). Selecting a contact fills a
//! readout with kind / name / range / bearing; `G` sets a flight [`Autopilot`]
//! GOTO on the player ship that persists after the computer closes.
//!
//! The app trait ([`NovaOsAppRuntime`]) only hands apps discrete key presses and
//! no mouse, so all of the interaction runs as this module's OWN systems, gated
//! on the map app being the active NOVA OS surface.

use bevy::{
    camera::{visibility::RenderLayers, ImageRenderTarget, RenderTarget},
    ecs::system::SystemParam,
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*,
    render::render_resource::{Extent3d, TextureFormat},
    // The activatable Button (fires `Activate` through the forwarded NOVA OS
    // pointer), matching the terminal's own buttons.
    ui_widgets::{Activate, Button},
};
use nova_events::prelude::EntityTypeName;
use nova_os::prelude::*;

use crate::{
    hud::{
        allegiance_markers::allegiance_color,
        nova_os::{
            nova_os_font, nova_os_text_font, DRAWER_LINE_FONT_PX, NOVA_OS_AMBER, NOVA_OS_PHOSPHOR,
            NOVA_OS_PHOSPHOR_DIM, NOVA_OS_PHOSPHOR_MUTED, NOVA_OS_SCREEN, NOVA_OS_TEXT,
        },
    },
    prelude::*,
};

/// Glob-import surface: `use nova_gameplay::hud::nova_os_map::prelude::*`.
pub mod prelude {
    pub use super::NovaOsMapPlugin;
}

/// The launch word / stable id of the map app.
const MAP_APP_ID: &str = "map";
/// Render layer the map scene + camera live on, isolated from the world (0) and
/// the NOVA OS terminal RTT (20).
const MAP_LAYER: usize = 21;
/// The map camera renders BEFORE the NOVA OS offscreen pass (-20) so its image is
/// ready when the NOVA OS content samples it.
const MAP_CAMERA_ORDER: isize = -30;
/// Distance-ring radii (world units) drawn on the map floor as scale reference.
const MAP_RING_RADII: [f32; 3] = [40.0, 80.0, 120.0];
/// Orbit-radius zoom clamp (world units from the focus).
const MAP_RADIUS_MIN: f32 = 30.0;
const MAP_RADIUS_MAX: f32 = 520.0;
/// Default orbit framing when the app opens or `R` resets the view.
const MAP_RADIUS_DEFAULT: f32 = 170.0;
const MAP_THETA_DEFAULT: f32 = 0.8;
const MAP_PHI_DEFAULT: f32 = 0.62;

/// Footer hints while the map owns the screen (swapped in by the runtime).
const MAP_HINTS: &[&str] = &[
    "WASD: MOVE",
    "Q/E: TURN",
    "R/F: TILT",
    "DRAG: LOOK",
    "WHEEL: ZOOM",
    "[ / ]: CYCLE",
    "G: GOTO",
    "T: RESET",
    "ESC: BACK",
];

// ---------------------------------------------------------------------------
// Contact model (shared by the CLI and the app)
// ---------------------------------------------------------------------------

/// What a map contact is, driving its color language and readout label.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum MapContactKind {
    OwnShip,
    Ally,
    Hostile,
    Objective,
    Terrain,
}

impl MapContactKind {
    fn label(self) -> &'static str {
        match self {
            MapContactKind::OwnShip => "OWN SHIP",
            MapContactKind::Ally => "ALLY",
            MapContactKind::Hostile => "HOSTILE",
            MapContactKind::Objective => "OBJECTIVE",
            MapContactKind::Terrain => "TERRAIN",
        }
    }

    /// Blip / readout tint, consistent with the allegiance-marker palette.
    fn color(self) -> Color {
        match self {
            MapContactKind::OwnShip => NOVA_OS_PHOSPHOR,
            MapContactKind::Ally => allegiance_color(Some(&Allegiance::Player)),
            MapContactKind::Hostile => allegiance_color(Some(&Allegiance::Enemy)),
            MapContactKind::Objective => NOVA_OS_AMBER,
            MapContactKind::Terrain => allegiance_color(Some(&Allegiance::Neutral)),
        }
    }

    fn note(self) -> &'static str {
        match self {
            MapContactKind::OwnShip => "That is you.",
            MapContactKind::Ally => "Friendly contact.",
            MapContactKind::Hostile => "Hostile contact.",
            MapContactKind::Objective => "Mission objective.",
            MapContactKind::Terrain => "Asteroid mass.",
        }
    }
}

/// One plotted contact: its live world position plus range/bearing relative to
/// the player ship.
#[derive(Clone)]
struct MapContact {
    entity: Entity,
    kind: MapContactKind,
    name: String,
    world_pos: Vec3,
    /// Range from the player ship, world units (rendered as `u`, matching the
    /// flight HUD's `u`/`u/s` convention).
    range: f32,
    /// Bearing in the player's local frame: 0 dead ahead, +90 to starboard.
    bearing_deg: f32,
    /// Elevation ("mark") above/below the player's horizontal plane.
    mark_deg: f32,
}

impl MapContact {
    /// The PoC readout line: `KIND / NAME - range X, bearing Y. note`.
    fn readout(&self) -> String {
        if self.kind == MapContactKind::OwnShip {
            return format!(
                "{} / {} - range 0 u, bearing ---. {}",
                self.kind.label(),
                self.name,
                self.kind.note()
            );
        }
        format!(
            "{} / {} - range {:.0} u, bearing {:03.0} mark {:+03.0}. {}",
            self.kind.label(),
            self.name,
            self.range,
            self.bearing_deg,
            self.mark_deg,
            self.kind.note(),
        )
    }

    /// The compact `map view` CLI row.
    fn cli_row(&self) -> String {
        if self.kind == MapContactKind::OwnShip {
            return format!("  {:<9} {:<18} ---", self.kind.label(), self.name);
        }
        format!(
            "  {:<9} {:<18} {:>5.0} u  {:03.0} mark {:+03.0}",
            self.kind.label(),
            self.name,
            self.range,
            self.bearing_deg,
            self.mark_deg,
        )
    }
}

/// Bundled queries that enumerate every plottable contact. Reused by the app
/// systems and by the `map view` CLI so the two never drift.
#[derive(SystemParam)]
pub struct MapContacts<'w, 's> {
    player: Query<
        'w,
        's,
        (Entity, &'static GlobalTransform, Option<&'static Name>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    ships: Query<
        'w,
        's,
        (
            Entity,
            &'static GlobalTransform,
            Option<&'static Name>,
            Option<&'static Allegiance>,
        ),
        (With<SpaceshipRootMarker>, Without<PlayerSpaceshipMarker>),
    >,
    objectives: Query<
        'w,
        's,
        (
            Entity,
            &'static GlobalTransform,
            &'static ObjectiveMarkerTarget,
        ),
    >,
    terrain: Query<
        'w,
        's,
        (Entity, &'static GlobalTransform, &'static EntityTypeName),
        Without<SpaceshipRootMarker>,
    >,
}

impl MapContacts<'_, '_> {
    /// The player ship's entity, world position and orientation, if one exists.
    fn player_frame(&self) -> Option<(Entity, Vec3, Quat)> {
        self.player.iter().next().map(|(entity, gt, _)| {
            let (_, rot, pos) = gt.to_scale_rotation_translation();
            (entity, pos, rot)
        })
    }

    /// The focus point the map orbits (the player, or the world origin).
    fn focus(&self) -> Vec3 {
        self.player_frame()
            .map(|(_, pos, _)| pos)
            .unwrap_or(Vec3::ZERO)
    }

    /// Enumerate every contact with live range/bearing, own ship first.
    fn collect(&self) -> Vec<MapContact> {
        let (player_entity, player_pos, player_rot) = match self.player_frame() {
            Some(frame) => frame,
            None => (Entity::PLACEHOLDER, Vec3::ZERO, Quat::IDENTITY),
        };
        let inv = player_rot.inverse();
        let bearing = |world_pos: Vec3| -> (f32, f32, f32) {
            let rel = world_pos - player_pos;
            let range = rel.length();
            // Player local frame: forward is -Z, starboard +X, up +Y.
            let local = inv * rel;
            let horiz = (local.x * local.x + local.z * local.z).sqrt();
            let mut brg = local.x.atan2(-local.z).to_degrees();
            if brg < 0.0 {
                brg += 360.0;
            }
            let mark = local.y.atan2(horiz.max(f32::EPSILON)).to_degrees();
            (range, brg, mark)
        };

        let mut contacts = Vec::new();
        if let Some((_, _, name)) = self.player.iter().next() {
            contacts.push(MapContact {
                entity: player_entity,
                kind: MapContactKind::OwnShip,
                name: name
                    .map(|n| n.as_str().to_string())
                    .unwrap_or_else(|| "NOVA".to_string()),
                world_pos: player_pos,
                range: 0.0,
                bearing_deg: 0.0,
                mark_deg: 0.0,
            });
        }
        for (entity, gt, name, allegiance) in &self.ships {
            let world_pos = gt.translation();
            let (range, brg, mark) = bearing(world_pos);
            let kind = match allegiance {
                Some(Allegiance::Enemy) => MapContactKind::Hostile,
                Some(Allegiance::Player) => MapContactKind::Ally,
                _ => MapContactKind::Terrain,
            };
            contacts.push(MapContact {
                entity,
                kind,
                name: name
                    .map(|n| n.as_str().to_string())
                    .unwrap_or_else(|| "CONTACT".to_string()),
                world_pos,
                range,
                bearing_deg: brg,
                mark_deg: mark,
            });
        }
        for (entity, gt, marker) in &self.objectives {
            let world_pos = gt.translation();
            let (range, brg, mark) = bearing(world_pos);
            contacts.push(MapContact {
                entity,
                kind: MapContactKind::Objective,
                name: marker.label.to_uppercase(),
                world_pos,
                range,
                bearing_deg: brg,
                mark_deg: mark,
            });
        }
        for (entity, gt, type_name) in &self.terrain {
            if type_name.0 != "asteroid" {
                continue;
            }
            let world_pos = gt.translation();
            let (range, brg, mark) = bearing(world_pos);
            contacts.push(MapContact {
                entity,
                kind: MapContactKind::Terrain,
                name: "ASTEROID".to_string(),
                world_pos,
                range,
                bearing_deg: brg,
                mark_deg: mark,
            });
        }
        contacts
    }
}

/// Build the `map view` CLI rows from the contact model (own ship first, then
/// nearest-first). A pure function so it is unit-testable off a fixed list.
fn map_rows_from_contacts(contacts: &[MapContact]) -> Vec<TerminalRow> {
    let mut rows = vec![
        TerminalRow {
            kind: TerminalRowKind::Info,
            text: "LOCAL SPACE - contacts".to_string(),
        },
        TerminalRow {
            kind: TerminalRowKind::Output,
            text: format!(
                "  {:<9} {:<18} {:>8}  {}",
                "KIND", "NAME", "RANGE", "BEARING"
            ),
        },
    ];
    if contacts.iter().all(|c| c.kind == MapContactKind::OwnShip) {
        rows.push(TerminalRow {
            kind: TerminalRowKind::Warn,
            text: "  no contacts in local space".to_string(),
        });
    }
    // Own ship first, then by ascending range.
    let mut ordered: Vec<&MapContact> = contacts.iter().collect();
    ordered.sort_by(|a, b| {
        let key = |c: &MapContact| (c.kind != MapContactKind::OwnShip, c.range);
        key(a)
            .partial_cmp(&key(b))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for contact in ordered {
        rows.push(TerminalRow {
            kind: TerminalRowKind::Output,
            text: contact.cli_row(),
        });
    }
    rows
}

/// The `map view` rows for the terminal snapshot. Called by the NOVA OS keyboard
/// handler when it builds a command snapshot.
pub fn terminal_map_rows(contacts: &MapContacts) -> Vec<TerminalRow> {
    map_rows_from_contacts(&contacts.collect())
}

// ---------------------------------------------------------------------------
// The app + its runtime state
// ---------------------------------------------------------------------------

/// The `map` app: identity + static body shell. All interaction runs in this
/// module's systems (the trait gives no mouse and only discrete keys).
struct MapApp;

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
const MAP_VIEW_BG: Color = Color::srgb_u8(0, 6, 3);

/// The app body's map viewport node (holds the RTT image + blip children).
#[derive(Component)]
struct MapViewportMarker;

/// The contact readout line under the viewport.
#[derive(Component)]
struct MapReadoutMarker;

/// The map's 3D camera (renders the schematic scene to the offscreen image).
#[derive(Component)]
struct MapCameraMarker;

/// The map camera's orbit state, driven directly (we own the spherical math
/// rather than routing through the shared `SphereOrbit` plugin, whose smoothed
/// input path did not rotate this render-to-texture camera in practice).
/// `theta` is the azimuth, `phi` the elevation above the plane, `radius` the
/// distance from `center`, the focus point WASD pans and selection recenters.
#[derive(Component)]
struct MapOrbit {
    theta: f32,
    phi: f32,
    radius: f32,
    center: Vec3,
}

/// The camera eye offset from the focus for a given orbit, on a Y-up sphere.
fn orbit_eye(radius: f32, theta: f32, phi: f32) -> Vec3 {
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
struct MapSceneRoot;

/// Holds the distance rings + hub; its transform tracks the orbit center so the
/// scale reference surrounds the focused object (`map_focus_follow`).
#[derive(Component)]
struct MapFocusAnchor;

/// A projected contact blip (a clickable UI marker over the viewport image).
#[derive(Component)]
struct MapBlip {
    contact: Entity,
}

/// Live state of the running map app.
#[derive(Resource, Default)]
struct MapRuntime {
    active: bool,
    image: Option<Handle<Image>>,
    camera: Option<Entity>,
    scene_root: Option<Entity>,
    blips: bevy::platform::collections::HashMap<Entity, Entity>,
    selected: Option<Entity>,
    /// The selection the focus last recentered on, so selecting a NEW contact
    /// snaps the map onto it once (without fighting WASD panning after).
    focused_on: Option<Entity>,
    /// A transient "GOTO SET" note shown in the readout for a short time.
    goto_note: Option<(String, f32)>,
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

/// Registers the `map` app and drives its scene, camera, blips and GOTO.
pub struct NovaOsMapPlugin;

impl Plugin for NovaOsMapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MapRuntime>();
        // Register the `map` command tree into the unified NOVA OS command
        // registry (created by NovaOsPlugin, added before this plugin): the launch
        // word `map` (which spawns the app), its `map view` CLI subcommand, and the
        // `MapApp` runtime, all declared together. `sync_nova_os_commands` mirrors
        // these into the terminal's command set.
        app.world_mut()
            .resource_mut::<NovaOsCommandRegistry>()
            .register(
                TerminalCommand::app(MAP_APP_ID, "Open the local-space map", MapApp)
                    .with_subcommand(TerminalCommand::cli(
                        "map view",
                        "Print local-space contacts",
                        CliOutput::Snapshot,
                    )),
            );

        // Scene lifecycle runs unconditionally so it can tear down when the
        // computer closes; the interactive systems gate on the map being active.
        app.add_systems(
            Update,
            (
                manage_map_scene,
                reconcile_map_target,
                map_input,
                map_focus_follow,
                drive_map_camera,
                project_map_blips,
                update_map_readout,
            )
                .chain()
                .in_set(NovaOsMapSystems),
        );
    }
}

/// System set for the map app's per-frame work.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct NovaOsMapSystems;

/// Whether the map app is the active NOVA OS surface right now.
fn map_is_active(pause: &State<PauseStates>, terminal: &NovaOsTerminal) -> bool {
    *pause.get() == PauseStates::NovaOs
        && terminal.active_mode() == TerminalMode::App { id: MAP_APP_ID }
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Spawn the schematic scene + camera on map open, tear it down on close.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_arguments)]
fn manage_map_scene(
    mut commands: Commands,
    pause: Res<State<PauseStates>>,
    terminal: Res<NovaOsTerminal>,
    mut runtime: ResMut<MapRuntime>,
    images: Option<ResMut<Assets<Image>>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    q_player: Query<&GlobalTransform, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
) {
    let active = map_is_active(&pause, &terminal);
    if active == runtime.active {
        return;
    }
    runtime.active = active;

    if !active {
        // Tear the scene + blips down.
        if let Some(root) = runtime.scene_root.take() {
            commands.entity(root).despawn();
        }
        for (_, blip) in runtime.blips.drain() {
            commands.entity(blip).try_despawn();
        }
        runtime.camera = None;
        runtime.image = None;
        runtime.selected = None;
        runtime.focused_on = None;
        runtime.goto_note = None;
        return;
    }

    // Building the scene needs render assets; headless rigs skip it (the CLI +
    // lifecycle still work). `active` is already recorded above.
    let (Some(mut images), Some(mut meshes), Some(mut materials)) = (images, meshes, materials)
    else {
        return;
    };

    // The map opens framed on the player ship (the sim is frozen, so this stays
    // put); WASD pans the focus from here.
    let focus = q_player
        .iter()
        .next()
        .map(|gt| gt.translation())
        .unwrap_or(Vec3::ZERO);

    let image = images.add(new_map_image(UVec2::splat(64)));
    runtime.image = Some(image.clone());

    let ring_mesh: Vec<Handle<Mesh>> = MAP_RING_RADII
        .iter()
        .map(|r| meshes.add(Torus::new(r - 0.35, r + 0.35)))
        .collect();
    let ring_mat = materials.add(unlit(NOVA_OS_PHOSPHOR_DIM.with_alpha(0.5)));
    let hub_mesh = meshes.add(Sphere::new(1.6));
    let hub_mat = materials.add(unlit(NOVA_OS_PHOSPHOR));

    let scene_root = commands
        .spawn((
            MapSceneRoot,
            Name::new("NovaOsMapScene"),
            Transform::default(),
            Visibility::Visible,
        ))
        .id();

    let camera = commands
        .spawn((
            MapCameraMarker,
            Name::new("NovaOsMapCamera"),
            Camera3d::default(),
            Camera {
                order: MAP_CAMERA_ORDER,
                clear_color: ClearColorConfig::Custom(NOVA_OS_SCREEN),
                is_active: true,
                ..default()
            },
            // RenderTarget is a standalone component in this Bevy version (see the
            // NOVA OS RTT camera), not a `Camera` field.
            RenderTarget::Image(ImageRenderTarget {
                handle: image.clone(),
                scale_factor: 1.0,
            }),
            Transform::from_translation(
                focus + orbit_eye(MAP_RADIUS_DEFAULT, MAP_THETA_DEFAULT, MAP_PHI_DEFAULT),
            )
            .looking_at(focus, Vec3::Y),
            RenderLayers::layer(MAP_LAYER),
            MapOrbit {
                theta: MAP_THETA_DEFAULT,
                phi: MAP_PHI_DEFAULT,
                radius: MAP_RADIUS_DEFAULT,
                // Seed the focus on the player ship; WASD pans it from here.
                center: focus,
            },
            ChildOf(scene_root),
        ))
        .id();
    runtime.camera = Some(camera);

    // The distance rings + central hub live under a focus anchor that tracks the
    // orbit center (the selected object, or the player), so the scale reference
    // always surrounds whatever you are looking at (map_focus_follow moves it).
    let anchor = commands
        .spawn((
            MapFocusAnchor,
            Name::new("NovaOsMapFocus"),
            Transform::from_translation(focus),
            Visibility::Visible,
            ChildOf(scene_root),
        ))
        .id();
    for mesh in ring_mesh {
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(ring_mat.clone()),
            Transform::default(),
            RenderLayers::layer(MAP_LAYER),
            ChildOf(anchor),
        ));
    }
    commands.spawn((
        Mesh3d(hub_mesh),
        MeshMaterial3d(hub_mat),
        Transform::default(),
        RenderLayers::layer(MAP_LAYER),
        ChildOf(anchor),
    ));

    runtime.scene_root = Some(scene_root);
}

/// Keep the offscreen image sized 1:1 to the viewport node and the camera pass
/// active; patch the viewport `ImageNode` with the RTT handle.
#[allow(clippy::type_complexity)]
fn reconcile_map_target(
    runtime: Res<MapRuntime>,
    mut images: Option<ResMut<Assets<Image>>>,
    mut q_viewport: Query<(&ComputedNode, &mut ImageNode), With<MapViewportMarker>>,
    mut q_camera: Query<(&mut Camera, &mut Projection), With<MapCameraMarker>>,
) {
    let (Some(image), Some(images)) = (runtime.image.as_ref(), images.as_mut()) else {
        return;
    };
    let Ok((computed, mut node)) = q_viewport.single_mut() else {
        return;
    };
    if node.image != *image {
        node.image = image.clone();
    }
    let desired = computed.size().round().as_uvec2().max(UVec2::ONE);
    let needs_resize = images
        .get(image)
        .map(|img| img.size() != desired)
        .unwrap_or(true);
    if needs_resize {
        if let Some(mut img) = images.get_mut(image) {
            img.resize(Extent3d {
                width: desired.x,
                height: desired.y,
                depth_or_array_layers: 1,
            });
        }
        // Force the camera to re-derive its target info after the in-place swap
        // (`bevy-camera-ignores-runtime-rendertarget-swap`).
        if let Ok((_, mut projection)) = q_camera.single_mut() {
            projection.set_changed();
        }
    }
}

/// Drive the map camera transform from the orbit output. The orbit `center` is
/// the focus point the player pans with WASD (seeded to the player ship on open,
/// reset with `R`); this system must NOT overwrite it, or WASD would snap back
/// every frame.
fn drive_map_camera(mut q_camera: Query<(&mut Transform, &MapOrbit), With<MapCameraMarker>>) {
    let Ok((mut transform, orbit)) = q_camera.single_mut() else {
        return;
    };
    let eye = orbit.center + orbit_eye(orbit.radius, orbit.theta, orbit.phi);
    *transform = Transform::from_translation(eye).looking_at(orbit.center, Vec3::Y);
}

/// The point the map frames: the selected contact if one is picked, else the
/// player ship.
fn focus_point(contacts: &MapContacts, selected: Option<Entity>) -> Vec3 {
    selected
        .and_then(|sel| {
            contacts
                .collect()
                .into_iter()
                .find(|c| c.entity == sel)
                .map(|c| c.world_pos)
        })
        .unwrap_or_else(|| contacts.focus())
}

/// When a NEW contact is selected, snap the orbit center onto it once (so the
/// map + rings recenter on it); after that WASD is free to pan away. Every frame
/// keep the ring/hub anchor sitting on the current center.
fn map_focus_follow(
    mut runtime: ResMut<MapRuntime>,
    contacts: MapContacts,
    mut q_camera: Query<&mut MapOrbit, With<MapCameraMarker>>,
    mut q_anchor: Query<&mut Transform, With<MapFocusAnchor>>,
) {
    if !runtime.active {
        return;
    }
    let Ok(mut orbit) = q_camera.single_mut() else {
        return;
    };
    if runtime.selected != runtime.focused_on {
        if let Some(sel) = runtime.selected {
            if let Some(pos) = contacts
                .collect()
                .into_iter()
                .find(|c| c.entity == sel)
                .map(|c| c.world_pos)
            {
                orbit.center = pos;
            }
        }
        runtime.focused_on = runtime.selected;
    }
    if let Ok(mut anchor) = q_anchor.single_mut() {
        anchor.translation = orbit.center;
    }
}

/// Read mouse + keyboard while the map owns the screen: RMB-drag look, wheel
/// zoom, WASD move, `R` reset, `[`/`]` cycle selection, `G` set GOTO.
#[allow(clippy::too_many_arguments)]
fn map_input(
    pause: Res<State<PauseStates>>,
    terminal: Res<NovaOsTerminal>,
    mut runtime: ResMut<MapRuntime>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut motion: MessageReader<MouseMotion>,
    mut wheel: MessageReader<MouseWheel>,
    time: Res<Time>,
    contacts: MapContacts,
    mut commands: Commands,
    mut q_camera: Query<(&mut MapOrbit, &Transform), With<MapCameraMarker>>,
) {
    // Only touch input while the map owns the screen; at the terminal the mouse
    // and keys belong to the prompt (history scroll, PageUp/PageDown, etc.).
    if !map_is_active(&pause, &terminal) {
        return;
    }
    let motion_delta: Vec2 = motion.read().map(|m| m.delta).sum();
    let wheel_delta: f32 = wheel.read().map(|w| w.y).sum();
    let dt = time.delta_secs().max(1.0 / 240.0);

    // Decay the transient GOTO note.
    if let Some((_, remaining)) = runtime.goto_note.as_mut() {
        *remaining -= dt;
        if *remaining <= 0.0 {
            runtime.goto_note = None;
        }
    }

    if let Ok((mut orbit, transform)) = q_camera.single_mut() {
        // Keyboard orbit: Q/E turn (yaw), R/F tilt (pitch). This is the reliable
        // path - mouse-drag look is unreliable through the NOVA OS pointer
        // forwarding. Applied straight to the orbit angles (no smoothing layer).
        let turn = 1.6 * dt;
        if keys.pressed(KeyCode::KeyQ) {
            orbit.theta += turn;
        }
        if keys.pressed(KeyCode::KeyE) {
            orbit.theta -= turn;
        }
        if keys.pressed(KeyCode::KeyR) {
            orbit.phi = (orbit.phi + turn).min(1.45);
        }
        if keys.pressed(KeyCode::KeyF) {
            orbit.phi = (orbit.phi - turn).max(0.12);
        }
        // Mouse drag (LMB or RMB) also orbits. Gentle sensitivity so a small drag
        // is a small turn.
        if mouse_buttons.pressed(MouseButton::Right) || mouse_buttons.pressed(MouseButton::Left) {
            orbit.theta -= motion_delta.x * 0.0024;
            orbit.phi = (orbit.phi + motion_delta.y * 0.0024).clamp(0.12, 1.45);
        }
        // Wheel zooms the focus distance.
        if wheel_delta != 0.0 {
            orbit.radius =
                (orbit.radius * (1.0 - wheel_delta * 0.12)).clamp(MAP_RADIUS_MIN, MAP_RADIUS_MAX);
        }
        // WASD pans the focus RELATIVE TO THE MAP VIEW (the camera's heading on
        // the ground plane), not the ship: W moves into the screen, D screen-right.
        let mut pan = Vec2::ZERO;
        if keys.pressed(KeyCode::KeyW) {
            pan.y += 1.0;
        }
        if keys.pressed(KeyCode::KeyS) {
            pan.y -= 1.0;
        }
        if keys.pressed(KeyCode::KeyA) {
            pan.x -= 1.0;
        }
        if keys.pressed(KeyCode::KeyD) {
            pan.x += 1.0;
        }
        if pan != Vec2::ZERO {
            let flatten = |v: Vec3| Vec3::new(v.x, 0.0, v.z).normalize_or_zero();
            let forward = flatten(*transform.forward());
            let right = flatten(*transform.right());
            let speed = orbit.radius * 0.8 * dt;
            orbit.center += (forward * pan.y + right * pan.x) * speed;
        }
        // T re-frames on the selected object (or the player if nothing is picked).
        if keys.just_pressed(KeyCode::KeyT) {
            orbit.radius = MAP_RADIUS_DEFAULT;
            orbit.theta = MAP_THETA_DEFAULT;
            orbit.phi = MAP_PHI_DEFAULT;
            orbit.center = focus_point(&contacts, runtime.selected);
            runtime.focused_on = runtime.selected;
        }
    }

    // Cycle selection with [ and ].
    let list = contacts.collect();
    if !list.is_empty() {
        let forward = keys.just_pressed(KeyCode::BracketRight);
        let backward = keys.just_pressed(KeyCode::BracketLeft);
        if forward || backward {
            let current = runtime
                .selected
                .and_then(|sel| list.iter().position(|c| c.entity == sel));
            let len = list.len();
            let next = match current {
                Some(i) if forward => (i + 1) % len,
                Some(i) => (i + len - 1) % len,
                None => 0,
            };
            runtime.selected = Some(list[next].entity);
        }
    }

    // GOTO on the selected contact (skip own ship). Sets a flight autopilot on
    // the player ship directly - this intentionally bypasses the normal
    // `FlightVerb::Goto` grant check (fine for the PoC nav computer).
    if keys.just_pressed(KeyCode::KeyG) {
        if let (Some(sel), Some((player, _, _))) = (runtime.selected, contacts.player_frame()) {
            if let Some(contact) = list.iter().find(|c| c.entity == sel) {
                if contact.kind != MapContactKind::OwnShip {
                    commands
                        .entity(player)
                        .insert(Autopilot::engage(AutopilotAction::Goto { target: sel }));
                    runtime.goto_note = Some((format!("GOTO SET: {}", contact.name), 2.5));
                }
            }
        }
    }
}

/// Project each contact through the map camera into the viewport and keep a
/// clickable UI blip per contact in sync (position, color, selection ring).
#[allow(clippy::type_complexity)]
fn project_map_blips(
    mut commands: Commands,
    mut runtime: ResMut<MapRuntime>,
    asset_server: Option<Res<AssetServer>>,
    contacts: MapContacts,
    time: Res<Time>,
    q_camera: Query<(&Camera, &GlobalTransform), With<MapCameraMarker>>,
    q_viewport: Query<(Entity, &ComputedNode), With<MapViewportMarker>>,
    mut q_blip: Query<(
        &mut Node,
        &mut Visibility,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
) {
    if !runtime.active {
        return;
    }
    let (Ok((camera, cam_gt)), Ok((viewport, computed))) = (q_camera.single(), q_viewport.single())
    else {
        return;
    };
    let size = computed.size();
    let list = contacts.collect();
    let font = nova_os_font(asset_server.as_deref());
    let pulse = 0.6 + 0.4 * (time.elapsed_secs() * 4.0).sin().abs();

    let mut seen = bevy::platform::collections::HashSet::new();
    for contact in &list {
        seen.insert(contact.entity);
        let projected = camera
            .world_to_viewport(cam_gt, contact.world_pos)
            .ok()
            .filter(|p| p.x >= 0.0 && p.y >= 0.0 && p.x <= size.x && p.y <= size.y);
        let selected = runtime.selected == Some(contact.entity);
        let mut base = contact.kind.color();
        if contact.kind == MapContactKind::Hostile {
            base = base.with_alpha(pulse);
        }

        let blip = if let Some(&blip) = runtime.blips.get(&contact.entity) {
            blip
        } else {
            let id = spawn_blip(&mut commands, viewport, contact, font.clone());
            runtime.blips.insert(contact.entity, id);
            id
        };
        if let Ok((mut node, mut vis, mut bg, mut border)) = q_blip.get_mut(blip) {
            match projected {
                Some(p) => {
                    node.left = Val::Px(p.x - MAP_BLIP_PX * 0.5);
                    node.top = Val::Px(p.y - MAP_BLIP_PX * 0.5);
                    *vis = Visibility::Inherited;
                }
                None => *vis = Visibility::Hidden,
            }
            bg.0 = base;
            *border = if selected {
                BorderColor::all(NOVA_OS_AMBER)
            } else {
                BorderColor::all(base.with_alpha(0.0))
            };
        }
    }

    // Drop blips whose contact vanished.
    let stale: Vec<Entity> = runtime
        .blips
        .keys()
        .copied()
        .filter(|c| !seen.contains(c))
        .collect();
    for contact in stale {
        if let Some(blip) = runtime.blips.remove(&contact) {
            commands.entity(blip).try_despawn();
        }
    }
}

/// Blip square side in pixels.
const MAP_BLIP_PX: f32 = 12.0;

fn spawn_blip(
    commands: &mut Commands,
    viewport: Entity,
    contact: &MapContact,
    font: Handle<Font>,
) -> Entity {
    let color = contact.kind.color();
    let id = commands
        .spawn((
            MapBlip {
                contact: contact.entity,
            },
            Button,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(MAP_BLIP_PX),
                height: Val::Px(MAP_BLIP_PX),
                border: UiRect::all(Val::Px(2.0)),
                // Round the blip into a dot rather than a square.
                border_radius: BorderRadius::MAX,
                ..default()
            },
            BorderColor::all(color.with_alpha(0.0)),
            BackgroundColor(color),
        ))
        // Selection goes through the Button `Activate` event (fires for the
        // forwarded NOVA OS pointer), not `Interaction` polling, which does not
        // update through the CRT-composited RTT.
        .observe(on_map_blip_click)
        .id();
    // The label rides beside the blip as a child node.
    commands.spawn((
        Text::new(contact.name.clone()),
        nova_os_text_font(11.0, font),
        TextColor(color),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(16.0),
            top: Val::Px(-3.0),
            ..default()
        },
        ChildOf(id),
    ));
    commands.entity(viewport).add_child(id);
    id
}

/// Select a contact when its blip button is activated (click through the
/// forwarded NOVA OS pointer, or keyboard activation).
fn on_map_blip_click(
    activate: On<Activate>,
    q_blip: Query<&MapBlip>,
    mut runtime: ResMut<MapRuntime>,
) {
    if let Ok(blip) = q_blip.get(activate.entity) {
        runtime.selected = Some(blip.contact);
    }
}

/// Fill the readout from the current selection (or a GOTO flash).
fn update_map_readout(
    runtime: Res<MapRuntime>,
    contacts: MapContacts,
    mut q_readout: Query<(&mut Text, &mut TextColor), With<MapReadoutMarker>>,
) {
    if !runtime.active {
        return;
    }
    let Ok((mut text, mut color)) = q_readout.single_mut() else {
        return;
    };
    if let Some((note, _)) = &runtime.goto_note {
        text.0 = note.clone();
        color.0 = NOVA_OS_AMBER;
        return;
    }
    match runtime
        .selected
        .and_then(|sel| contacts.collect().into_iter().find(|c| c.entity == sel))
    {
        Some(contact) => {
            text.0 = contact.readout();
            color.0 = if contact.kind == MapContactKind::Hostile {
                NOVA_OS_AMBER
            } else {
                NOVA_OS_TEXT
            };
        }
        None => {
            text.0 = "Select a contact for range and bearing.".to_string();
            color.0 = NOVA_OS_PHOSPHOR_MUTED;
        }
    }
}

// ---------------------------------------------------------------------------
// Small render helpers
// ---------------------------------------------------------------------------

fn new_map_image(size: UVec2) -> Image {
    Image::new_target_texture(
        size.x.max(1),
        size.y.max(1),
        TextureFormat::Rgba8UnormSrgb,
        None,
    )
}

/// An unlit emissive-ish material so proxy meshes read at full color without a
/// light on the map layer.
fn unlit(color: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    }
}

#[cfg(test)]
mod tests {
    use bevy::{ecs::system::RunSystemOnce, state::app::StatesPlugin};

    use super::*;

    /// Spawn a scripted local-space scene: own ship at origin (facing -Z), a
    /// hostile dead ahead, an objective to starboard, an asteroid astern.
    fn scripted_world() -> (World, Entity, Entity) {
        let mut world = World::new();
        let player = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.0)),
                Name::new("NOVA"),
            ))
            .id();
        let raider = world
            .spawn((
                SpaceshipRootMarker,
                Allegiance::Enemy,
                GlobalTransform::from(Transform::from_xyz(0.0, 0.0, -50.0)),
                Name::new("RAIDER"),
            ))
            .id();
        world.spawn((
            ObjectiveMarkerTarget {
                label: "salvage".to_string(),
            },
            GlobalTransform::from(Transform::from_xyz(50.0, 0.0, 0.0)),
        ));
        world.spawn((
            EntityTypeName::new("asteroid"),
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 60.0)),
        ));
        (world, player, raider)
    }

    #[test]
    fn map_contacts_report_kinds_range_and_bearing() {
        let (mut world, _player, raider) = scripted_world();
        let contacts = world.run_system_once(|c: MapContacts| c.collect()).unwrap();

        // Own ship is enumerated first.
        assert_eq!(contacts[0].kind, MapContactKind::OwnShip);
        assert_eq!(contacts[0].range, 0.0);

        let find = |kind: MapContactKind| contacts.iter().find(|c| c.kind == kind).unwrap();
        let hostile = find(MapContactKind::Hostile);
        assert_eq!(hostile.entity, raider);
        assert!((hostile.range - 50.0).abs() < 0.01);
        // Dead ahead (-Z) reads bearing ~0.
        assert!(hostile.bearing_deg < 1.0 || hostile.bearing_deg > 359.0);

        let objective = find(MapContactKind::Objective);
        assert!((objective.range - 50.0).abs() < 0.01);
        // Starboard (+X) reads ~090.
        assert!((objective.bearing_deg - 90.0).abs() < 1.0);

        let asteroid = find(MapContactKind::Terrain);
        assert!((asteroid.range - 60.0).abs() < 0.01);
        // Astern (+Z) reads ~180.
        assert!((asteroid.bearing_deg - 180.0).abs() < 1.0);
    }

    #[test]
    fn map_view_rows_render_contacts_and_empty_state() {
        let (mut world, _player, _raider) = scripted_world();
        let contacts = world.run_system_once(|c: MapContacts| c.collect()).unwrap();
        let rows = map_rows_from_contacts(&contacts);
        let joined: String = rows
            .iter()
            .map(|r| r.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains("LOCAL SPACE"));
        assert!(joined.contains("HOSTILE"));
        assert!(joined.contains("RAIDER"));
        assert!(joined.contains("OBJECTIVE"));

        // With only the own ship, the CLI reports no contacts.
        let own_only: Vec<MapContact> = contacts
            .into_iter()
            .filter(|c| c.kind == MapContactKind::OwnShip)
            .collect();
        let empty_rows = map_rows_from_contacts(&own_only);
        assert!(empty_rows.iter().any(|r| r.text.contains("no contacts")));
    }

    /// The scene lifecycle tracks the active NOVA OS surface (headless: no
    /// render assets, so only the active flag toggles - the scene build is
    /// skipped, but open/close is proven).
    #[test]
    fn map_scene_activates_with_the_app_surface() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin));
        app.insert_state(PauseStates::NovaOs);
        app.init_resource::<MapRuntime>();
        app.insert_resource(NovaOsTerminal::default());

        // At the prompt: inactive.
        app.update();
        app.world_mut().run_system_once(manage_map_scene).unwrap();
        assert!(!app.world().resource::<MapRuntime>().active);

        // Launch the map app: active.
        app.world_mut()
            .resource_mut::<NovaOsTerminal>()
            .enter_app(MAP_APP_ID);
        app.world_mut().run_system_once(manage_map_scene).unwrap();
        assert!(app.world().resource::<MapRuntime>().active);

        // Exit back to the terminal: inactive again.
        app.world_mut().resource_mut::<NovaOsTerminal>().exit_app();
        app.world_mut().run_system_once(manage_map_scene).unwrap();
        assert!(!app.world().resource::<MapRuntime>().active);
    }

    /// With the asset stores present, opening the map actually builds the
    /// schematic scene (camera + proxy meshes + RTT image) and the per-frame
    /// systems run without panicking - the path a real GPU would render.
    #[test]
    fn map_scene_builds_and_drives_with_render_assets() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, AssetPlugin::default()));
        app.init_asset::<Image>();
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.insert_state(PauseStates::NovaOs);
        app.init_resource::<MapRuntime>();

        let mut terminal = NovaOsTerminal::default();
        terminal.enter_app(MAP_APP_ID);
        app.insert_resource(terminal);

        app.world_mut().spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.0)),
            Name::new("NOVA"),
        ));

        // Build the scene.
        app.world_mut().run_system_once(manage_map_scene).unwrap();
        {
            let runtime = app.world().resource::<MapRuntime>();
            assert!(runtime.active);
            assert!(runtime.scene_root.is_some(), "scene root spawned");
            assert!(runtime.image.is_some(), "RTT image created");
            assert!(runtime.camera.is_some(), "map camera spawned");
        }
        // A camera entity carries the render layer + orbit.
        let cameras = app
            .world_mut()
            .query_filtered::<(), (With<MapCameraMarker>, With<MapOrbit>)>()
            .iter(app.world())
            .count();
        assert_eq!(cameras, 1, "exactly one orbit map camera");

        // The per-frame systems run without panicking (no viewport UI node here,
        // so projection/reconcile early-return, but the code path is exercised).
        app.world_mut()
            .run_system_once(reconcile_map_target)
            .unwrap();
        app.world_mut().run_system_once(drive_map_camera).unwrap();
        app.world_mut().run_system_once(project_map_blips).unwrap();

        // Closing the app tears the scene down.
        app.world_mut().resource_mut::<NovaOsTerminal>().exit_app();
        app.world_mut().run_system_once(manage_map_scene).unwrap();
        assert!(app.world().resource::<MapRuntime>().scene_root.is_none());
        let remaining = app
            .world_mut()
            .query_filtered::<(), With<MapCameraMarker>>()
            .iter(app.world())
            .count();
        assert_eq!(remaining, 0, "camera despawned on close");
    }

    #[test]
    fn map_focus_follow_recenters_on_a_new_selection() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, AssetPlugin::default()));
        app.init_asset::<Image>();
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.insert_state(PauseStates::NovaOs);
        app.init_resource::<MapRuntime>();

        let mut terminal = NovaOsTerminal::default();
        terminal.enter_app(MAP_APP_ID);
        app.insert_resource(terminal);

        app.world_mut().spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.0)),
            Name::new("NOVA"),
        ));
        let raider = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                Allegiance::Enemy,
                GlobalTransform::from(Transform::from_xyz(90.0, 0.0, -30.0)),
                Name::new("RAIDER"),
            ))
            .id();

        // Build the scene (framed on the player).
        app.world_mut().run_system_once(manage_map_scene).unwrap();

        // Select the raider: the orbit center + ring anchor snap onto it.
        app.world_mut().resource_mut::<MapRuntime>().selected = Some(raider);
        app.world_mut().run_system_once(map_focus_follow).unwrap();

        let center = app
            .world_mut()
            .query_filtered::<&MapOrbit, With<MapCameraMarker>>()
            .single(app.world())
            .unwrap()
            .center;
        assert!(
            center.distance(Vec3::new(90.0, 0.0, -30.0)) < 0.01,
            "the map recenters on the selected contact",
        );
        let anchor = app
            .world_mut()
            .query_filtered::<&Transform, With<MapFocusAnchor>>()
            .single(app.world())
            .unwrap()
            .translation;
        assert!(
            anchor.distance(Vec3::new(90.0, 0.0, -30.0)) < 0.01,
            "the ring anchor follows the focus",
        );
    }

    #[test]
    fn map_goto_sets_autopilot_on_the_player_ship() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, bevy::input::InputPlugin));
        app.insert_state(PauseStates::NovaOs);
        app.init_resource::<MapRuntime>();

        let mut terminal = NovaOsTerminal::default();
        terminal.enter_app(MAP_APP_ID);
        app.insert_resource(terminal);

        let player = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.0)),
                Name::new("NOVA"),
            ))
            .id();
        let target = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                Allegiance::Enemy,
                GlobalTransform::from(Transform::from_xyz(0.0, 0.0, -50.0)),
                Name::new("RAIDER"),
            ))
            .id();

        {
            let mut runtime = app.world_mut().resource_mut::<MapRuntime>();
            runtime.active = true;
            runtime.selected = Some(target);
        }
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyG);

        app.world_mut().run_system_once(map_input).unwrap();

        let autopilot = app
            .world()
            .get::<Autopilot>(player)
            .expect("GOTO inserts an Autopilot on the player ship");
        assert!(
            matches!(autopilot.action, AutopilotAction::Goto { target: t } if t == target),
            "the autopilot targets the selected contact",
        );
    }
}
