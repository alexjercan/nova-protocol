use bevy::{
    camera::{visibility::RenderLayers, ImageRenderTarget, RenderTarget},
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*,
    render::render_resource::{Extent3d, TextureFormat},
    // The activatable Button (fires `Activate` through the forwarded NOVA OS
    // pointer), matching the terminal's own buttons.
    ui_widgets::{Activate, Button},
};
use nova_os::prelude::*;
use nova_ui::font::UiFont;

use super::{app::*, contacts::*, *};
use crate::{
    hud::nova_os::{
        nova_os_font, nova_os_text_font, NOVA_OS_AMBER, NOVA_OS_PHOSPHOR, NOVA_OS_PHOSPHOR_DIM,
        NOVA_OS_PHOSPHOR_MUTED, NOVA_OS_SCREEN, NOVA_OS_TEXT,
    },
    prelude::*,
};

/// Spawn the schematic scene + camera on map open, tear it down on close.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn manage_map_scene(
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
/// map + rings recenter on it); after that WASD is free to pan away. Every frame
/// keep the ring/hub anchor sitting on the current center.
pub(crate) fn map_focus_follow(
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
pub(crate) fn map_input(
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
        // Mouse drag orbits, RIGHT button ONLY. LMB is the contact-select click
        // (the blip `Button` widget), so letting it orbit turned a small
        // press-with-motion into a drag that slid the blip out from under the
        // cursor and ate the selection. Gentle sensitivity so a small drag is a
        // small turn.
        if mouse_buttons.pressed(MouseButton::Right) {
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
pub(crate) fn project_map_blips(
    mut commands: Commands,
    mut runtime: ResMut<MapRuntime>,
    ui_font: Option<Res<UiFont>>,
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
    let font = nova_os_font(ui_font.as_deref());
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

/// Blip square side in pixels (border box), and its border width.
pub(crate) const MAP_BLIP_PX: f32 = 12.0;
pub(crate) const MAP_BLIP_BORDER_PX: f32 = 2.0;

/// Where the label pill starts, measured from the dot's PADDING edge - which is
/// where an absolutely-positioned child's `left` is measured from, i.e. already
/// inside the dot's border. Offsetting by the border width lands the pill exactly
/// on the dot's outer right edge, so the two are one unbroken hit target.
pub(crate) const MAP_LABEL_LEFT_PX: f32 = MAP_BLIP_PX - MAP_BLIP_BORDER_PX;

pub(crate) fn spawn_blip(
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
                border: UiRect::all(Val::Px(MAP_BLIP_BORDER_PX)),
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
    // The label rides beside the blip as a child node, in a dark backing pill -
    // the same shape the ship app's section labels use, and for the same two
    // reasons: it reads clearly against the phosphor
    // scene, and it is a SOLID hit target rather than a box tight to the glyph
    // run. `Pointer<Click>` bubbles, so a click anywhere on the pill activates
    // the blip `Button` it is a child of.
    //
    // It starts at exactly the dot's right edge (see [`MAP_LABEL_LEFT_PX`]), so
    // dot and label are one unbroken target: the old `left: 16` left a 6 px dead
    // band between them that selected nothing. The 1 px vertical padding under
    // `top: -4` keeps the glyph baseline exactly where `top: -3` put it; the
    // glyphs shift 2 px left (18 -> 16 px from the dot's left edge), which is the
    // whole visual change.
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(MAP_LABEL_LEFT_PX),
            top: Val::Px(-4.0),
            padding: UiRect::axes(Val::Px(4.0), Val::Px(1.0)),
            border_radius: BorderRadius::all(Val::Px(3.0)),
            ..default()
        },
        BackgroundColor(NOVA_OS_SCREEN.with_alpha(0.82)),
        ChildOf(id),
        children![(
            // The blip carries its unique CODE (the `map goto <label>` handle),
            // not the freeform name, so the label you read is the label you type.
            Text::new(contact.code.clone()),
            nova_os_text_font(11.0, font),
            TextColor(color),
        )],
    ));
    commands.entity(viewport).add_child(id);
    id
}

/// Select a contact when its blip button is activated (click through the
/// forwarded NOVA OS pointer, or keyboard activation).
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

pub(crate) fn new_map_image(size: UVec2) -> Image {
    Image::new_target_texture(
        size.x.max(1),
        size.y.max(1),
        TextureFormat::Rgba8UnormSrgb,
        None,
    )
}

/// An unlit emissive-ish material so proxy meshes read at full color without a
/// light on the map layer.
pub(crate) fn unlit(color: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    }
}
