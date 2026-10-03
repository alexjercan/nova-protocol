//! The map's 3D view: its own camera into a render target, the orbit controls,
//! and the blip overlay projected back onto the viewport.
//!
//! Blips are UI nodes positioned from the camera projection rather than
//! world-space sprites, so they stay a fixed pixel size and stay clickable.
//!
//! Touch this module when changing how the map renders or how it is navigated.

use bevy::{
    camera::{visibility::RenderLayers, ImageRenderTarget, RenderTarget},
    prelude::*,
    ui::InteractionDisabled,
    // The activatable Button (fires `Activate` on click or keyboard
    // activation), matching the terminal's own buttons.
    ui_widgets::{Activate, Button},
};
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_ui::{
    theme::{ActiveUiTheme, UiColor},
    widget::{ThemedBorder, ThemedFill, ThemedImageTint, ThemedText},
};

use super::{app::*, contacts::*, *};
use crate::{
    icons::{icon_node, BodyIconType, InterfaceIcons},
    pane::{interface_shown, legend_entry, play_menu_select, themed_label, InterfacePaneType},
    terminal::{NovaOsAppInput, NovaOsCloseTransition},
    // The viewer the Map and Ship panes are two framings of.
    viewer::{cycle_index, orbit_eye, unlit, zoom_radius, OrbitGesture},
};

/// Spawn the schematic scene + camera on map open, tear it down on close.
pub(crate) fn manage_map_scene(
    mut commands: Commands,
    pause: Res<State<PauseStates>>,
    close: Res<NovaOsCloseTransition>,
    pane: Res<InterfacePaneType>,
    mut runtime: ResMut<MapRuntime>,
    images: Option<ResMut<Assets<Image>>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    contacts: MapContacts,
    theme: Res<ActiveUiTheme>,
    q_player: Query<&GlobalTransform, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
) {
    // Shown, not active: the scene, camera and selection survive the command
    // modal over the pane. Input gates on the pane owning the screen instead.
    let active = interface_shown(&pause, &close) && *pane == InterfacePaneType::Map;
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
        runtime.goto_requested = false;
        return;
    }

    // Building the scene needs render assets; headless rigs skip it (the pane
    // lifecycle still works). `active` is already recorded above.
    let (Some(mut images), Some(mut meshes), Some(mut materials)) = (images, meshes, materials)
    else {
        return;
    };

    // The map opens framed on the player ship (the sim is frozen, so this stays
    // put); the pan actions move the focus from here.
    let focus = q_player
        .iter()
        .next()
        .map(|gt| gt.translation())
        .unwrap_or(Vec3::ZERO);

    let image = images.add(new_render_target_image(UVec2::splat(64)));
    runtime.image = Some(image.clone());

    // Framed on what is actually out there: a scenario spread over 20 km opens
    // showing all of it, a two-ship skirmish opens where it always did.
    let framing = map_radius_default(map_spread(&contacts, focus));
    // Rings a fixed share of the framing thick, so they read at any spread.
    let half_width = framing * 0.0025;
    let ring_mesh: Vec<Handle<Mesh>> = map_ring_radii(framing)
        .iter()
        .map(|r| meshes.add(Torus::new(r - half_width, r + half_width)))
        .collect();
    // Under the blips: the rings are a scale reference, not a contact.
    let ring_mat = materials.add(unlit(theme.color_alpha(UiColor::Secondary, 0.45)));
    // A UNIT sphere: `map_focus_follow` scales it to whatever is focused, and
    // a sphere is the one shape uniform scaling cannot distort.
    let hub_mesh = meshes.add(Sphere::new(1.0));
    let hub_mat = materials.add(unlit(theme.color(UiColor::Primary)));

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
                clear_color: ClearColorConfig::Custom(theme.color(UiColor::Surface)),
                is_active: true,
                ..default()
            },
            // RenderTarget is a standalone component in this Bevy version, not a
            // `Camera` field.
            RenderTarget::Image(ImageRenderTarget {
                handle: image,
                scale_factor: 1.0,
            }),
            Transform::from_translation(
                focus + orbit_eye(framing, MAP_THETA_DEFAULT, MAP_PHI_DEFAULT),
            )
            .looking_at(focus, Vec3::Y),
            RenderLayers::layer(MAP_LAYER),
            MapOrbit {
                theta: MAP_THETA_DEFAULT,
                phi: MAP_PHI_DEFAULT,
                radius: framing,
                // Seed the focus on the player ship; the pan actions move it.
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
        MapFocusHub,
        Mesh3d(hub_mesh),
        MeshMaterial3d(hub_mat),
        Transform::from_scale(Vec3::splat(MAP_HUB_MIN.to_engine())),
        RenderLayers::layer(MAP_LAYER),
        ChildOf(anchor),
    ));

    runtime.scene_root = Some(scene_root);
}

