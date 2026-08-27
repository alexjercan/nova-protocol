//! The ship's 3D view: the block model built from the live section tree, its
//! camera and orbit controls, and the blips and outlines drawn over it.
//!
//! Blocks are placed in the ship's LOCAL space so the view is identical
//! wherever the ship is in the world.
//!
//! Touch this module when changing how the ship is drawn or selected in.

use std::collections::BTreeSet;

use bevy::{
    asset::RenderAssetUsages,
    camera::{visibility::RenderLayers, ImageRenderTarget, RenderTarget},
    mesh::PrimitiveTopology,
    prelude::*,
    render::render_resource::{Extent3d, TextureFormat},
    ui_widgets::{Activate, Button},
};
use nova_gameplay::prelude::*;
use nova_os::prelude::*;
use nova_ship::prelude::{derive_link_point_graph, PlacedSectionLinkPoints};
use nova_ui::font::UiFont;

use super::{sections::*, *};
use crate::terminal::{
    nova_os_font, nova_os_text_font, NovaOsAppInput, NOVA_OS_AMBER, NOVA_OS_PHOSPHOR,
    NOVA_OS_PHOSPHOR_MUTED, NOVA_OS_SCREEN, NOVA_OS_TEXT,
};

/// Dark fill behind the schematic image.
pub(crate) const SHIP_VIEW_BG: Color = Color::srgb_u8(0, 6, 3);

#[derive(Component)]
pub(crate) struct ShipViewportMarker;
/// The inspector-panel container.
#[derive(Component)]
pub(crate) struct ShipPanelMarker;
/// Which live text line of the panel a node is, so one system refreshes all three.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShipPanelField {
    Title,
    Detail,
    Note,
}
/// Which action a panel button raises, for its per-frame enabled styling.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShipPanelButton {
    Repair,
    Reload,
    Rebind,
}
#[derive(Component)]
pub(crate) struct ShipCameraMarker;
#[derive(Component)]
pub(crate) struct ShipSceneRoot;
/// Default-off structural edge overlay derived from live section sockets.
#[derive(Component)]
pub(crate) struct ShipMateOverlay;

/// A schematic block for one section: a dim, uniform-green translucent fill. It
/// no longer tracks status by colour (status rides the blip integrity bar); it
/// stays green so the schematic
/// reads as a labelled wireframe rather than one green blob.
#[derive(Component)]
pub(crate) struct ShipBlock {
    pub(crate) section: Entity,
}

/// The bright box outline riding on a [`ShipBlock`] (a `LineList` of the cuboid's
/// 12 edges) - the separation cue that keeps adjacent sections from merging. Its
/// material tints amber while its section is selected; the section identity lives
/// on the parent [`ShipBlock`], which the outline derives via [`ChildOf`].
#[derive(Component)]
pub(crate) struct ShipBlockOutline;

/// A projected, clickable section blip over the viewport: a status-coloured dot
/// plus a labelled marker. HP/ammo/status detail lives in the inspector panel,
/// so the blip carries no bar or pips.
#[derive(Component)]
pub(crate) struct ShipBlip {
    pub(crate) section: Entity,
}

/// The ship camera's orbit state (own spherical math, like `MapOrbit`, because
/// the shared smoothed orbit does not drive an RTT camera -
/// `verify-reused-driver-actually-moves`).
#[derive(Component, Clone, Copy)]
pub(crate) struct ShipOrbit {
    pub(crate) theta: f32,
    pub(crate) phi: f32,
    pub(crate) radius: f32,
    /// The eased orbit center the camera currently looks at and orbits around.
    pub(crate) center: Vec3,
    /// Where `center` is easing toward - the selected section's local position,
    /// or `center_home` after a `novaos_reframe`.
    pub(crate) center_target: Vec3,
    /// The whole-ship centroid; `novaos_reframe` re-frames the ship by
    /// retargeting here.
    pub(crate) center_home: Vec3,
    /// The selection `center_target` was last set for. The center only retargets
    /// when `ShipRuntime.selected` diverges from this, so the home reframe is
    /// not immediately chased back to the still-selected section.
    pub(crate) centered_on: Option<Entity>,
}

