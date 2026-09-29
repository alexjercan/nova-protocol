//! The ship's 3D view: the block model built from the live section tree, its
//! camera and orbit controls, and the badges and outlines drawn over it; plus
//! the observers of the view and panel buttons and the per-frame panel
//! refresh.
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
    ui::InteractionDisabled,
    ui_widgets::{Activate, Button},
};
use nova_gameplay::prelude::*;
use nova_ship::prelude::{derive_link_point_graph, PlacedSectionLinkPoints};
use nova_ui::{
    theme::{ActiveUiTheme, UiColor},
    widget::{ThemedBorder, ThemedFill, ThemedImageTint, ThemedText},
};

use super::{sections::*, *};
use crate::{
    icons::{icon_node, InterfaceIcons, SectionIconType},
    pane::{interface_shown, play_menu_select, themed_label, InterfacePaneType},
    terminal::{NovaOsAppInput, NovaOsCloseTransition},
    viewer::{cycle_index, orbit_eye, unlit, zoom_radius, OrbitGesture},
};

#[derive(Component)]
pub(crate) struct ShipViewportMarker;
/// The section-panel container.
#[derive(Component)]
pub(crate) struct ShipPanelMarker;
/// Which live text line of the panel or the footer a node is, so one system
/// refreshes them all.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShipPanelField {
    Title,
    Status,
    Detail,
    Note,
    /// The selection summary on the right of the pane footer.
    Summary,
}
/// The selected section's family icon in the panel head.
#[derive(Component)]
pub(crate) struct ShipPreviewIcon;
/// The panel's condition bar fill, as wide as the selected section's
/// integrity.
#[derive(Component)]
pub(crate) struct ShipConditionFill;
/// The status dot in a section badge's corner.
#[derive(Component)]
pub(crate) struct ShipStatusPip;
/// Which action a panel button raises, so `update_ship_panel` can disable it.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShipPanelButton {
    Repair,
    Rebind,
}
#[derive(Component)]
pub(crate) struct ShipCameraMarker;
#[derive(Component)]
pub(crate) struct ShipSceneRoot;
/// Default-off structural edge overlay derived from live section sockets.
#[derive(Component)]
pub(crate) struct ShipMateOverlay;

/// A schematic block for one section, filled in its family's tint so the
/// weapons, thrusters and controller read apart from the hull.
#[derive(Component)]
pub(crate) struct ShipBlock {
    pub(crate) section: Entity,
}

/// The bright box outline riding on a [`ShipBlock`] (a `LineList` of the cuboid's
/// 12 edges) - the separation cue that keeps adjacent sections from merging. Its
/// material switches to the accent while its section is selected; the section identity lives
/// on the parent [`ShipBlock`], which the outline derives via [`ChildOf`].
#[derive(Component)]
pub(crate) struct ShipBlockOutline;

/// A projected, clickable section badge over the viewport: the family icon on
/// a dark tile, a status pip in its corner and a code label. HP and ammo live
/// in the section panel.
#[derive(Component)]
pub(crate) struct ShipBlip {
    pub(crate) section: Entity,
}

/// A badge's code label. Shown only for the selected section: a carrier has
/// two thousand of these, and two thousand labels is a wall, not a label layer.
#[derive(Component)]
pub(crate) struct ShipBlipLabel;

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
    /// or `center_home` after a `viewer_reframe`.
    pub(crate) center_target: Vec3,
    /// The whole-ship centroid; `viewer_reframe` re-frames the ship by
    /// retargeting here.
    pub(crate) center_home: Vec3,
    /// The selection `center_target` was last set for. The center only retargets
    /// when `ShipRuntime.selected` diverges from this, so the home reframe is
    /// not immediately chased back to the still-selected section.
    pub(crate) centered_on: Option<Entity>,
}