/// Keep the offscreen image sized 1:1 to the viewport node and the camera pass
/// active; patch the viewport `ImageNode` with the RTT handle.
pub(crate) fn reconcile_map_target(
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
    // A viewport that is not laid out yet measures zero. Attaching the image
    // then lets its placeholder size shape the node, which paints one frame
    // as a square before the real layout lands.
    let desired = computed.size().round().as_uvec2();
    if desired.cmpeq(UVec2::ZERO).any() {
        return;
    }
    if node.image != *image {
        node.image = image.clone();
    }
    resize_render_target(
        images,
        image,
        desired,
        q_camera.single_mut().ok().map(|(_, projection)| projection),
    );
}

/// Drive the map camera transform from the orbit output. The orbit `center` is
/// the focus point `viewer_pan_*` moves (seeded to the player ship on open,
/// re-framed by `viewer_reframe`); this system must NOT overwrite it, or a pan
/// would snap back every frame.
pub(crate) fn drive_map_camera(
    mut q_camera: Query<(&mut Transform, &MapOrbit), With<MapCameraMarker>>,
) {
    let Ok((mut transform, orbit)) = q_camera.single_mut() else {
        return;
    };
    let eye = orbit.center + orbit_eye(orbit.radius, orbit.theta, orbit.phi);
    *transform = Transform::from_translation(eye).looking_at(orbit.center, Vec3::Y);
}