pub(crate) fn orbit_eye(radius: f32, theta: f32, phi: f32) -> Vec3 {
    let horizontal = radius * phi.cos();
    Vec3::new(
        horizontal * theta.sin(),
        radius * phi.sin(),
        horizontal * theta.cos(),
    )
}

/// Live state of the running ship app.
#[derive(Resource, Default)]
pub(crate) struct ShipRuntime {
    pub(crate) active: bool,
    pub(crate) image: Option<Handle<Image>>,
    pub(crate) scene_root: Option<Entity>,
    pub(crate) blips: bevy::platform::collections::HashMap<Entity, Entity>,
    /// The uniform dim-green block fill, shared by every block (status no longer
    /// recolours blocks - it rides the blip bar instead).
    pub(crate) mat_fill: Option<Handle<StandardMaterial>>,
    /// The bright phosphor box outline, and its amber variant for the selected
    /// section; swapped per block in `update_ship_blocks`.
    pub(crate) mat_outline: Option<Handle<StandardMaterial>>,
    pub(crate) mat_outline_selected: Option<Handle<StandardMaterial>>,
    pub(crate) selected: Option<Entity>,
    /// The codes last pushed as arg-completions, so the terminal is only marked
    /// changed when the set changes.
    pub(crate) completion_codes: Vec<String>,
    /// A transient note (e.g. an in-app action result) shown in the panel.
    pub(crate) note: Option<(String, f32)>,
    /// Whether Repair / Reload are valid for the current selection, cached by
    /// `update_ship_panel` so the button `Activate` observers can no-op on a
    /// disabled action without re-deriving the section's validity.
    pub(crate) panel_repair_enabled: bool,
    pub(crate) panel_reload_enabled: bool,
    pub(crate) panel_rebind_enabled: bool,
    /// Section waiting for a replacement keyboard or mouse binding.
    pub(crate) rebinding: Option<Entity>,
    /// Skips the key or click that armed the capture.
    pub(crate) rebind_just_armed: bool,
    /// Whether the structural mate overlay is visible.
    pub(crate) show_mates: bool,
}