/// Live state of the Ship pane.
#[derive(Resource, Default)]
pub struct ShipRuntime {
    pub(crate) active: bool,
    pub(crate) image: Option<Handle<Image>>,
    pub(crate) scene_root: Option<Entity>,
    pub(crate) blips: bevy::platform::collections::HashMap<Entity, Entity>,
    /// The box outline, and its accent variant for the selected section;
    /// swapped per block in `update_ship_blocks`.
    pub(crate) mat_outline: Option<Handle<StandardMaterial>>,
    pub(crate) mat_outline_selected: Option<Handle<StandardMaterial>>,
    pub(crate) selected: Option<Entity>,
    /// A transient note (e.g. an action result) shown in the panel.
    pub(crate) note: Option<(String, f32)>,
    /// Whether Repair is valid for the current selection, cached by
    /// `update_ship_panel` so the button `Activate` observer can no-op on a
    /// disabled action without re-deriving the section's validity.
    pub(crate) panel_repair_enabled: bool,
    pub(crate) panel_rebind_enabled: bool,
    /// Section waiting for a replacement keyboard or mouse binding.
    pub(crate) rebinding: Option<Entity>,
    /// Holds the capture until every button is up, so the key or click that
    /// ARMED it is not the one captured.
    pub(crate) rebind_awaiting_release: bool,
    /// Whether the structural mate overlay is visible.
    pub(crate) show_mates: bool,
}

impl ShipRuntime {
    /// Whether a section rebind capture is waiting for its key. While it is,
    /// the next key belongs to the capture, so global keys such as `:` must
    /// refuse it.
    pub fn rebind_armed(&self) -> bool {
        self.rebinding.is_some()
    }
}

/// Build the schematic block scene + camera when the Ship pane opens, tear it down on
/// close.
#[expect(
    clippy::too_many_arguments,
    reason = "one system reading the panel, its camera and the ship it draws"
)]
pub(crate) fn manage_ship_scene(
    mut commands: Commands,
    pause: Res<State<PauseStates>>,
    close: Res<NovaOsCloseTransition>,
    pane: Res<InterfacePaneType>,
    theme: Res<ActiveUiTheme>,
    mut runtime: ResMut<ShipRuntime>,
    images: Option<ResMut<Assets<Image>>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    sections: ShipSections,
) {
    // Shown, not active: the scene, camera and selection survive the command
    // modal over the pane. Input gates on the pane owning the screen instead.
    let active = interface_shown(&pause, &close) && *pane == InterfacePaneType::Ship;
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
        runtime.rebind_awaiting_release = false;
        runtime.show_mates = false;
        return;
    }

    let (Some(mut images), Some(mut meshes), Some(mut materials)) = (images, meshes, materials)
    else {
        return;
    };

    let views = sections.collect();
    runtime.show_mates = false;
    let (centroid, radius) = ship_framing(&views);

    let image = images.add(new_render_target_image(UVec2::splat(64)));
    runtime.image = Some(image.clone());

    // One fill per section family, plus the edge outline and its selected
    // variant. Shared handles, so selecting a section only swaps its outline
    // material, never respawns. Colours come from the theme when the pane
    // opens; a theme change shows on the next open.
    let block_mats = SectionIconType::ALL
        .map(|icon| materials.add(unlit(theme.color_alpha(icon.color(), icon.block_alpha()))));
    runtime.mat_outline = Some(materials.add(unlit(theme.color_alpha(UiColor::Primary, 0.95))));
    runtime.mat_outline_selected = Some(materials.add(unlit(theme.color(UiColor::Accent))));

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
            clear_color: ClearColorConfig::Custom(theme.color(UiColor::Surface)),
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

    // One proxy per section: a family-tinted fill shrunk to leave a GAP to its
    // neighbours, wrapped in a box OUTLINE at the full collider size. The gap
    // and outline keep adjacent sections from merging.
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
                MeshMaterial3d(block_mats[SectionIconType::of(view.kind) as usize].clone()),
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
            MeshMaterial3d(materials.add(unlit(theme.color(UiColor::Accent)))),
            Transform::default(),
            Visibility::Hidden,
            RenderLayers::layer(SHIP_LAYER),
            ChildOf(scene_root),
        ));
    }

    // Bow is ship-local -Z. The arrow starts clear of the foremost block and
    // points away from the hull, so the view reads fore and aft.
    let bow = views
        .iter()
        .map(|view| view.local.translation.z - view.half_extents.max_element())
        .fold(f32::INFINITY, f32::min);
    if bow.is_finite() {
        let length = radius * 0.2;
        let gap = length * 0.15;
        let shaft = length * 0.03;
        let point_back = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
        let bow_mat = materials.add(unlit(theme.color_alpha(UiColor::Accent, 0.9)));
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(shaft, length * 0.7))),
            MeshMaterial3d(bow_mat.clone()),
            Transform::from_translation(Vec3::new(
                centroid.x,
                centroid.y,
                bow - gap - length * 0.35,
            ))
            .with_rotation(point_back),
            RenderLayers::layer(SHIP_LAYER),
            ChildOf(scene_root),
        ));
        commands.spawn((
            Mesh3d(meshes.add(Cone {
                radius: shaft * 3.5,
                height: length * 0.3,
            })),
            MeshMaterial3d(bow_mat),
            Transform::from_translation(Vec3::new(
                centroid.x,
                centroid.y,
                bow - gap - length * 0.85,
            ))
            .with_rotation(point_back),
            RenderLayers::layer(SHIP_LAYER),
            ChildOf(scene_root),
        ));
    }

    runtime.scene_root = Some(scene_root);
    // Default the selection to the first section so the panel is useful at once.
    runtime.selected = views.first().map(|v| v.entity);
}