/// The point the map frames: the selected contact if one is picked, else the
/// player ship.
pub(crate) fn focus_point(contacts: &MapContacts, selected: Option<Entity>) -> Vec3 {
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
/// map + rings recenter on it); after that a pan is free to move away. Every frame
/// keep the ring/hub anchor sitting on the current center.
pub(crate) fn map_focus_follow(
    mut runtime: ResMut<MapRuntime>,
    contacts: MapContacts,
    mut q_camera: Query<&mut MapOrbit, With<MapCameraMarker>>,
    mut q_anchor: Query<&mut Transform, (With<MapFocusAnchor>, Without<MapFocusHub>)>,
    mut q_hub: Query<&mut Transform, With<MapFocusHub>>,
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
    // The hub says how big the thing you are looking at IS, not how big a hub
    // is: a fixed 16 m sphere buried the skiff it marked and vanished inside
    // the planetoid it marked.
    if let Ok(mut hub) = q_hub.single_mut() {
        let radius = runtime
            .focused_on
            .and_then(|focused| contacts.radius_of(focused))
            .unwrap_or(0.0)
            .max(MAP_HUB_MIN.to_engine());
        hub.scale = Vec3::splat(radius);
    }
}

/// Read mouse + actions while the map owns the screen: RMB-drag look and wheel
/// zoom are raw pointer gestures; everything else is a named action -
/// `viewer_pan_*`, `viewer_orbit_*`, `viewer_reframe`, `viewer_next` /
/// `viewer_prev`, and `map_goto`. The GOTO button's request joins `map_goto`
/// here, so both pass the same checks.
pub(crate) fn map_input(
    mut input: NovaOsAppInput,
    mut runtime: ResMut<MapRuntime>,
    contacts: MapContacts,
    mut commands: Commands,
    mut q_camera: Query<(&mut MapOrbit, &Transform), With<MapCameraMarker>>,
    q_docked: Query<&DockedShip>,
    q_connections: Query<&DockingConnection>,
) {
    // Taken before the gate: a request the map cannot act on now is dropped,
    // not held for a later frame.
    let button_goto = runtime.goto_requested;
    if button_goto {
        runtime.goto_requested = false;
    }
    // Only touch input while the map owns the screen; at the terminal the mouse
    // and keys belong to the prompt (history scroll, PageUp/PageDown, etc.).
    if !input.app_is_active(InterfacePaneType::Map) {
        return;
    }
    let motion_delta = input.motion_delta();
    let wheel_delta = input.wheel_delta();
    let dt = input.dt();

    // Decay the transient GOTO note.
    if let Some((_, remaining)) = runtime.goto_note.as_mut() {
        *remaining -= dt;
        if *remaining <= 0.0 {
            runtime.goto_note = None;
        }
    }

    if let Ok((mut orbit, transform)) = q_camera.single_mut() {
        // Turn, tilt and RMB-drag are the shared viewer's feel, not the map's.
        let gesture = OrbitGesture::read(&input, motion_delta);
        if !gesture.is_idle() {
            let (theta, phi) = gesture.apply(dt, orbit.theta, orbit.phi);
            orbit.theta = theta;
            orbit.phi = phi;
        }
        // Wheel zooms the focus distance, out to what the live scene needs:
        // a fixed ceiling left a contact 20 km out permanently off the map.
        if wheel_delta != 0.0 {
            let reach = map_radius_max(map_spread(&contacts, orbit.center));
            orbit.radius = zoom_radius(orbit.radius, wheel_delta, MAP_RADIUS_MIN, reach);
        }
        // The pan actions move the focus RELATIVE TO THE MAP VIEW (the camera's
        // heading on the ground plane), not the ship: forward goes into the
        // screen, right goes screen-right.
        let mut pan = Vec2::ZERO;
        if input.pressed("viewer_pan_forward") {
            pan.y += 1.0;
        }
        if input.pressed("viewer_pan_back") {
            pan.y -= 1.0;
        }
        if input.pressed("viewer_pan_left") {
            pan.x -= 1.0;
        }
        if input.pressed("viewer_pan_right") {
            pan.x += 1.0;
        }
        if pan != Vec2::ZERO {
            let flatten = |v: Vec3| Vec3::new(v.x, 0.0, v.z).normalize_or_zero();
            let forward = flatten(*transform.forward());
            let right = flatten(*transform.right());
            let speed = orbit.radius * 0.8 * dt;
            orbit.center += (forward * pan.y + right * pan.x) * speed;
        }
        // Re-frame on the selected object (or the player if nothing is picked).
        if input.just_pressed("viewer_reframe") {
            reframe_map(&mut orbit, &contacts, &mut runtime);
        }
    }

    // Cycle selection.
    let list = contacts.collect();
    if !list.is_empty() {
        let forward = input.just_pressed("viewer_next");
        let backward = input.just_pressed("viewer_prev");
        if forward || backward {
            let current = runtime
                .selected
                .and_then(|sel| list.iter().position(|c| c.entity == sel));
            if let Some(next) = cycle_index(current, list.len(), forward) {
                runtime.selected = Some(list[next].entity);
            }
        }
    }

    // GOTO on the selected contact (skip own ship). Sets a flight autopilot
    // and the travel lock on the player ship directly - this intentionally
    // bypasses the normal GOTO capability check (fine for the PoC nav
    // computer). The docked rule is NOT bypassed: a GOTO set without the helm
    // would fly the pair the moment the player takes it, and a pair that
    // cannot be measured is flown by no one. A refusal writes only the note.
    if input.just_pressed("map_goto") || button_goto {
        if let (Some(sel), Some((player, _, _))) = (runtime.selected, contacts.player_frame()) {
            if let Some(contact) = list.iter().find(|c| c.entity == sel) {
                if contact.kind != MapContactKind::OwnShip {
                    let docked = q_docked.get(player).ok();
                    let connection =
                        docked.and_then(|docked| q_connections.get(docked.connection).ok());
                    let note = if connection.is_some_and(|connection| connection.measurement_fault)
                    {
                        "GOTO REFUSED: HELM FAULT".to_string()
                    } else if docked.is_some_and(|docked| !docked.drives) {
                        "GOTO REFUSED: TAKE THE HELM".to_string()
                    } else if connection.is_some_and(|connection| connection.joins(sel)) {
                        "GOTO REFUSED: DOCKED PARTNER".to_string()
                    } else {
                        commands.entity(player).insert((
                            Autopilot::engage(AutopilotAction::Goto { target: sel }),
                            TravelLock(Some(sel)),
                        ));
                        format!("GOTO SET: {}", contact.name)
                    };
                    runtime.goto_note = Some((note, 2.5));
                }
            }
        }
    }
}

/// Project each contact through the map camera into the viewport and keep a
/// clickable UI blip per contact in sync: position, stance tint, the
/// selection border and whether its [`MapBlipLabel`] shows. Writes only on a
/// change, so still blips stay unchanged for the UI layout.
#[expect(
    clippy::too_many_arguments,
    reason = "blips, their parts and the scene"
)]
pub(crate) fn project_map_blips(
    mut commands: Commands,
    mut runtime: ResMut<MapRuntime>,
    icons: Res<InterfaceIcons>,
    contacts: MapContacts,
    q_camera: Query<(&Camera, &GlobalTransform), With<MapCameraMarker>>,
    q_viewport: Query<(Entity, &ComputedNode), With<MapViewportMarker>>,
    mut q_blip: Query<(
        &mut MapBlip,
        &mut Node,
        &mut Visibility,
        &mut ThemedBorder,
        &mut ZIndex,
        &Children,
    )>,
    mut q_tint: Query<&mut ThemedImageTint>,
    mut q_label: Query<&mut Visibility, (With<MapBlipLabel>, Without<MapBlip>)>,
) {
    if !runtime.active {
        return;
    }
    let (Ok((camera, cam_gt)), Ok((viewport, computed))) = (q_camera.single(), q_viewport.single())
    else {
        return;
    };
    // The camera re-derives its target size in `PostUpdate`, so in the frame
    // the target reconciler resizes the image it still projects into the old
    // size. On open that is the small placeholder: every point lands in the
    // top-left corner and passes the bounds filter below. A viewport that is
    // not laid out yet measures zero and never matches. Only position and
    // visibility wait for it; new blips still spawn hidden, so the legend
    // does not wait.
    let stale_camera = camera.physical_target_size() != Some(computed.size().round().as_uvec2());
    // The scene camera draws into an image whose pixels ARE the viewport
    // node's physical pixels, so it answers in physical pixels while `Node`
    // asks for logical ones. Do the conversion once, here.
    let to_logical = computed.inverse_scale_factor();
    let size = computed.size() * to_logical;
    let list = contacts.collect();

    let mut seen = bevy::platform::collections::HashSet::new();
    for contact in &list {
        seen.insert(contact.entity);
        let Some(&blip) = runtime.blips.get(&contact.entity) else {
            let id = spawn_blip(&mut commands, viewport, contact, &icons);
            runtime.blips.insert(contact.entity, id);
            continue;
        };
        let Ok((mut marker, mut node, mut vis, mut border, mut z, children)) = q_blip.get_mut(blip)
        else {
            continue;
        };
        if !stale_camera {
            let projected = camera
                .world_to_viewport(cam_gt, contact.world_pos)
                .ok()
                .map(|p| p * to_logical)
                .filter(|p| p.x >= 0.0 && p.y >= 0.0 && p.x <= size.x && p.y <= size.y);
            match projected {
                Some(p) => {
                    let (left, top) = (
                        Val::Px(p.x - MAP_BLIP_PX * 0.5),
                        Val::Px(p.y - MAP_BLIP_PX * 0.5),
                    );
                    if node.left != left || node.top != top {
                        node.left = left;
                        node.top = top;
                    }
                    vis.set_if_neq(Visibility::Inherited);
                }
                None => {
                    vis.set_if_neq(Visibility::Hidden);
                }
            }
        }
        // A ship can change sides while the map is open.
        if marker.kind != contact.kind {
            marker.kind = contact.kind;
            for child in children.iter() {
                if let Ok(mut tint) = q_tint.get_mut(child) {
                    tint.color = contact.kind.color();
                }
            }
        }
        let selected = runtime.selected == Some(contact.entity);
        let (color, alpha) = blip_border(selected);
        if border.color != color || border.alpha != alpha {
            border.color = color;
            border.alpha = alpha;
        }
        let labelled = selected || contact.kind != MapContactKind::Terrain;
        let label = if labelled {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        // A shown label draws over the unlabelled rocks around it, and the
        // selection over everything, so a belt cannot cover the code you read.
        let order = ZIndex(i32::from(labelled) + i32::from(selected));
        if *z != order {
            *z = order;
        }
        for child in children.iter() {
            if let Ok(mut visibility) = q_label.get_mut(child) {
                visibility.set_if_neq(label);
            }
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

/// Thickness of the GOTO route line, in logical px.
const MAP_ROUTE_PX: f32 = 2.0;
/// Height of the `GOTO` tag over the target blip, in logical px.
const MAP_GOTO_MARKER_PX: f32 = 18.0;

/// Draw the route line from the player ship to its live GOTO target and the
/// `GOTO` tag over that target, through the same camera projection as the
/// blips. Both follow the player's `Autopilot` GOTO, not the selection or the
/// travel lock, so they hide when the GOTO is cancelled, arrives or is
/// replaced by another order. A target the map does not plot draws nothing.
/// Spawns both nodes under the viewport on first run; route endpoints are
/// clipped before sizing the rotated line so its stroke stays in the viewport.
#[expect(
    clippy::type_complexity,
    reason = "the route line and the marker are two disjoint node queries"
)]
pub(crate) fn project_map_route(
    mut commands: Commands,
    runtime: Res<MapRuntime>,
    contacts: MapContacts,
    q_autopilot: Query<&Autopilot>,
    q_camera: Query<(&Camera, &GlobalTransform), With<MapCameraMarker>>,
    q_viewport: Query<(Entity, &ComputedNode), With<MapViewportMarker>>,
    mut q_line: Query<
        (&mut Node, &mut UiTransform, &mut Visibility),
        (With<MapRouteLine>, Without<MapGotoMarker>),
    >,
    mut q_marker: Query<(&mut Node, &mut Visibility), (With<MapGotoMarker>, Without<MapRouteLine>)>,
) {
    if !runtime.active {
        return;
    }
    let (Ok((camera, cam_gt)), Ok((viewport, computed))) = (q_camera.single(), q_viewport.single())
    else {
        return;
    };
    let (Ok((mut line, mut transform, mut line_vis)), Ok((mut marker, mut marker_vis))) =
        (q_line.single_mut(), q_marker.single_mut())
    else {
        spawn_map_route(&mut commands, viewport);
        return;
    };
    let route = contacts.player_frame().and_then(|(player, from, _)| {
        let Ok(Autopilot {
            action: AutopilotAction::Goto { target },
            ..
        }) = q_autopilot.get(player)
        else {
            return None;
        };
        contacts
            .collect()
            .into_iter()
            .find(|contact| contact.entity == *target)
            .map(|contact| (from, contact.world_pos))
    });
    // The same stale-target rule as the blips: the route must meet them.
    let stale_camera = camera.physical_target_size() != Some(computed.size().round().as_uvec2());
    if route.is_some() && stale_camera {
        return;
    }
    let to_logical = computed.inverse_scale_factor();
    // Off-viewport ends still project; only a point behind the camera fails.
    let Some((from, to)) = route.and_then(|(from, to)| {
        let from = camera.world_to_viewport(cam_gt, from).ok()?;
        let to = camera.world_to_viewport(cam_gt, to).ok()?;
        Some((from * to_logical, to * to_logical))
    }) else {
        line_vis.set_if_neq(Visibility::Hidden);
        marker_vis.set_if_neq(Visibility::Hidden);
        return;
    };

    // The tag sits on the target tile's top edge, so it never covers the tile
    // or the code label beside it. It follows the target even when the route
    // misses the viewport.
    let (left, top) = (
        Val::Px(to.x - MAP_BLIP_PX * 0.5),
        Val::Px(to.y - MAP_BLIP_PX * 0.5 - MAP_GOTO_MARKER_PX),
    );
    if marker.left != left || marker.top != top {
        marker.left = left;
        marker.top = top;
    }
    marker_vis.set_if_neq(Visibility::Inherited);

    let size = computed.size() * to_logical;
    let inset = MAP_ROUTE_PX * 0.5;
    let max = size - Vec2::splat(inset);
    let span = to - from;
    if !size.is_finite() || !from.is_finite() || !to.is_finite() || max.min_element() <= inset {
        line_vis.set_if_neq(Visibility::Hidden);
        return;
    }
    // Clip the logical-pixel segment against the stroke-inset viewport. A
    // rotated UI node can escape the parent's clip even if its drawn pixels do
    // not; bound the drawn endpoints before deriving its size and rotation.
    let mut enter = 0.0_f32;
    let mut exit = 1.0_f32;
    for (p, q) in [
        (-span.x, from.x - inset),
        (span.x, max.x - from.x),
        (-span.y, from.y - inset),
        (span.y, max.y - from.y),
    ] {
        if p == 0.0 {
            if q < 0.0 {
                line_vis.set_if_neq(Visibility::Hidden);
                return;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                enter = enter.max(t);
            } else {
                exit = exit.min(t);
            }
        }
    }
    if enter >= exit || !enter.is_finite() || !exit.is_finite() {
        line_vis.set_if_neq(Visibility::Hidden);
        return;
    }
    let clipped_from = from + span * enter;
    let clipped_to = from + span * exit;
    let clipped_span = clipped_to - clipped_from;
    let length = clipped_span.length();
    if !length.is_finite() || length <= f32::EPSILON {
        line_vis.set_if_neq(Visibility::Hidden);
        return;
    }
    // `UiTransform` rotates a horizontal bar about its centre, clockwise in
    // screen space, so the angle is read with y down.
    let mid = (clipped_from + clipped_to) * 0.5;
    let (left, top, width) = (
        Val::Px(mid.x - length * 0.5),
        Val::Px(mid.y - MAP_ROUTE_PX * 0.5),
        Val::Px(length),
    );
    if line.left != left || line.top != top || line.width != width {
        line.left = left;
        line.top = top;
        line.width = width;
    }
    let rotation = Rot2::radians(clipped_span.y.atan2(clipped_span.x));
    if transform.rotation != rotation {
        transform.rotation = rotation;
    }
    line_vis.set_if_neq(Visibility::Inherited);
}

/// Spawn the hidden route line and `GOTO` tag under the viewport, below the
/// blips (which sit at `ZIndex` 0 and up), and out of picking.
fn spawn_map_route(commands: &mut Commands, viewport: Entity) {
    commands.spawn((
        MapRouteLine,
        Node {
            position_type: PositionType::Absolute,
            height: Val::Px(MAP_ROUTE_PX),
            ..default()
        },
        UiTransform::default(),
        Visibility::Hidden,
        ZIndex(-1),
        Pickable::IGNORE,
        BackgroundColor(Color::NONE),
        ThemedFill::alpha(UiColor::Accent, 0.8),
        ChildOf(viewport),
    ));
    commands.spawn((
        MapGotoMarker,
        Node {
            position_type: PositionType::Absolute,
            height: Val::Px(MAP_GOTO_MARKER_PX),
            padding: UiRect::horizontal(Val::Px(4.0)),
            border: UiRect::all(Val::Px(1.0)),
            align_items: AlignItems::Center,
            ..default()
        },
        Visibility::Hidden,
        ZIndex(-1),
        Pickable::IGNORE,
        BackgroundColor(Color::NONE),
        ThemedFill::alpha(UiColor::Void, 0.85),
        BorderColor::all(Color::NONE),
        ThemedBorder::alpha(UiColor::Accent, 1.0),
        ChildOf(viewport),
        children![(
            themed_label("GOTO", 11.0, UiColor::Accent),
            Pickable::IGNORE
        )],
    ));
}

/// Colour and alpha of a blip tile's border. An unselected tile keeps a faint
/// edge so it reads against a bright body behind it; the selection is a full
/// accent edge, and its label also shows on an asteroid, so it does not rely
/// on colour alone.
fn blip_border(selected: bool) -> (UiColor, f32) {
    if selected {
        (UiColor::Accent, 1.0)
    } else {
        (UiColor::Secondary, 0.20)
    }
}

/// Edge of a map blip tile, in logical px.
pub(crate) const MAP_BLIP_PX: f32 = 22.0;
/// Border width of a map blip tile, in logical px.
pub(crate) const MAP_BLIP_BORDER_PX: f32 = 2.0;

/// A clickable contact tile over the viewport: the body icon in its stance
/// colour on a dark tile, and its code to the right. The border marks the
/// selection.
pub(crate) fn spawn_blip(
    commands: &mut Commands,
    viewport: Entity,
    contact: &MapContact,
    icons: &InterfaceIcons,
) -> Entity {
    let blip = commands
        .spawn((
            MapBlip {
                contact: contact.entity,
                body: contact.body,
                kind: contact.kind,
            },
            Button,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(MAP_BLIP_PX),
                height: Val::Px(MAP_BLIP_PX),
                border: UiRect::all(Val::Px(MAP_BLIP_BORDER_PX)),
                border_radius: BorderRadius::all(Val::Px(5.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            // Hidden until the first projection places it.
            Visibility::Hidden,
            BackgroundColor(Color::NONE),
            ThemedFill::alpha(UiColor::Void, 0.75),
            BorderColor::all(Color::NONE),
            {
                let (color, alpha) = blip_border(false);
                ThemedBorder::alpha(color, alpha)
            },
            children![icon_node(
                icons.body(contact.body),
                contact.kind.color(),
                MAP_BLIP_PX - 6.0,
            )],
        ))
        // Selection goes through the Button `Activate` event, not `Interaction`
        // polling.
        .observe(on_map_blip_click)
        .id();
    // The label is a child, so a click on it bubbles to the blip `Button`.
    // `project_map_blips` hides it on an unselected asteroid.
    // `left` is measured from the tile's padding edge, inside its border, so
    // backing it off by the border lands the label on the tile's outer edge
    // and tile and label are one unbroken hit target.
    commands.spawn((
        MapBlipLabel,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(MAP_BLIP_PX - MAP_BLIP_BORDER_PX),
            top: Val::Px(MAP_BLIP_PX * 0.5 - 10.0),
            padding: UiRect::axes(Val::Px(4.0), Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(Color::NONE),
        ThemedFill::alpha(UiColor::Void, 0.85),
        ChildOf(blip),
        // The blip carries its unique CODE, not the freeform name, so the
        // label you read is the label the contact panel shows.
        children![themed_label(&contact.code, 12.0, UiColor::Body)],
    ));
    commands.entity(viewport).add_child(blip);
    blip
}

/// Select a contact when its blip button is activated (click or keyboard
/// activation), with one click if the selection changes.
pub(crate) fn on_map_blip_click(
    activate: On<Activate>,
    q_blip: Query<&MapBlip>,
    mut runtime: ResMut<MapRuntime>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    if let Ok(blip) = q_blip.get(activate.entity) {
        if runtime.selected != Some(blip.contact) {
            runtime.selected = Some(blip.contact);
            play_menu_select(&mut commands, bank.as_deref());
        }
    }
}

/// Request a GOTO on the selection when the GOTO button is activated.
/// [`map_input`] takes the request with the `map_goto` key, so the button
/// passes the same checks.
pub(crate) fn on_map_goto_button(_activate: On<Activate>, mut runtime: ResMut<MapRuntime>) {
    runtime.goto_requested = true;
}

/// Fill the contact panel from the selection, with a GOTO result on the note
/// while it shows, the player's live GOTO destination, and the GOTO button
/// enabled only on a contact other than the own ship. Rewrites text, colour,
/// the icon and the button state in place and only on a difference, so a held
/// selection writes nothing.
#[expect(
    clippy::too_many_arguments,
    reason = "the panel's text, icon and button, and the autopilot it reads"
)]
pub(crate) fn update_map_panel(
    mut commands: Commands,
    runtime: Res<MapRuntime>,
    icons: Res<InterfaceIcons>,
    contacts: MapContacts,
    q_autopilot: Query<&Autopilot>,
    mut q_field: Query<(&MapPanelField, &mut Text, &mut ThemedText)>,
    mut q_icon: Query<(&mut ImageNode, &mut ThemedImageTint, &mut Visibility), With<MapPanelIcon>>,
    q_goto_button: Query<(Entity, Has<InteractionDisabled>), With<MapGotoButton>>,
) {
    if !runtime.active {
        return;
    }
    let list = contacts.collect();
    let selected = runtime
        .selected
        .and_then(|sel| list.iter().find(|c| c.entity == sel));
    let destination =
        contacts
            .player_frame()
            .and_then(|(player, _, _)| match q_autopilot.get(player) {
                Ok(Autopilot {
                    action: AutopilotAction::Goto { target },
                    ..
                }) => list.iter().find(|c| c.entity == *target),
                _ => None,
            });
    let destination = match destination {
        Some(contact) => (
            format!("GOTO {}  {}", contact.code, contact.range_text()),
            UiColor::Accent,
        ),
        None => ("No GOTO set".to_string(), UiColor::Label),
    };
    let note = match (&runtime.goto_note, &selected) {
        (Some((goto, _)), _) => (goto.clone(), UiColor::Accent),
        (None, Some(contact)) => (contact.kind.note().to_string(), UiColor::Label),
        (None, None) => ("Click a contact to select it.".to_string(), UiColor::Label),
    };

    for (field, mut text, mut themed) in &mut q_field {
        let (value, color) = match (&selected, field) {
            (_, MapPanelField::Note) => note.clone(),
            (_, MapPanelField::Destination) => destination.clone(),
            (Some(contact), MapPanelField::Code) => (contact.code.clone(), UiColor::Primary),
            (Some(contact), MapPanelField::Name) => (contact.name.clone(), UiColor::Body),
            (Some(contact), MapPanelField::Kind) => {
                (contact.kind.label().to_string(), contact.kind.color())
            }
            (Some(contact), MapPanelField::Range) => {
                (format!("Range {}", contact.range_text()), UiColor::Primary)
            }
            (Some(contact), MapPanelField::Bearing) => (contact.bearing_text(), UiColor::Primary),
            (None, MapPanelField::Code) => ("No contact".to_string(), UiColor::Label),
            (
                None,
                MapPanelField::Name
                | MapPanelField::Kind
                | MapPanelField::Range
                | MapPanelField::Bearing,
            ) => (String::new(), UiColor::Body),
        };
        if text.0 != value {
            text.0 = value;
        }
        if themed.color != color {
            themed.color = color;
        }
    }

    let goto_ready = selected.is_some_and(|contact| contact.kind != MapContactKind::OwnShip);
    for (button, disabled) in &q_goto_button {
        if goto_ready && disabled {
            commands.entity(button).remove::<InteractionDisabled>();
        } else if !goto_ready && !disabled {
            commands.entity(button).insert(InteractionDisabled);
        }
    }

    for (mut image, mut tint, mut visibility) in &mut q_icon {
        let Some(contact) = selected else {
            visibility.set_if_neq(Visibility::Hidden);
            continue;
        };
        let wanted = icons.body(contact.body);
        if image.image != wanted {
            image.image = wanted;
        }
        if tint.color != contact.kind.color() {
            tint.color = contact.kind.color();
        }
        visibility.set_if_neq(Visibility::Inherited);
    }
}

/// Legend order: ships by stance, then rocks, planets and nav points.
fn legend_rank((body, kind): (BodyIconType, MapContactKind)) -> (usize, usize) {
    (body as usize, kind.code_slot())
}

/// The legend word of a body drawn in a stance.
fn legend_label((body, kind): (BodyIconType, MapContactKind)) -> &'static str {
    match (body, kind) {
        (BodyIconType::Ship, MapContactKind::OwnShip) => "Own ship",
        (BodyIconType::Ship, MapContactKind::Ally) => "Ally ship",
        (BodyIconType::Ship, MapContactKind::Hostile) => "Hostile ship",
        (BodyIconType::Ship, _) => "Neutral ship",
        (BodyIconType::Asteroid, _) => "Asteroid",
        (BodyIconType::Planet, _) => "Planet",
        (BodyIconType::Objective, _) => "Objective",
    }
}

/// Refill the map legend with one entry per body and stance the map plots,
/// when that set changes or the legend is new.
pub(crate) fn refresh_map_legend(
    mut commands: Commands,
    icons: Res<InterfaceIcons>,
    q_blip: Query<&MapBlip>,
    q_legend: Query<(Entity, Ref<MapLegendMarker>)>,
    mut shown: Local<Vec<(BodyIconType, MapContactKind)>>,
) {
    let Ok((legend, slot)) = q_legend.single() else {
        return;
    };
    let mut marks: Vec<(BodyIconType, MapContactKind)> =
        q_blip.iter().map(|blip| (blip.body, blip.kind)).collect();
    marks.sort_by_key(|mark| legend_rank(*mark));
    marks.dedup();
    if !slot.is_added() && *shown == marks {
        return;
    }
    commands
        .entity(legend)
        .despawn_related::<Children>()
        .with_children(|legend| {
            for mark in &marks {
                legend.spawn(legend_entry()).with_children(|entry| {
                    entry.spawn(icon_node(icons.body(mark.0), mark.1.color(), 16.0));
                    entry.spawn(themed_label(legend_label(*mark), 12.0, UiColor::Body));
                });
            }
        });
    *shown = marks;
}

/// Put the map back on its opening framing around the selected contact, or
/// the player with nothing picked. Shared by `viewer_reframe` and Reframe.
fn reframe_map(orbit: &mut MapOrbit, contacts: &MapContacts, runtime: &mut MapRuntime) {
    orbit.center = focus_point(contacts, runtime.selected);
    orbit.radius = map_radius_default(map_spread(contacts, orbit.center));
    orbit.theta = MAP_THETA_DEFAULT;
    orbit.phi = MAP_PHI_DEFAULT;
    runtime.focused_on = runtime.selected;
}

/// Reframe the map when its Reframe button is activated, with one click.
pub(crate) fn on_map_reframe_button(
    _activate: On<Activate>,
    contacts: MapContacts,
    mut runtime: ResMut<MapRuntime>,
    mut q_camera: Query<&mut MapOrbit, With<MapCameraMarker>>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    if let Ok(mut orbit) = q_camera.single_mut() {
        reframe_map(&mut orbit, &contacts, &mut runtime);
        play_menu_select(&mut commands, bank.as_deref());
    }
}