/// Build the schematic block scene + camera on ship-app open, tear it down on
/// close.
#[expect(
    clippy::too_many_arguments,
    reason = "one system reading the panel, its camera and the ship it draws"
)]
pub(crate) fn manage_ship_scene(
    mut commands: Commands,
    pause: Res<State<PauseStates>>,
    terminal: Res<NovaOsTerminal>,
    mut runtime: ResMut<ShipRuntime>,
    images: Option<ResMut<Assets<Image>>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    sections: ShipSections,
) {
    let active = ship_is_active(&pause, &terminal);
    if active == runtime.active {
        return;
    }
    runtime.active = active;

    if !active {
        if let Some(root) = runtime.scene_root.take() {
            commands.entity(root).despawn();
        }
        for (_, blip) in runtime.blips.drain() {
            commands.entity(blip).try_despawn();
        }
        runtime.image = None;
        runtime.selected = None;
        runtime.note = None;
        runtime.rebinding = None;
        runtime.rebind_just_armed = false;
        runtime.show_mates = false;
        return;
    }

    let (Some(mut images), Some(mut meshes), Some(mut materials)) = (images, meshes, materials)
    else {
        return;
    };

    let views = sections.collect();
    runtime.show_mates = false;
    // Frame the whole ship: centroid of the section blocks + a radius that fits
    // the furthest one.
    let centroid = if views.is_empty() {
        Vec3::ZERO
    } else {
        views.iter().map(|v| v.local.translation).sum::<Vec3>() / views.len() as f32
    };
    let extent = views
        .iter()
        .map(|v| (v.local.translation - centroid).length() + v.half_extents.max_element())
        .fold(2.0_f32, f32::max);
    let radius = (extent * 2.6).clamp(SHIP_RADIUS_MIN, SHIP_RADIUS_MAX);

    let image = images.add(new_rtt_image(UVec2::splat(64)));
    runtime.image = Some(image.clone());

    // A dim, uniform-green fill for every block (status rides the blip bar now),
    // plus a bright edge outline and its amber selected variant. Shared handles,
    // so selecting a section only swaps its outline material, never respawns.
    runtime.mat_fill = Some(materials.add(unlit(NOVA_OS_PHOSPHOR.with_alpha(0.22))));
    runtime.mat_outline = Some(materials.add(unlit(NOVA_OS_PHOSPHOR.with_alpha(0.95))));
    runtime.mat_outline_selected = Some(materials.add(unlit(NOVA_OS_AMBER)));

    let scene_root = commands
        .spawn((
            ShipSceneRoot,
            Name::new("NovaOsShipScene"),
            Transform::default(),
            Visibility::Visible,
        ))
        .id();

    commands.spawn((
        ShipCameraMarker,
        Name::new("NovaOsShipCamera"),
        Camera3d::default(),
        Camera {
            order: SHIP_CAMERA_ORDER,
            clear_color: ClearColorConfig::Custom(NOVA_OS_SCREEN),
            is_active: true,
            ..default()
        },
        RenderTarget::Image(ImageRenderTarget {
            handle: image,
            scale_factor: 1.0,
        }),
        Transform::from_translation(
            centroid + orbit_eye(radius, SHIP_THETA_DEFAULT, SHIP_PHI_DEFAULT),
        )
        .looking_at(centroid, Vec3::Y),
        RenderLayers::layer(SHIP_LAYER),
        ShipOrbit {
            theta: SHIP_THETA_DEFAULT,
            phi: SHIP_PHI_DEFAULT,
            radius,
            center: centroid,
            center_target: centroid,
            center_home: centroid,
            // Treat the default selection as already centered at home, so the app
            // OPENS framed on the whole ship instead of chasing section 0 on frame 1.
            centered_on: views.first().map(|v| v.entity),
        },
        ChildOf(scene_root),
    ));

    // One proxy per section: a dim uniform-green fill shrunk to leave a GAP to its
    // neighbours, wrapped in a bright box OUTLINE at the full collider size. The
    // gap + outline are the separation cue that breaks up the "green blob".
    let fill_mat = runtime.mat_fill.clone().unwrap();
    let outline_mat = runtime.mat_outline.clone().unwrap();
    // One shared unit-cube edge mesh, scaled per block by the outline's transform.
    let edge_mesh = meshes.add(cuboid_edges());
    for view in &views {
        let full = (view.half_extents * 2.0).max(Vec3::splat(0.2));
        let fill = full * SHIP_BLOCK_FILL_SCALE;
        let mesh = meshes.add(Cuboid::new(fill.x, fill.y, fill.z));
        commands
            .spawn((
                ShipBlock {
                    section: view.entity,
                },
                Mesh3d(mesh),
                MeshMaterial3d(fill_mat.clone()),
                Transform {
                    translation: view.local.translation,
                    rotation: view.local.rotation,
                    scale: Vec3::ONE,
                },
                RenderLayers::layer(SHIP_LAYER),
                ChildOf(scene_root),
            ))
            .with_children(|block| {
                block.spawn((
                    ShipBlockOutline,
                    Mesh3d(edge_mesh.clone()),
                    MeshMaterial3d(outline_mat.clone()),
                    // Unit edges scaled to the FULL collider size (the fill inside
                    // is smaller, so the outline frames it with a visible gap).
                    Transform::from_scale(full),
                    RenderLayers::layer(SHIP_LAYER),
                ));
            });
    }

    if let Some(mesh) = mate_edges_mesh(&views) {
        commands.spawn((
            ShipMateOverlay,
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(unlit(NOVA_OS_AMBER))),
            Transform::default(),
            Visibility::Hidden,
            RenderLayers::layer(SHIP_LAYER),
            ChildOf(scene_root),
        ));
    }

    runtime.scene_root = Some(scene_root);
    // Default the selection to the first section so the panel is useful at once.
    runtime.selected = views.first().map(|v| v.entity);
}

/// Keep the offscreen image sized 1:1 to the viewport node and patch the image
/// handle onto the viewport.
pub(crate) fn reconcile_ship_target(
    runtime: Res<ShipRuntime>,
    mut images: Option<ResMut<Assets<Image>>>,
    mut q_viewport: Query<(&ComputedNode, &mut ImageNode), With<ShipViewportMarker>>,
    mut q_camera: Query<(&mut Camera, &mut Projection), With<ShipCameraMarker>>,
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
        if let Ok((_, mut projection)) = q_camera.single_mut() {
            projection.set_changed();
        }
    }
}

