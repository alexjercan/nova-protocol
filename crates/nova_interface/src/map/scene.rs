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
    pane::{interface_shown, legend_entry, themed_label, InterfacePaneType},
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
    let ring_mat = materials.add(unlit(theme.color_alpha(UiColor::Secondary, 0.6)));
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
    if node.image != *image {
        node.image = image.clone();
    }
    let desired = computed.size().round().as_uvec2().max(UVec2::ONE);
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
/// `viewer_prev`, and `map_goto`.
pub(crate) fn map_input(
    mut input: NovaOsAppInput,
    mut runtime: ResMut<MapRuntime>,
    contacts: MapContacts,
    mut commands: Commands,
    mut q_camera: Query<(&mut MapOrbit, &Transform), With<MapCameraMarker>>,
    q_docked: Query<&DockedShip>,
    q_connections: Query<&DockingConnection>,
) {
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

    // GOTO on the selected contact (skip own ship). Sets a flight autopilot on
    // the player ship directly - this intentionally bypasses the normal
    // GOTO capability check (fine for the PoC nav computer). The docked rule
    // is NOT bypassed: a GOTO set without the helm would fly the pair the
    // moment the player takes it, and a pair that cannot be measured is
    // flown by no one.
    if input.just_pressed("map_goto") {
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
                        commands
                            .entity(player)
                            .insert(Autopilot::engage(AutopilotAction::Goto { target: sel }));
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
    // The scene camera draws into an image whose pixels ARE the viewport
    // node's physical pixels, so it answers in physical pixels while `Node`
    // asks for logical ones. Do the conversion once, here.
    let to_logical = computed.inverse_scale_factor();
    let size = computed.size() * to_logical;
    // A node that has not been laid out yet reports an inverse scale factor of
    // 0, which would collapse `size` to zero AND every projected point to the
    // origin - and `Vec2::ZERO` PASSES the bounds filter below. One frame of
    // every blip piled in the corner, each time the panel opens.
    if to_logical <= 0.0 {
        return;
    }
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
        let alpha = if selected { 1.0 } else { 0.0 };
        if border.alpha != alpha {
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
            ThemedFill::alpha(UiColor::Void, 0.55),
            BorderColor::all(Color::NONE),
            ThemedBorder::alpha(UiColor::Accent, 0.0),
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
        ThemedFill::alpha(UiColor::Void, 0.8),
        ChildOf(blip),
        // The blip carries its unique CODE, not the freeform name, so the
        // label you read is the label the readout shows.
        children![themed_label(&contact.code, 12.0, UiColor::Body)],
    ));
    commands.entity(viewport).add_child(blip);
    blip
}

/// Select a contact when its blip button is activated (click or keyboard
/// activation).
pub(crate) fn on_map_blip_click(
    activate: On<Activate>,
    q_blip: Query<&MapBlip>,
    mut runtime: ResMut<MapRuntime>,
) {
    if let Ok(blip) = q_blip.get(activate.entity) {
        runtime.selected = Some(blip.contact);
    }
}

/// Fill the readout from the current selection (or a GOTO flash).
pub(crate) fn update_map_readout(
    runtime: Res<MapRuntime>,
    contacts: MapContacts,
    mut q_readout: Query<(&mut Text, &mut ThemedText), With<MapReadoutMarker>>,
) {
    if !runtime.active {
        return;
    }
    let Ok((mut text, mut themed)) = q_readout.single_mut() else {
        return;
    };
    let (value, color) = if let Some((note, _)) = &runtime.goto_note {
        (note.clone(), UiColor::Accent)
    } else {
        match runtime
            .selected
            .and_then(|sel| contacts.collect().into_iter().find(|c| c.entity == sel))
        {
            Some(contact) if contact.kind == MapContactKind::Hostile => {
                (contact.readout(), UiColor::Danger)
            }
            Some(contact) => (contact.readout(), UiColor::Body),
            None => (
                "Select a contact for range and bearing.".to_string(),
                UiColor::Label,
            ),
        }
    };
    if text.0 != value {
        text.0 = value;
    }
    if themed.color != color {
        themed.color = color;
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

/// Reframe the map when its Reframe button is activated.
pub(crate) fn on_map_reframe_button(
    _activate: On<Activate>,
    contacts: MapContacts,
    mut runtime: ResMut<MapRuntime>,
    mut q_camera: Query<&mut MapOrbit, With<MapCameraMarker>>,
) {
    if let Ok(mut orbit) = q_camera.single_mut() {
        reframe_map(&mut orbit, &contacts, &mut runtime);
    }
}
