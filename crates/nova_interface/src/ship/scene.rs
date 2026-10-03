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
    ui_widgets::{Activate, Button, SliderRange, SliderValue, ValueChange},
};
use nova_gameplay::prelude::*;
use nova_ship::prelude::{derive_link_point_graph, PlacedSectionLinkPoints};
use nova_ui::{
    theme::{ActiveUiTheme, UiColor},
    widget::{
        TextFieldError, TextFieldFocused, TextFieldValue, ThemedBorder, ThemedFill,
        ThemedImageTint, ThemedText,
    },
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
/// Which live part of the panel a node is, so one system refreshes them all.
/// On a text node the field is the text it shows; on a node without text it
/// is the part that holds that text, shown only while the selection has it.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShipPanelField {
    Title,
    Status,
    /// What the section kind does.
    About,
    Integrity,
    Ammo,
    /// The section's input binding.
    Control,
    /// The repair form, shown only while the selection has repairable damage.
    RepairForm,
    /// The live hull plate stock beside the typed quantity.
    RepairStock,
    /// The predicted integrity, or why the repair is refused.
    RepairPreview,
    Note,
}
/// The selected section's family icon in the panel head.
#[derive(Component)]
pub(crate) struct ShipPreviewIcon;
/// The panel's condition bar fill, as wide as the selected section's
/// integrity.
#[derive(Component)]
pub(crate) struct ShipConditionFill;
/// A whole-plate selector for the current repair draft: the typed field and
/// the slider.
#[derive(Component)]
pub(crate) struct ShipRepairQuantity;
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
    /// Draft quantity belongs to this section; changing selection starts at All.
    /// A full section, or any section before the player ship's inventory
    /// exists, does not take the draft, so damage or arriving stock starts it
    /// at All instead of the 0 snapshot taken before.
    pub(crate) repair_target: Option<Entity>,
    /// The repair draft in whole plates. `None` while the field holds text
    /// that is not a whole number, which Repair refuses.
    pub(crate) requested_plates: Option<u32>,
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
        runtime.repair_target = None;
        runtime.requested_plates = None;
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
    q_inventory: Query<&ShipInventory, With<PlayerSpaceshipMarker>>,
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
            let (theta, phi) = gesture.apply_ship(dt, orbit.theta, orbit.phi);
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

    // A changed selection starts at its own current All, before either P or a
    // panel click can reuse a quantity from the previous section.
    if runtime.repair_target != runtime.selected {
        let view = runtime
            .selected
            .and_then(|sel| list.iter().find(|view| view.entity == sel));
        // Mid-transition the pane can be active a frame before the player
        // ship's ShipInventory exists; treat it as empty stock rather than
        // panicking, and leave the draft unowned until the stock arrives.
        let stock = q_inventory
            .single()
            .ok()
            .map(|stock| stock.count(ItemType::HullPlate));
        runtime.requested_plates = view.map(|view| {
            plate_repair_limit(view.health.as_ref(), view.disabled, stock.unwrap_or(0))
        });
        if runtime.selected.is_none()
            || (stock.is_some()
                && view.is_none_or(|view| {
                    plate_repair_limit(view.health.as_ref(), view.disabled, u32::MAX) > 0
                }))
        {
            runtime.repair_target = runtime.selected;
        }
    }
    // What the app does to the section it has selected. Route mutation actions
    // through their shared seams.
    if let Some(sel) = runtime.selected {
        if input.just_pressed("ship_rebind")
            && list
                .iter()
                .find(|view| view.entity == sel)
                .is_some_and(|view| {
                    view.bindings
                        .as_ref()
                        .is_some_and(|bindings| !bindings.is_empty())
                })
        {
            runtime.rebinding = Some(sel);
            runtime.rebind_awaiting_release = true;
            runtime.note = None;
            return;
        }
        if input.just_pressed("ship_repair") {
            let repairable = list
                .iter()
                .find(|view| view.entity == sel)
                .is_some_and(|view| {
                    plate_repair_limit(view.health.as_ref(), view.disabled, u32::MAX) > 0
                });
            // With the form shown, an empty, invalid or zero quantity is not
            // sent. Without it, P still sends, so the refusal reaches the
            // note line.
            let requested = if repairable {
                runtime.requested_plates.filter(|plates| *plates > 0)
            } else {
                Some(runtime.requested_plates.unwrap_or(0))
            };
            if let Some(requested_plates) = requested {
                commands.write(SectionRepairCommand {
                    target: sel,
                    requested_plates,
                });
            }
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
    if *pause.get() != PauseStates::Interface
        || !runtime.panel_repair_enabled
        || runtime.repair_target != runtime.selected
    {
        return;
    }
    if let (Some(target), Some(requested_plates)) = (runtime.selected, runtime.requested_plates) {
        section_commands.write(SectionRepairCommand {
            target,
            requested_plates,
        });
        play_menu_select(&mut commands, bank.as_deref());
    }
}

/// Reset the selected section's draft to the current affordable maximum.
pub(crate) fn on_ship_repair_all_button(
    _activate: On<Activate>,
    mut runtime: ResMut<ShipRuntime>,
    sections: ShipSections,
    inventory: Query<&ShipInventory, With<PlayerSpaceshipMarker>>,
) {
    let Some(view) = runtime.selected.and_then(|selected| {
        sections
            .collect()
            .into_iter()
            .find(|view| view.entity == selected)
    }) else {
        return;
    };
    let Ok(stock) = inventory.single() else {
        return;
    };
    runtime.requested_plates = Some(plate_repair_limit(
        view.health.as_ref(),
        view.disabled,
        stock.count(ItemType::HullPlate),
    ));
}

/// Use a whole slider step, never a fractional hull plate.
pub(crate) fn on_ship_repair_slider(
    change: On<ValueChange<f32>>,
    mut runtime: ResMut<ShipRuntime>,
    sections: ShipSections,
    inventory: Query<&ShipInventory, With<PlayerSpaceshipMarker>>,
) {
    let Some(view) = runtime.selected.and_then(|selected| {
        sections
            .collect()
            .into_iter()
            .find(|view| view.entity == selected)
    }) else {
        return;
    };
    let Ok(stock) = inventory.single() else {
        return;
    };
    let max = plate_repair_limit(
        view.health.as_ref(),
        view.disabled,
        stock.count(ItemType::HullPlate),
    );
    if max > 0 {
        runtime.requested_plates = Some(change.value.round().clamp(1.0, max as f32) as u32);
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
/// family icon, condition bar, description, fact rows, the repair form,
/// button enabled-state, and the note line (the rebind prompt or a transient
/// action result). Caches the enabled flags for the observers.
///
/// The repair form shows only while the selection has repairable damage, and
/// disables Repair with the reason in its summary while the draft is refused.
/// Typed text becomes the draft before a selection change, damage to a full
/// selected section, or the player ship's inventory arriving resets it to All.
/// The field is rewritten only when the draft holds a number its text does not
/// read as, so invalid text stays for the player to fix, focused or not; a
/// hidden form lets go of the keyboard.
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
    mut q_part: Query<
        (&ShipPanelField, &mut Node),
        (
            Without<Text>,
            Without<ShipPanelButton>,
            Without<ShipConditionFill>,
            Without<ShipRepairQuantity>,
        ),
    >,
    mut q_button: Query<
        (
            Entity,
            &ShipPanelButton,
            Has<InteractionDisabled>,
            &mut Node,
        ),
        (Without<ShipConditionFill>, Without<ShipRepairQuantity>),
    >,
    mut q_preview: Query<(&mut ImageNode, &mut ThemedImageTint), With<ShipPreviewIcon>>,
    mut q_bar: Query<
        (&mut Node, &mut ThemedFill),
        (
            With<ShipConditionFill>,
            Without<ShipRepairQuantity>,
            Without<ShipPanelButton>,
        ),
    >,
    mut q_slider: Query<
        (Entity, &SliderRange, &SliderValue, &mut Node),
        (With<ShipRepairQuantity>, Without<ShipPanelButton>),
    >,
    mut q_field: Query<
        (
            Entity,
            &mut TextFieldValue,
            Has<TextFieldFocused>,
            Has<TextFieldError>,
        ),
        With<ShipRepairQuantity>,
    >,
    q_inventory: Query<&ShipInventory, With<PlayerSpaceshipMarker>>,
) {
    if !runtime.active {
        return;
    }
    let selected = runtime
        .selected
        .and_then(|sel| sections.collect().into_iter().find(|v| v.entity == sel));

    // Mid-transition (e.g. a fresh spawn or a ship swap) the pane can be
    // active a frame before the player ship's ShipInventory exists; render the
    // no-section state rather than panicking on a query the scene hasn't
    // caught up to yet.
    let inventory = q_inventory.single().ok();
    let stock = inventory.map_or(0, |inventory| inventory.count(ItemType::HullPlate));
    let selected = inventory.and(selected);
    // This system's own rewrite below does not read as a change on its next
    // run, so only the player's typing lands here.
    for (_, value, _, _) in &mut q_field {
        if value.is_changed() && !value.is_added() {
            runtime.requested_plates = value.trim().parse::<u32>().ok();
        }
    }
    if runtime.repair_target != runtime.selected {
        runtime.requested_plates = selected
            .as_ref()
            .map(|view| plate_repair_limit(view.health.as_ref(), view.disabled, stock));
        if runtime.selected.is_none()
            || (inventory.is_some()
                && selected.as_ref().is_none_or(|view| {
                    plate_repair_limit(view.health.as_ref(), view.disabled, u32::MAX) > 0
                }))
        {
            runtime.repair_target = runtime.selected;
        }
    }
    let requested = runtime.requested_plates;
    let limit = selected.as_ref().map_or(0, |view| {
        plate_repair_limit(view.health.as_ref(), view.disabled, stock)
    });
    let actions = match (&selected, requested) {
        (Some(view), Some(requested)) => panel_action_state(view, requested, stock),
        _ => PanelActions::none(),
    };
    // The form's stock line and summary, while the selection has repairable
    // damage. A refused summary is the same reason the handler would note.
    let form = selected
        .as_ref()
        .filter(|view| plate_repair_limit(view.health.as_ref(), view.disabled, u32::MAX) > 0)
        .map(|view| {
            let summary = match requested {
                None => Err("Type a whole number".to_string()),
                Some(requested) => {
                    plan_plate_repair(view.health.as_ref(), view.disabled, requested, stock)
                        .map(|repair| {
                            format!(
                                "Predicted integrity: {:.0}/{:.0} HP\n1 hull plate restores up to {:.0} HP.",
                                repair.current,
                                view.health.as_ref().map_or(0.0, |h| h.max),
                                HULL_PLATE_HEALTH
                            )
                        })
                        .map_err(|_| actions.reason.clone().unwrap_or_default())
                }
            };
            let noun = if stock == 1 { "plate" } else { "plates" };
            (format!("{stock} {noun} in stock"), summary)
        });
    let (title, status, about, about_color) = match &selected {
        Some(view) => (
            format!("{}  {}", view.code, view.name),
            panel_status_text(view),
            kind_description(view.kind).to_string(),
            UiColor::Body,
        ),
        None => (
            "No section".to_string(),
            String::new(),
            "Select a section:\nclick a badge or use Prev / Next.".to_string(),
            UiColor::Label,
        ),
    };
    let integrity = selected.as_ref().map(ShipSectionView::health_text);
    let ammo = selected
        .as_ref()
        .and_then(|view| view.ammo)
        .map(|ammo| format!("{} / {} rounds", ammo.rounds, ammo.capacity));
    let control = selected.as_ref().and_then(ShipSectionView::binding_text);

    runtime.panel_repair_enabled = actions.repair_enabled;
    runtime.panel_rebind_enabled = selected.as_ref().is_some_and(|view| {
        view.bindings
            .as_ref()
            .is_some_and(|bindings| !bindings.is_empty())
    });

    // Note line: the rebind prompt, else a transient action result. A
    // disabled Repair gives its reason in the form summary.
    let (note, note_color) = if runtime.rebinding.is_some() {
        (
            "PRESS A KEY OR MOUSE BUTTON - ESC CANCELS".to_string(),
            UiColor::Accent,
        )
    } else if let Some((note, _)) = &runtime.note {
        (note.clone(), UiColor::Accent)
    } else {
        (String::new(), UiColor::Label)
    };

    for (field, mut node) in &mut q_part {
        let shown = match field {
            ShipPanelField::Integrity => integrity.is_some(),
            ShipPanelField::Ammo => ammo.is_some(),
            ShipPanelField::Control => control.is_some(),
            ShipPanelField::RepairForm => form.is_some(),
            _ => true,
        };
        let display = if shown { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
    }

    for (field, mut text, mut themed) in &mut q_text {
        let (value, tint) = match field {
            ShipPanelField::Title => (title.as_str(), UiColor::Primary),
            ShipPanelField::Status => (status.as_str(), UiColor::Body),
            ShipPanelField::About => (about.as_str(), about_color),
            ShipPanelField::Integrity => {
                (integrity.as_deref().unwrap_or_default(), UiColor::Primary)
            }
            ShipPanelField::Ammo => (ammo.as_deref().unwrap_or_default(), UiColor::Primary),
            ShipPanelField::Control => (control.as_deref().unwrap_or_default(), UiColor::Primary),
            ShipPanelField::RepairStock => (
                form.as_ref().map_or("", |(stock, _)| stock.as_str()),
                UiColor::Primary,
            ),
            ShipPanelField::RepairPreview => match form.as_ref().map(|(_, summary)| summary) {
                Some(Ok(summary)) => (summary.as_str(), UiColor::Body),
                Some(Err(reason)) => (reason.as_str(), UiColor::Danger),
                None => ("", UiColor::Body),
            },
            ShipPanelField::Note => (note.as_str(), note_color),
            ShipPanelField::RepairForm => continue,
        };
        if text.0 != value {
            text.0 = value.to_string();
        }
        if themed.color != tint {
            themed.color = tint;
        }
    }

    for (entity, button, disabled, mut node) in &mut q_button {
        let enabled = match button {
            ShipPanelButton::Repair => actions.repair_enabled,
            ShipPanelButton::Rebind => runtime.panel_rebind_enabled,
        };
        if *button == ShipPanelButton::Rebind {
            let wanted = if enabled {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != wanted {
                node.display = wanted;
            }
        }
        // Runs every frame the pane owns the screen, so only a change in the
        // enabled state touches the button.
        if enabled && disabled {
            commands.entity(entity).remove::<InteractionDisabled>();
        } else if !enabled && !disabled {
            commands.entity(entity).insert(InteractionDisabled);
        }
    }

    for (entity, range, value, mut node) in &mut q_slider {
        let display = if limit > 1 {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
        let wanted = SliderRange::new(1.0, limit.max(1) as f32);
        if *range != wanted {
            commands.entity(entity).insert(wanted);
        }
        // Invalid text keeps the last whole number on the slider.
        if let Some(requested) = requested {
            let shown = requested.clamp(1, limit.max(1)) as f32;
            if value.0 != shown {
                commands.entity(entity).insert(SliderValue(shown));
            }
        }
    }

    for (entity, mut value, focused, marked) in &mut q_field {
        if form.is_none() {
            if focused {
                commands.entity(entity).remove::<TextFieldFocused>();
            }
            continue;
        }
        let Some(requested) = requested else {
            if !marked {
                commands
                    .entity(entity)
                    .insert(TextFieldError(String::new()));
            }
            continue;
        };
        if marked {
            commands.entity(entity).remove::<TextFieldError>();
        }
        if value.trim().parse::<u32>().ok() == Some(requested) {
            continue;
        }
        value.0 = requested.to_string();
        if focused {
            commands
                .entity(entity)
                .insert(TextFieldFocused::at_end(&value.0));
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