/// Drive the ship camera transform from its orbit state.
pub(crate) fn drive_ship_camera(
    time: Res<Time>,
    mut q_camera: Query<(&mut Transform, &mut ShipOrbit), With<ShipCameraMarker>>,
) {
    let Ok((mut transform, mut orbit)) = q_camera.single_mut() else {
        return;
    };
    // Frame-rate-independent exponential ease of the center toward its target, so
    // Keyed orbit + drag around the section being inspected.
    let dt = time.delta_secs().max(1.0 / 240.0);
    let alpha = 1.0 - (-SHIP_CENTER_EASE * dt).exp();
    orbit.center = orbit.center.lerp(orbit.center_target, alpha);
    let eye = orbit.center + orbit_eye(orbit.radius, orbit.theta, orbit.phi);
    *transform = Transform::from_translation(eye).looking_at(orbit.center, Vec3::Y);
}

/// Read the keyboard/mouse while the ship app owns the screen: orbit, cycle the
/// selection, and raise reload/repair on the selected section.
pub(crate) fn ship_input(
    mut input: NovaOsAppInput,
    mut runtime: ResMut<ShipRuntime>,
    sections: ShipSections,
    mut commands: MessageWriter<ShipSectionCommand>,
    mut q_camera: Query<&mut ShipOrbit, With<ShipCameraMarker>>,
    mut q_mates: Query<&mut Visibility, With<ShipMateOverlay>>,
) {
    if !input.app_is_active(SHIP_APP_ID) {
        return;
    }
    let motion_delta = input.motion_delta();
    let wheel_delta = input.wheel_delta();
    let dt = input.dt();

    if let Some((_, remaining)) = runtime.note.as_mut() {
        *remaining -= dt;
        if *remaining <= 0.0 {
            runtime.note = None;
        }
    }
    if runtime.rebinding.is_some() {
        return;
    }

    if let Ok(mut orbit) = q_camera.single_mut() {
        let turn = 1.6 * dt;
        if input.pressed("novaos_orbit_left") {
            orbit.theta += turn;
        }
        if input.pressed("novaos_orbit_right") {
            orbit.theta -= turn;
        }
        if input.pressed("novaos_orbit_up") {
            orbit.phi = (orbit.phi + turn).min(1.45);
        }
        if input.pressed("novaos_orbit_down") {
            orbit.phi = (orbit.phi - turn).max(0.12);
        }
        // Mouse drag orbits, RIGHT button ONLY. LMB is the blip-select click (the
        // `Button` widget), so letting it orbit turned a small press-with-motion
        // into a drag that slid the blip out from under the cursor and ate the
        // selection.
        if input.mouse_pressed(MouseButton::Right) {
            orbit.theta -= motion_delta.x * 0.0024;
            orbit.phi = (orbit.phi + motion_delta.y * 0.0024).clamp(0.12, 1.45);
        }
        if wheel_delta != 0.0 {
            orbit.radius =
                (orbit.radius * (1.0 - wheel_delta * 0.12)).clamp(SHIP_RADIUS_MIN, SHIP_RADIUS_MAX);
        }
    }

    if input.just_pressed("ship_mates") {
        runtime.show_mates = !runtime.show_mates;
        for mut visibility in &mut q_mates {
            *visibility = if runtime.show_mates {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }

    let list = sections.collect();
    if list.is_empty() {
        return;
    }
    // Cycle the selection.
    let forward = input.just_pressed("novaos_next");
    let backward = input.just_pressed("novaos_prev");
    if forward || backward {
        let current = runtime
            .selected
            .and_then(|sel| list.iter().position(|v| v.entity == sel));
        let len = list.len();
        let next = match current {
            Some(i) if forward => (i + 1) % len,
            Some(i) => (i + len - 1) % len,
            None => 0,
        };
        runtime.selected = Some(list[next].entity);
    }

    // Reconcile the orbit center after any selection change this frame. This is
    // the single funnel for the cycle actions, blip clicks, and the default
    // selection: each caller only sets `runtime.selected`, and the center
    // chases it here.
    if let Ok(mut orbit) = q_camera.single_mut() {
        if input.just_pressed("novaos_reframe") {
            // Reset re-frames the whole ship: restore the default angles and
            // retarget the center home. Consuming the current selection
            // (centered_on = selected) makes the reframe STICK - the change check
            // below will not chase the still-selected section back (DECISION.md).
            orbit.theta = SHIP_THETA_DEFAULT;
            orbit.phi = SHIP_PHI_DEFAULT;
            orbit.center_target = orbit.center_home;
            orbit.centered_on = runtime.selected;
        } else if runtime.selected != orbit.centered_on {
            // Selection changed: ease the center onto the newly selected section's
            // local position (same scene frame as the blips, so no reprojection).
            if let Some(view) = runtime
                .selected
                .and_then(|sel| list.iter().find(|v| v.entity == sel))
            {
                orbit.center_target = view.local.translation;
            }
            orbit.centered_on = runtime.selected;
        }
    }

    // What the app does to the section it has selected. Route mutation actions
    // through their shared seams.
    if let Some(sel) = runtime.selected {
        if input.just_pressed("ship_rebind")
            && list
                .iter()
                .find(|view| view.entity == sel)
                .is_some_and(|view| view.bindings.is_some())
        {
            runtime.rebinding = Some(sel);
            runtime.rebind_just_armed = true;
            runtime.note = None;
            return;
        }
        if input.just_pressed("ship_reload") {
            commands.write(ShipSectionCommand {
                target: sel,
                action: ShipAction::Reload,
            });
        }
        if input.just_pressed("ship_repair") {
            commands.write(ShipSectionCommand {
                target: sel,
                action: ShipAction::Repair,
            });
        }
    }
}

/// Tint the SELECTED section's box outline amber and leave every other block's
/// outline phosphor. Blocks must not recolour by status - the fill stays a
/// uniform green and status rides the blip integrity bar instead.
pub(crate) fn update_ship_blocks(
    runtime: Res<ShipRuntime>,
    mut q_outline: Query<(&ChildOf, &mut MeshMaterial3d<StandardMaterial>), With<ShipBlockOutline>>,
    q_block: Query<&ShipBlock>,
) {
    if !runtime.active {
        return;
    }
    let (Some(base), Some(selected)) = (&runtime.mat_outline, &runtime.mat_outline_selected) else {
        return;
    };
    for (parent, mut material) in &mut q_outline {
        // The section identity lives on the parent block, not the outline.
        let Ok(block) = q_block.get(parent.0) else {
            continue;
        };
        let want = if runtime.selected == Some(block.section) {
            selected
        } else {
            base
        };
        if material.0 != *want {
            material.0 = want.clone();
        }
    }
}

/// Project each section through the ship camera into the viewport and keep a
/// clickable, labelled UI blip per section in sync.
#[expect(
    clippy::too_many_arguments,
    reason = "one system reading the panel, its camera and the ship it draws"
)]
pub(crate) fn project_ship_blips(
    mut commands: Commands,
    mut runtime: ResMut<ShipRuntime>,
    ui_font: Option<Res<UiFont>>,
    sections: ShipSections,
    q_camera: Query<(&Camera, &GlobalTransform), With<ShipCameraMarker>>,
    q_viewport: Query<(Entity, &ComputedNode), With<ShipViewportMarker>>,
    q_blip: Query<&ShipBlip>,
    // Disjoint component queries so the dot's node / visibility / border /
    // background can each be updated by entity id without conflicting `&mut`.
    mut q_node: Query<&mut Node>,
    mut q_vis: Query<&mut Visibility>,
    mut q_border: Query<&mut BorderColor>,
    mut q_bg: Query<&mut BackgroundColor>,
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
    let list = sections.collect();
    let font = nova_os_font(ui_font.as_deref());

    let mut seen = bevy::platform::collections::HashSet::new();
    for view in &list {
        seen.insert(view.entity);
        // Project the section's SCENE-space position (its local offset - the
        // scene root is anchored at the origin, so blocks and blips share this
        // frame). Projecting the world position would only line up when the ship
        // sits at the world origin, so blips would drift off the blocks in flight.
        let projected = camera
            .world_to_viewport(cam_gt, view.local.translation)
            .ok()
            .map(|p| p * to_logical)
            .filter(|p| p.x >= 0.0 && p.y >= 0.0 && p.x <= size.x && p.y <= size.y);
        let selected = runtime.selected == Some(view.entity);
        let color = view.status_color();

        let blip = if let Some(&blip) = runtime.blips.get(&view.entity) {
            blip
        } else {
            let id = spawn_ship_blip(&mut commands, viewport, view, font.clone());
            runtime.blips.insert(view.entity, id);
            id
        };
        // A just-spawned blip is not queryable until the command flush, so its
        // updates simply wait a frame (the guards skip it).
        if q_blip.get(blip).is_err() {
            continue;
        }

        // The dot: position from the projection, fill coloured by status (so a
        // damaged section is spottable at a glance), amber border while selected.
        if let Ok(mut node) = q_node.get_mut(blip) {
            if let Some(p) = projected {
                node.left = Val::Px(p.x - SHIP_BLIP_PX * 0.5);
                node.top = Val::Px(p.y - SHIP_BLIP_PX * 0.5);
            }
        }
        if let Ok(mut vis) = q_vis.get_mut(blip) {
            *vis = match projected {
                Some(_) => Visibility::Inherited,
                None => Visibility::Hidden,
            };
        }
        if let Ok(mut bg) = q_bg.get_mut(blip) {
            bg.0 = color;
        }
        if let Ok(mut border) = q_border.get_mut(blip) {
            *border = if selected {
                BorderColor::all(NOVA_OS_AMBER)
            } else {
                // The same amber the spawn uses, just transparent (the border only
                // reads when selected).
                BorderColor::all(NOVA_OS_AMBER.with_alpha(0.0))
            };
        }
    }

    let stale: Vec<Entity> = runtime
        .blips
        .keys()
        .copied()
        .filter(|s| !seen.contains(s))
        .collect();
    for section in stale {
        if let Some(blip) = runtime.blips.remove(&section) {
            commands.entity(blip).try_despawn();
        }
    }
}