/// The whole-ship framing in ship-local space: the centroid of the sections
/// and the orbit radius that fits the furthest one.
pub fn ship_framing(views: &[ShipSectionView]) -> (Vec3, f32) {
    let centroid = if views.is_empty() {
        Vec3::ZERO
    } else {
        views.iter().map(|v| v.local.translation).sum::<Vec3>() / views.len() as f32
    };
    let extent = views
        .iter()
        .map(|v| (v.local.translation - centroid).length() + v.half_extents.max_element())
        .fold(2.0_f32, f32::max);
    (
        centroid,
        (extent * 2.6).clamp(SHIP_RADIUS_MIN, SHIP_RADIUS_MAX),
    )
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

/// Drive the ship camera transform from its orbit state.
pub(crate) fn drive_ship_camera(
    time: Res<Time<Real>>,
    mut q_camera: Query<(&mut Transform, &mut ShipOrbit), With<ShipCameraMarker>>,
) {
    let Ok((mut transform, mut orbit)) = q_camera.single_mut() else {
        return;
    };
    orbit.center = ease_orbit_center(orbit.center, orbit.center_target, time.delta_secs());
    let eye = orbit.center + orbit_eye(orbit.radius, orbit.theta, orbit.phi);
    *transform = Transform::from_translation(eye).looking_at(orbit.center, Vec3::Y);
}

/// One frame of the orbit center chasing `target` over `dt` seconds.
///
/// A frame-rate independent exponential ease (`1 - exp(-k * dt)`), so a
/// selection glides the view onto the section instead of jumping to it.
pub fn ease_orbit_center(center: Vec3, target: Vec3, dt: f32) -> Vec3 {
    let alpha = 1.0 - (-SHIP_CENTER_EASE * dt).exp();
    center.lerp(target, alpha)
}

/// Read the keyboard/mouse while the Ship pane is showing: orbit, cycle the
/// selection, and raise repair on the selected section.
pub(crate) fn ship_input(
    mut input: NovaOsAppInput,
    mut runtime: ResMut<ShipRuntime>,
    sections: ShipSections,
    mut commands: MessageWriter<SectionRepairCommand>,
    mut q_camera: Query<&mut ShipOrbit, With<ShipCameraMarker>>,
    mut q_mates: Query<&mut Visibility, With<ShipMateOverlay>>,
) {
    if !input.app_is_active(InterfacePaneType::Ship) {
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
        // Turn, tilt and RMB-drag are the shared viewer's feel, not the ship's.
        let gesture = OrbitGesture::read(&input, motion_delta);
        if !gesture.is_idle() {
            let (theta, phi) = gesture.apply(dt, orbit.theta, orbit.phi);
            orbit.theta = theta;
            orbit.phi = phi;
        }
        // The zoom clamp is the ship's own: a hull has a fixed reach, where the
        // map's ceiling tracks the live contact spread.
        if wheel_delta != 0.0 {
            orbit.radius = zoom_radius(orbit.radius, wheel_delta, SHIP_RADIUS_MIN, SHIP_RADIUS_MAX);
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
    let forward = input.just_pressed("viewer_next");
    let backward = input.just_pressed("viewer_prev");
    if forward || backward {
        let current = runtime
            .selected
            .and_then(|sel| list.iter().position(|v| v.entity == sel));
        if let Some(next) = cycle_index(current, list.len(), forward) {
            runtime.selected = Some(list[next].entity);
        }
    }

    // Reconcile the orbit center after any selection change this frame. This is
    // the single funnel for the cycle actions, blip clicks, and the default
    // selection: each caller only sets `runtime.selected`, and the center
    // chases it here.
    if let Ok(mut orbit) = q_camera.single_mut() {
        if input.just_pressed("viewer_reframe") {
            reset_ship_orbit(&mut orbit, ship_framing(&list).1, runtime.selected);
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
            runtime.rebind_awaiting_release = true;
            runtime.note = None;
            return;
        }
        if input.just_pressed("ship_repair") {
            commands.write(SectionRepairCommand { target: sel });
        }
    }
}

/// Give the SELECTED section's box outline the accent material and every other
/// block's outline the base one. Blocks keep their family fill; status rides
/// the badge pip and the panel condition bar.
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
/// clickable badge per section in sync. Writes only what changed: a carrier
/// has two thousand badges.
#[expect(
    clippy::too_many_arguments,
    reason = "one system reading the panel, its camera and the ship it draws"
)]
pub(crate) fn project_ship_blips(
    mut commands: Commands,
    mut runtime: ResMut<ShipRuntime>,
    icons: Res<InterfaceIcons>,
    sections: ShipSections,
    q_camera: Query<(&Camera, &GlobalTransform), With<ShipCameraMarker>>,
    q_viewport: Query<(Entity, &ComputedNode), With<ShipViewportMarker>>,
    mut q_blip: Query<(&mut Node, &mut Visibility, &mut ThemedBorder, &Children), With<ShipBlip>>,
    mut q_pip: Query<&mut ThemedFill, With<ShipStatusPip>>,
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
    let list = sections.collect();

    let mut seen = bevy::platform::collections::HashSet::new();
    for view in &list {
        seen.insert(view.entity);
        let Some(&blip) = runtime.blips.get(&view.entity) else {
            let id = spawn_ship_blip(&mut commands, viewport, view, &icons);
            runtime.blips.insert(view.entity, id);
            continue;
        };
        // A just-spawned blip is not queryable until the command flush, so its
        // updates simply wait a frame.
        let Ok((mut node, mut visibility, mut border, children)) = q_blip.get_mut(blip) else {
            continue;
        };
        if !stale_camera {
            // Project the section's SCENE-space position (its local offset - the
            // scene root is anchored at the origin, so blocks and blips share this
            // frame). Projecting the world position would only line up when the ship
            // sits at the world origin, so blips would drift off the blocks in flight.
            let projected = camera
                .world_to_viewport(cam_gt, view.local.translation)
                .ok()
                .map(|p| p * to_logical)
                .filter(|p| p.x >= 0.0 && p.y >= 0.0 && p.x <= size.x && p.y <= size.y);
            if let Some(p) = projected {
                let left = Val::Px(p.x - SHIP_BLIP_PX * 0.5);
                let top = Val::Px(p.y - SHIP_BLIP_PX * 0.5);
                if node.left != left || node.top != top {
                    node.left = left;
                    node.top = top;
                }
            }
            visibility.set_if_neq(match projected {
                Some(_) => Visibility::Inherited,
                None => Visibility::Hidden,
            });
        }
        let selected = runtime.selected == Some(view.entity);
        let alpha = if selected { 1.0 } else { 0.0 };
        if border.alpha != alpha {
            border.alpha = alpha;
        }
        let status = view.status_color();
        for child in children.iter() {
            if let Ok(mut pip) = q_pip.get_mut(child) {
                if pip.color != status {
                    pip.color = status;
                }
            }
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

pub(crate) const SHIP_BLIP_PX: f32 = 24.0;
pub(crate) const SHIP_BLIP_BORDER_PX: f32 = 2.0;

/// Where the label starts, measured from the badge's PADDING edge - which is
/// where an absolutely-positioned child's `left` is measured from, i.e. already
/// inside the badge's border. Offsetting by the border width lands the label
/// exactly on the badge's outer right edge, so badge and label are one
/// unbroken hit target.
pub(crate) const SHIP_LABEL_LEFT_PX: f32 = SHIP_BLIP_PX - SHIP_BLIP_BORDER_PX;
/// Fraction of the collider a block's body fills; the remainder is the gap to
/// its neighbours that the bright outline frames.
pub const SHIP_BLOCK_FILL_SCALE: f32 = 0.86;

pub(crate) fn spawn_ship_blip(
    commands: &mut Commands,
    viewport: Entity,
    view: &ShipSectionView,
    icons: &InterfaceIcons,
) -> Entity {
    let icon = SectionIconType::of(view.kind);
    let badge = commands
        .spawn((
            ShipBlip {
                section: view.entity,
            },
            Button,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(SHIP_BLIP_PX),
                height: Val::Px(SHIP_BLIP_PX),
                border: UiRect::all(Val::Px(SHIP_BLIP_BORDER_PX)),
                border_radius: BorderRadius::all(Val::Px(5.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            // Hidden until the first projection places it.
            Visibility::Hidden,
            BackgroundColor(Color::NONE),
            ThemedFill::alpha(UiColor::Void, 0.8),
            BorderColor::all(Color::NONE),
            ThemedBorder::alpha(UiColor::Accent, 0.0),
            children![
                icon_node(icons.section(icon), icon.color(), SHIP_BLIP_PX - 6.0),
                (
                    ShipStatusPip,
                    Node {
                        position_type: PositionType::Absolute,
                        right: Val::Px(-4.0),
                        top: Val::Px(-4.0),
                        width: Val::Px(7.0),
                        height: Val::Px(7.0),
                        border_radius: BorderRadius::MAX,
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                    ThemedFill::new(view.status_color()),
                    Pickable::IGNORE,
                ),
            ],
        ))
        // Selection through `Activate` (fires on click or keyboard activation),
        // not `Interaction` polling (`rtt-ui-select-via-activate-not-interaction`).
        .observe(on_ship_blip_click)
        .id();

    // The code label, hidden until this section is the selection
    // (`label_the_selected_section`). A child, so a click on it bubbles to the
    // badge `Button`.
    commands.spawn((
        ShipBlipLabel,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(SHIP_LABEL_LEFT_PX),
            top: Val::Px(SHIP_BLIP_PX * 0.5 - 10.0),
            padding: UiRect::axes(Val::Px(4.0), Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(Color::NONE),
        ThemedFill::alpha(UiColor::Void, 0.8),
        Visibility::Hidden,
        ChildOf(badge),
        children![themed_label(&view.code, 12.0, UiColor::Body)],
    ));
    commands.entity(viewport).add_child(badge);
    badge
}

/// Show a section's code label only while it is the selection.
///
/// The carrier has 2081 sections. A label on each one is not a label layer, it
/// is a wall of overlapping text with the schematic somewhere behind it. Every
/// section keeps its clickable badge; the code appears where the panel is
/// already pointing.
pub(crate) fn label_the_selected_section(
    runtime: Res<ShipRuntime>,
    q_blip: Query<(&ShipBlip, &Children)>,
    mut q_label: Query<&mut Visibility, With<ShipBlipLabel>>,
) {
    for (blip, children) in &q_blip {
        let selected = runtime.selected == Some(blip.section);
        for child in children.iter() {
            let Ok(mut visibility) = q_label.get_mut(child) else {
                continue;
            };
            visibility.set_if_neq(if selected {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            });
        }
    }
}

/// Select a section when its blip is activated, with one click if the
/// selection changes.
pub(crate) fn on_ship_blip_click(
    activate: On<Activate>,
    q_blip: Query<&ShipBlip>,
    mut runtime: ResMut<ShipRuntime>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    if let Ok(blip) = q_blip.get(activate.entity) {
        if runtime.selected != Some(blip.section) {
            runtime.selected = Some(blip.section);
            play_menu_select(&mut commands, bank.as_deref());
        }
    }
}

/// Step the selection `step` sections along the list when Prev or Next is
/// clicked, the same cycle as the `[` and `]` keys. Clicks if the selection
/// moves.
#[expect(
    clippy::type_complexity,
    reason = "the observer's system parameters, spelled out for the closure"
)]
pub(crate) fn on_ship_step_button(
    step: isize,
) -> impl FnMut(
    On<Activate>,
    Res<State<PauseStates>>,
    ResMut<ShipRuntime>,
    ShipSections,
    Option<Res<SoundBank<UiSfx>>>,
    Commands,
) {
    move |_activate, pause, mut runtime, sections, bank, mut commands| {
        if *pause.get() != PauseStates::Interface || runtime.rebinding.is_some() {
            return;
        }
        let list = sections.collect();
        let current = runtime
            .selected
            .and_then(|sel| list.iter().position(|v| v.entity == sel));
        if let Some(next) = cycle_index(current, list.len(), step > 0) {
            if runtime.selected != Some(list[next].entity) {
                runtime.selected = Some(list[next].entity);
                play_menu_select(&mut commands, bank.as_deref());
            }
        }
    }
}

/// Frame the whole ship when Fit is clicked: ease the centre home and set the
/// radius that fits every section, keeping the current angles. Clicks once.
pub(crate) fn on_ship_fit_button(
    _activate: On<Activate>,
    pause: Res<State<PauseStates>>,
    runtime: Res<ShipRuntime>,
    sections: ShipSections,
    mut q_camera: Query<&mut ShipOrbit, With<ShipCameraMarker>>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    if *pause.get() != PauseStates::Interface {
        return;
    }
    let Ok(mut orbit) = q_camera.single_mut() else {
        return;
    };
    let (_, radius) = ship_framing(&sections.collect());
    orbit.radius = radius;
    orbit.center_target = orbit.center_home;
    orbit.centered_on = runtime.selected;
    play_menu_select(&mut commands, bank.as_deref());
}

/// Restore the default view when Reset is clicked, the same reframe as the
/// `viewer_reframe` key. Clicks once.
pub(crate) fn on_ship_reset_button(
    _activate: On<Activate>,
    pause: Res<State<PauseStates>>,
    runtime: Res<ShipRuntime>,
    sections: ShipSections,
    mut q_camera: Query<&mut ShipOrbit, With<ShipCameraMarker>>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    if *pause.get() != PauseStates::Interface {
        return;
    }
    if let Ok(mut orbit) = q_camera.single_mut() {
        let (_, radius) = ship_framing(&sections.collect());
        reset_ship_orbit(&mut orbit, radius, runtime.selected);
        play_menu_select(&mut commands, bank.as_deref());
    }
}

/// Re-frame the whole ship as the pane opened: restore the default angles and
/// the `radius` that fits every section, and retarget the centre home.
/// Consuming the current selection (`centered_on = selected`)
/// makes the reframe STICK - `ship_input` will not chase the still-selected
/// section back.
fn reset_ship_orbit(orbit: &mut ShipOrbit, radius: f32, selected: Option<Entity>) {
    orbit.theta = SHIP_THETA_DEFAULT;
    orbit.phi = SHIP_PHI_DEFAULT;
    orbit.radius = radius;
    orbit.center_target = orbit.center_home;
    orbit.centered_on = selected;
}

/// Raise a Repair on the selected section when the panel button is clicked, unless
/// the panel marked repair disabled for it. Same seam as the `P` key. Clicks
/// once when it raises the command.
pub(crate) fn on_ship_repair_button(
    _activate: On<Activate>,
    pause: Res<State<PauseStates>>,
    runtime: Res<ShipRuntime>,
    mut section_commands: MessageWriter<SectionRepairCommand>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    if *pause.get() != PauseStates::Interface || !runtime.panel_repair_enabled {
        return;
    }
    if let Some(target) = runtime.selected {
        section_commands.write(SectionRepairCommand { target });
        play_menu_select(&mut commands, bank.as_deref());
    }
}

/// Arm a rebind capture on the selected section when the panel button is
/// clicked, unless the panel marked rebind disabled for it. Clicks once when
/// it arms.
pub(crate) fn on_ship_rebind_button(
    _activate: On<Activate>,
    pause: Res<State<PauseStates>>,
    mut runtime: ResMut<ShipRuntime>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    if *pause.get() != PauseStates::Interface || !runtime.panel_rebind_enabled {
        return;
    }
    runtime.rebinding = runtime.selected;
    runtime.rebind_awaiting_release = runtime.rebinding.is_some();
    runtime.note = None;
    if runtime.rebinding.is_some() {
        play_menu_select(&mut commands, bank.as_deref());
    }
}

/// Refresh the section panel from the current selection: title, status line,
/// family icon, condition bar, detail, button enabled-state, and the note line
/// (a transient action result, or the reason a button is disabled). Caches the
/// enabled flags for the observers.
#[expect(
    clippy::too_many_arguments,
    reason = "one system writing every live part of the panel"
)]
pub(crate) fn update_ship_panel(
    mut commands: Commands,
    mut runtime: ResMut<ShipRuntime>,
    sections: ShipSections,
    icons: Res<InterfaceIcons>,
    mut q_text: Query<(&ShipPanelField, &mut Text, &mut ThemedText)>,
    q_button: Query<(Entity, &ShipPanelButton, Has<InteractionDisabled>)>,
    mut q_preview: Query<(&mut ImageNode, &mut ThemedImageTint), With<ShipPreviewIcon>>,
    mut q_bar: Query<(&mut Node, &mut ThemedFill), With<ShipConditionFill>>,
    q_inventory: Query<&ShipInventory, With<PlayerSpaceshipMarker>>,
) {
    if !runtime.active {
        return;
    }
    let selected = runtime
        .selected
        .and_then(|sel| sections.collect().into_iter().find(|v| v.entity == sel));

    let (title, status, detail, detail_color, actions) = match &selected {
        Some(view) => (
            format!("{}  {}", view.code, view.name),
            panel_status_text(view),
            panel_detail_text(view),
            UiColor::Body,
            panel_action_state(
                view,
                q_inventory
                    .single()
                    .expect("the player ship carries a ShipInventory")
                    .count(ItemType::HullPlate),
            ),
        ),
        None => (
            "No section".to_string(),
            String::new(),
            "Select a section:\nclick a badge or use Prev / Next.".to_string(),
            UiColor::Label,
            PanelActions::none(),
        ),
    };
    let (summary, summary_color) = match &selected {
        Some(view) => (
            format!("{title}  {}  {}", view.integrity_pct(), view.status()),
            view.status_color(),
        ),
        None => (title.clone(), UiColor::Label),
    };

    runtime.panel_repair_enabled = actions.repair_enabled;
    runtime.panel_rebind_enabled = selected
        .as_ref()
        .is_some_and(|view| view.bindings.is_some());

    // Note line: a transient action result wins; else the disabled reason.
    let (note, note_color) = if runtime.rebinding.is_some() {
        (
            "PRESS A KEY OR MOUSE BUTTON - ESC CANCELS".to_string(),
            UiColor::Accent,
        )
    } else if let Some((note, _)) = &runtime.note {
        (note.clone(), UiColor::Accent)
    } else if let Some(reason) = &actions.reason {
        (reason.clone(), UiColor::Label)
    } else {
        (String::new(), UiColor::Label)
    };

    for (field, mut text, mut themed) in &mut q_text {
        let (value, tint) = match field {
            ShipPanelField::Title => (&title, UiColor::Primary),
            ShipPanelField::Status => (&status, UiColor::Body),
            ShipPanelField::Detail => (&detail, detail_color),
            ShipPanelField::Note => (&note, note_color),
            ShipPanelField::Summary => (&summary, summary_color),
        };
        if text.0 != *value {
            text.0 = value.clone();
        }
        if themed.color != tint {
            themed.color = tint;
        }
    }

    for (entity, button, disabled) in &q_button {
        let enabled = match button {
            ShipPanelButton::Repair => actions.repair_enabled,
            ShipPanelButton::Rebind => runtime.panel_rebind_enabled,
        };
        // Runs every frame the pane owns the screen, so only a change in the
        // enabled state touches the button.
        if enabled && disabled {
            commands.entity(entity).remove::<InteractionDisabled>();
        } else if !enabled && !disabled {
            commands.entity(entity).insert(InteractionDisabled);
        }
    }

    let Some(view) = selected else {
        return;
    };
    let icon = SectionIconType::of(view.kind);
    for (mut image, mut tint) in &mut q_preview {
        let wanted = icons.section(icon);
        if image.image != wanted {
            image.image = wanted;
        }
        if tint.color != icon.color() {
            tint.color = icon.color();
        }
    }
    let width = Val::Percent(view.integrity().unwrap_or(1.0) * 100.0);
    let status_color = view.status_color();
    for (mut node, mut fill) in &mut q_bar {
        if node.width != width {
            node.width = width;
        }
        if fill.color != status_color {
            fill.color = status_color;
        }
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
pub fn cuboid_edges() -> Mesh {
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