pub(crate) const SHIP_BLIP_PX: f32 = 12.0;
pub(crate) const SHIP_BLIP_BORDER_PX: f32 = 2.0;

/// Where the label pill starts, measured from the dot's PADDING edge - which is
/// where an absolutely-positioned child's `left` is measured from, i.e. already
/// inside the dot's border. Offsetting by the border width lands the pill exactly
/// on the dot's outer right edge, so dot and label are one unbroken hit target
/// (NOTE: `left: 14` leaves a 4 px dead band here).
pub(crate) const SHIP_LABEL_LEFT_PX: f32 = SHIP_BLIP_PX - SHIP_BLIP_BORDER_PX;
/// Fraction of the collider a block's green body fills; the remainder is the gap
/// to its neighbours that the bright outline frames.
pub(crate) const SHIP_BLOCK_FILL_SCALE: f32 = 0.86;

pub(crate) fn spawn_ship_blip(
    commands: &mut Commands,
    viewport: Entity,
    view: &ShipSectionView,
    font: Handle<Font>,
) -> Entity {
    let color = view.status_color();
    // The clickable dot: fill coloured by status (recoloured each frame in
    // `project_ship_blips`); its amber border marks the selection.
    let dot = commands
        .spawn((
            Button,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(SHIP_BLIP_PX),
                height: Val::Px(SHIP_BLIP_PX),
                border: UiRect::all(Val::Px(SHIP_BLIP_BORDER_PX)),
                border_radius: BorderRadius::MAX,
                ..default()
            },
            BorderColor::all(NOVA_OS_AMBER.with_alpha(0.0)),
            BackgroundColor(color),
        ))
        // Selection through `Activate` (fires for the forwarded NOVA OS pointer),
        // not `Interaction` polling (`rtt-ui-select-via-activate-not-interaction`).
        .observe(on_ship_blip_click)
        .id();

    // Label: the per-kind glyph + section code, in a dark backing pill so it reads
    // clearly against the phosphor scene instead of tiny green-on-green. Detail
    // (HP/ammo/status) lives in the inspector panel now, not on the blip.
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(SHIP_LABEL_LEFT_PX),
                top: Val::Px(-4.0),
                padding: UiRect::axes(Val::Px(4.0), Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(3.0)),
                ..default()
            },
            BackgroundColor(NOVA_OS_SCREEN.with_alpha(0.82)),
            ChildOf(dot),
        ))
        .with_children(|pill| {
            pill.spawn((
                Text::new(format!("{} {}", kind_glyph(view.kind), view.code)),
                nova_os_text_font(11.0, font),
                TextColor(NOVA_OS_TEXT),
            ));
        });

    commands.entity(dot).insert(ShipBlip {
        section: view.entity,
    });
    commands.entity(viewport).add_child(dot);
    dot
}

pub(crate) fn on_ship_blip_click(
    activate: On<Activate>,
    q_blip: Query<&ShipBlip>,
    mut runtime: ResMut<ShipRuntime>,
) {
    if let Ok(blip) = q_blip.get(activate.entity) {
        runtime.selected = Some(blip.section);
    }
}

/// Raise a Repair on the selected section when the panel button is clicked, unless
/// the panel marked repair disabled for it. Same seam as the `P` key and the CLI.
pub(crate) fn on_ship_repair_button(
    _activate: On<Activate>,
    runtime: Res<ShipRuntime>,
    mut commands: MessageWriter<ShipSectionCommand>,
) {
    if !runtime.panel_repair_enabled {
        return;
    }
    if let Some(target) = runtime.selected {
        commands.write(ShipSectionCommand {
            target,
            action: ShipAction::Repair,
        });
    }
}

/// Raise a Reload on the selected section when the panel button is clicked, unless
/// the panel marked reload disabled for it.
pub(crate) fn on_ship_rebind_button(_activate: On<Activate>, mut runtime: ResMut<ShipRuntime>) {
    if !runtime.panel_rebind_enabled {
        return;
    }
    runtime.rebinding = runtime.selected;
    runtime.rebind_just_armed = runtime.rebinding.is_some();
    runtime.note = None;
}

pub(crate) fn on_ship_reload_button(
    _activate: On<Activate>,
    runtime: Res<ShipRuntime>,
    mut commands: MessageWriter<ShipSectionCommand>,
) {
    if !runtime.panel_reload_enabled {
        return;
    }
    if let Some(target) = runtime.selected {
        commands.write(ShipSectionCommand {
            target,
            action: ShipAction::Reload,
        });
    }
}

/// Refresh the inspector panel from the current selection: title, live detail,
/// button enabled-state, and the note line (a transient action result, or the
/// reason a button is disabled). Caches the enabled flags for the observers.
pub(crate) fn update_ship_panel(
    mut runtime: ResMut<ShipRuntime>,
    sections: ShipSections,
    mut q_text: Query<(&ShipPanelField, &mut Text, &mut TextColor)>,
    mut q_button: Query<(&ShipPanelButton, &mut BorderColor, &mut BackgroundColor)>,
) {
    if !runtime.active {
        return;
    }
    let selected = runtime
        .selected
        .and_then(|sel| sections.collect().into_iter().find(|v| v.entity == sel));

    let (title, detail, detail_color, actions) = match &selected {
        Some(view) => (
            format!("{}  {}", view.code, view.name),
            panel_detail_text(view),
            if view.status() == "nominal" {
                NOVA_OS_TEXT
            } else {
                NOVA_OS_AMBER
            },
            panel_action_state(view),
        ),
        None => (
            "INSPECTOR".to_string(),
            "Select a section:\nclick a block or press [ / ].".to_string(),
            NOVA_OS_PHOSPHOR_MUTED,
            PanelActions::none(),
        ),
    };

    runtime.panel_repair_enabled = actions.repair_enabled;
    runtime.panel_reload_enabled = actions.reload_enabled;
    runtime.panel_rebind_enabled = selected
        .as_ref()
        .is_some_and(|view| view.bindings.is_some());

    // Note line: a transient action result wins; else the disabled reason; else a
    // key hint when a section is selected.
    let (note, note_color) = if runtime.rebinding.is_some() {
        (
            "PRESS A KEY OR MOUSE BUTTON - ESC CANCELS".to_string(),
            NOVA_OS_AMBER,
        )
    } else if let Some((note, _)) = &runtime.note {
        (note.clone(), NOVA_OS_AMBER)
    } else if let Some(reason) = &actions.reason {
        (reason.clone(), NOVA_OS_PHOSPHOR_MUTED)
    } else if selected.is_some() {
        (
            "P repair   L reload   B rebind".to_string(),
            NOVA_OS_PHOSPHOR_MUTED,
        )
    } else {
        (String::new(), NOVA_OS_PHOSPHOR_MUTED)
    };

    for (field, mut text, mut color) in &mut q_text {
        let (value, tint) = match field {
            ShipPanelField::Title => (&title, NOVA_OS_PHOSPHOR),
            ShipPanelField::Detail => (&detail, detail_color),
            ShipPanelField::Note => (&note, note_color),
        };
        if text.0 != *value {
            text.0 = value.clone();
        }
        color.set_if_neq(TextColor(tint));
    }

    for (button, mut border, mut background) in &mut q_button {
        let enabled = match button {
            ShipPanelButton::Repair => actions.repair_enabled,
            ShipPanelButton::Reload => actions.reload_enabled,
            ShipPanelButton::Rebind => runtime.panel_rebind_enabled,
        };
        let (border_color, background_color) = if enabled {
            (NOVA_OS_PHOSPHOR, NOVA_OS_PHOSPHOR.with_alpha(0.14))
        } else {
            (NOVA_OS_PHOSPHOR_MUTED.with_alpha(0.35), NOVA_OS_SCREEN)
        };
        // Runs every frame the app owns the screen, so both writes are guarded
        // like the `Text` write above: an unguarded colour write marks the node
        // changed and re-uploads its UI batch each frame.
        border.set_if_neq(BorderColor::all(border_color));
        background.set_if_neq(BackgroundColor(background_color));
    }
}

pub(crate) fn mate_edges_mesh(views: &[ShipSectionView]) -> Option<Mesh> {
    let placed: Vec<_> = views
        .iter()
        .map(|view| PlacedSectionLinkPoints {
            position: view.local.translation,
            rotation: view.local.rotation,
            link_points: &view.link_points,
        })
        .collect();
    let mates = derive_link_point_graph(&placed).ok()?;
    let edges: BTreeSet<_> = mates
        .into_iter()
        .map(|mate| {
            let a = mate.a.section_index;
            let b = mate.b.section_index;
            (a.min(b), a.max(b))
        })
        .collect();
    if edges.is_empty() {
        return None;
    }
    let positions: Vec<[f32; 3]> = edges
        .into_iter()
        .flat_map(|(a, b)| {
            [
                views[a].local.translation.to_array(),
                views[b].local.translation.to_array(),
            ]
        })
        .collect();
    let count = positions.len();
    Some(
        Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::default())
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; count])
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; count]),
    )
}

/// A `LineList` mesh of a unit cuboid's 12 edges (corners at +/-0.5, no face
/// diagonals), so a block can carry a crisp box outline instead of merging into
/// its neighbours. NORMAL and UV attributes are filled with placeholders: the
/// outline material is `unlit` and ignores them, but the mesh pipeline still
/// binds those vertex attributes.
pub(crate) fn cuboid_edges() -> Mesh {
    let c = 0.5;
    let corners = [
        Vec3::new(-c, -c, -c),
        Vec3::new(c, -c, -c),
        Vec3::new(c, c, -c),
        Vec3::new(-c, c, -c),
        Vec3::new(-c, -c, c),
        Vec3::new(c, -c, c),
        Vec3::new(c, c, c),
        Vec3::new(-c, c, c),
    ];
    // Four edges per end face + four connectors = the 12 cuboid edges.
    let edges = [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 0),
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 4),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ];
    let positions: Vec<[f32; 3]> = edges
        .iter()
        .flat_map(|&(a, b)| [corners[a].to_array(), corners[b].to_array()])
        .collect();
    let count = positions.len();
    Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; count])
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; count])
}

pub(crate) fn new_rtt_image(size: UVec2) -> Image {
    Image::new_target_texture(
        size.x.max(1),
        size.y.max(1),
        TextureFormat::Rgba8UnormSrgb,
        None,
    )
}

pub(crate) fn unlit(color: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    }
}
