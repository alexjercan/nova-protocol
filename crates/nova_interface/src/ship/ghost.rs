//! The Ship pane's placement ghost: the catalogue part in hand, shown as a
//! block at the free socket nearest the pointer, with its price or refusal
//! in a callout beside it, and the click that adds it.
//!
//! One solve a frame ([`update_ship_placement`]) feeds the ghost block, the
//! callout and the click, so a click adds what is on screen. The pointer's
//! ray picks the block under it ([`pick_ship_block`]); of that section's
//! targets ([`ghost_targets`]: free sockets open to space where the part
//! stands clear) the one nearest the hit ([`nearest_link_point`]) is the
//! point. A block with no target, such as one inside the hull, shows no
//! ghost. [`section_add_pose`] at the player's [`MountPose`] (R rolls the
//! part, F cycles its mating socket) gives the ghost's pose, and
//! [`section_add_fit`] at that pose with the payment and binding plan gives
//! the verdict. A valid verdict names the price and how many parts come from
//! the hold and how many the click buys from the station at ask, so a part
//! whose recipe the hold cannot cover is bought and placed in one click. A click on a valid
//! ghost raises the [`SectionAddCommand`] the add handler applies by the same
//! rule; a click on a refused one changes nothing and shows the refusal on
//! the note line. With no part in hand, a click on a block selects its
//! section.
//!
//! Touch this module when changing how the ghost is aimed, drawn or
//! committed.

use std::hash::{DefaultHasher, Hash, Hasher};

use bevy::{
    camera::visibility::RenderLayers,
    math::bounding::{Aabb3d, RayCast3d},
    picking::events::{Click, Enter, Leave, Move, Pointer},
    prelude::*,
};
use nova_gameplay::{prelude::*, PauseStates};
use nova_input::prelude::{InputBindings, InputSources};
use nova_ship::prelude::*;
use nova_ui::{
    theme::UiColor,
    widget::{ThemedFill, ThemedText},
};

use super::{
    rebuild::{rebuild_price_text, StationSite},
    refit::{add_refusal_text, live_bindings, SectionAddCommand, SectionBindingRow},
    scene::{ShipCameraMarker, ShipRuntime, ShipViewportMarker, SHIP_BLOCK_FILL_SCALE},
    sections::{design_section_code, ShipSectionView, ShipSections},
    SHIP_LAYER,
};
use crate::{
    icons::SectionIconType,
    pane::{play_menu_select, themed_label, InterfacePaneType},
};

/// What a click on the view would add, solved under the pointer. Read by
/// [`ship_placement`] for a diagnostics overlay.
#[derive(Clone, Debug, PartialEq)]
pub struct ShipPlacement {
    /// The design section id the part mounts on.
    pub section: String,
    /// The socket id of that section the part mates.
    pub point: String,
    /// The catalogue part in hand.
    pub prototype: String,
    /// How the part mates the socket.
    pub mount: MountPose,
    /// The part's pose in ship-local space, or `None` when it has no socket
    /// to mount by.
    pub pose: Option<Transform>,
    /// Where the callout points, in ship-local space: the target socket.
    pub anchor: Vec3,
    /// The target socket's outward normal, in ship-local space.
    pub normal: Vec3,
    /// The price line of a valid add, or the refusal line.
    pub verdict: Result<String, String>,
    /// The payment a click makes: the hold's parts, the parts bought from
    /// the station at ask, and the price. `None` for a refused add.
    pub payment: Option<RecipePayment>,
}

/// The block under the pointer and the placement a click would add,
/// rebuilt every frame by [`update_ship_placement`].
#[derive(Resource, Default, Debug)]
pub(crate) struct ShipPlacementPreview {
    /// The live section whose block the pointer is over.
    pub(crate) hovered: Option<Entity>,
    /// The placement of the part in hand, while one is aimed.
    pub(crate) placement: Option<ShipPlacement>,
}

/// The ghost block in the schematic scene, for one catalogue part.
#[derive(Component)]
pub(crate) struct ShipGhostBlock {
    prototype: String,
}

/// The ghost block's outline, in the valid or the refused colour.
#[derive(Component)]
pub(crate) struct ShipGhostOutline;

/// The callout beside the ghost: the price or the refusal.
#[derive(Component)]
pub(crate) struct ShipGhostCallout;

/// The `Name` of the ghost callout, for driven runs.
pub const SHIP_GHOST_CALLOUT: &str = "ShipGhostCallout";

/// The callout's offset right of and below the point it names, in logical
/// pixels: clear of the selected section's code label, which sits level
/// with its badge.
const CALLOUT_OFFSET_PX: Vec2 = Vec2::new(12.0, 14.0);

/// The fit of one part at one point, and the targets of one section for one
/// part ([`ghost_targets`]), each kept while the ship, the section or point
/// and the part stay the same: a fit reads the socket graph and the exit
/// lanes of the whole ship, and the targets read every block, too much to
/// redo every frame.
#[derive(Default)]
pub(crate) struct GhostFitCache {
    key: Option<u64>,
    fit: Option<Result<(Vec3, Quat), SectionAddFaultType>>,
    targets_key: Option<u64>,
    targets: Vec<usize>,
}

/// Keep the pointer's place while it is over the view or a badge on it.
pub(crate) fn on_ship_viewport_enter(enter: On<Pointer<Enter>>, mut runtime: ResMut<ShipRuntime>) {
    runtime.pointer = Some(enter.pointer_location.position);
}

/// Follow the pointer while it moves over the view or a badge on it.
pub(crate) fn on_ship_viewport_move(moved: On<Pointer<Move>>, mut runtime: ResMut<ShipRuntime>) {
    runtime.pointer = Some(moved.pointer_location.position);
}

/// Forget the pointer when it leaves the view.
pub(crate) fn on_ship_viewport_leave(_leave: On<Pointer<Leave>>, mut runtime: ResMut<ShipRuntime>) {
    runtime.pointer = None;
}

/// Add the part in hand where the ghost stands, or select the block under
/// the pointer when no part is in hand.
///
/// With a part in hand a click on a badge places too, so the badge under a
/// small block does not swallow it. With none, a badge click is the badge's
/// own selection, and only a click on the bare view picks a block.
pub(crate) fn on_ship_viewport_click(
    click: On<Pointer<Click>>,
    preview: Res<ShipPlacementPreview>,
    mut runtime: ResMut<ShipRuntime>,
    mut adds: MessageWriter<SectionAddCommand>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    if click.button != PointerButton::Primary {
        return;
    }
    if runtime.held.is_some() {
        commit_placement(
            &preview,
            &mut runtime,
            &mut adds,
            bank.as_deref(),
            &mut commands,
        );
        return;
    }
    if click.original_event_target() != click.entity {
        return;
    }
    if let Some(hovered) = preview.hovered {
        if runtime.selected != Some(hovered) {
            runtime.selected = Some(hovered);
            play_menu_select(&mut commands, bank.as_deref());
        }
    }
}

/// Commit the solved placement on a click with a part in hand: a valid one
/// raises its [`SectionAddCommand`], a refused one shows its refusal on the
/// note line, and no placement changes nothing. A click on the view and a
/// click on a section badge, which stops the click there, both commit here.
pub(crate) fn commit_placement(
    preview: &ShipPlacementPreview,
    runtime: &mut ShipRuntime,
    adds: &mut MessageWriter<SectionAddCommand>,
    bank: Option<&SoundBank<UiSfx>>,
    commands: &mut Commands,
) {
    let Some(placement) = &preview.placement else {
        return;
    };
    match &placement.verdict {
        Ok(_) => {
            adds.write(SectionAddCommand {
                section: placement.section.clone(),
                point: placement.point.clone(),
                prototype: placement.prototype.clone(),
                mount: placement.mount,
            });
            play_menu_select(commands, bank);
        }
        Err(refusal) => runtime.note = Some((refusal.clone(), 2.5)),
    }
}

/// The ray into the schematic scene under the pointer at `pointer`, in
/// logical window pixels, or `None` when it is off the view.
///
/// The scene camera draws into an image whose pixels are the view node's
/// physical pixels, so the pointer is moved into the node's physical space
/// before the camera unprojects it.
fn view_ray(
    pointer: Vec2,
    camera: &Camera,
    camera_pose: &GlobalTransform,
    node: &ComputedNode,
    transform: &UiGlobalTransform,
) -> Option<Ray3d> {
    let scale = node.inverse_scale_factor();
    if scale <= 0.0 {
        return None;
    }
    let size = node.size();
    let local = pointer / scale - (transform.translation - size * 0.5);
    if local.cmplt(Vec2::ZERO).any() || local.cmpgt(size).any() {
        return None;
    }
    camera.viewport_to_world(camera_pose, local).ok()
}

/// Where `ray` enters `view`'s block, in the scene's ship-local space. A
/// block is its section's full collider box at its local pose, the box its
/// outline draws. A ray that starts inside the box enters it where it starts.
fn enter_block(ray: Ray3d, view: &ShipSectionView) -> Option<Vec3> {
    let to_local = view.local.compute_affine().inverse();
    let origin = to_local.transform_point3(ray.origin);
    let direction = Dir3::new(to_local.transform_vector3(*ray.direction)).ok()?;
    let half = view.half_extents.max(Vec3::splat(0.1));
    let distance = RayCast3d::new(origin, direction, f32::MAX)
        .aabb_intersection_at(&Aabb3d::new(Vec3::ZERO, half))?;
    Some(view.local.transform_point(origin + *direction * distance))
}

/// The block `ray` enters first among `views`, and where it enters it, in
/// the scene's ship-local space.
pub(crate) fn pick_ship_block(ray: Ray3d, views: &[ShipSectionView]) -> Option<(Entity, Vec3)> {
    views
        .iter()
        .filter_map(|view| {
            let hit = enter_block(ray, view)?;
            Some((view.entity, hit, ray.origin.distance_squared(hit)))
        })
        .min_by(|a, b| a.2.total_cmp(&b.2))
        .map(|(entity, hit, _)| (entity, hit))
}

/// How far off its face a socket's outward lane starts, in engine units, so
/// the lane does not count the face it leaves. A section that touches the
/// face still blocks it: the lane starts inside that section's box.
const LANE_START: f32 = 1e-3;

/// The sockets of design section `index`, whose block is `own`, that the
/// ghost may aim `offered` at; the pointer takes the one nearest its hit
/// ([`nearest_link_point`]). Empty when the section has none, and the ghost
/// shows nothing there.
///
/// A target is a free socket ([`free_link_points`]) whose outward lane
/// leaves the ship without entering another block in `views`, where the
/// part stands clear as drawn ([`section_add_clear`] at the default
/// [`MountPose`]), so a roll or socket step never moves the ghost to
/// another point: the fit at the chosen pose gives the verdict. A section
/// inside the hull has no open lane, and a socket a part covers fails the
/// clear check, so neither is ever a target. A part with no socket keeps
/// every open point, so the ghost still shows its refusal.
pub(crate) fn ghost_targets(
    design: &[RefitPlacedSection<'_>],
    index: usize,
    own: Entity,
    views: &[ShipSectionView],
    offered: &SectionConfig,
) -> Vec<usize> {
    let target = &design[index];
    let sockets = &target.config.base.link_points;
    free_link_points(design, index)
        .into_iter()
        .filter(|&point| {
            let (anchor, normal) =
                link_point_pose(target.position, target.rotation, &sockets[point]);
            socket_lane_open(anchor, normal, own, views)
        })
        .filter(|&point| {
            !matches!(
                section_add_clear(design, index, point, offered, MountPose::default()),
                Err(SectionAddFaultType::Occupied | SectionAddFaultType::Overlap)
            )
        })
        .collect()
}

/// Whether the outward lane of a socket at `anchor` facing `normal`, both
/// in ship-local space, leaves the ship without entering a block in `views`
/// other than `own`, the block of the section that carries the socket. A
/// free socket with a closed lane is inside the hull, and the ghost never
/// aims there ([`ghost_targets`]).
pub fn socket_lane_open(
    anchor: Vec3,
    normal: Vec3,
    own: Entity,
    views: &[ShipSectionView],
) -> bool {
    let Ok(direction) = Dir3::new(normal) else {
        return false;
    };
    let lane = Ray3d::new(anchor + *direction * LANE_START, direction);
    views
        .iter()
        .filter(|view| view.entity != own)
        .all(|view| enter_block(lane, view).is_none())
}

/// Solve the block under the pointer and, with a part in hand, where a click
/// would add it and whether it may.
#[expect(
    clippy::too_many_arguments,
    reason = "one solve reading the view, the catalogue, both holds and the bindings"
)]
pub(crate) fn update_ship_placement(
    runtime: Res<ShipRuntime>,
    items: Option<Res<GameItems>>,
    sections: Option<Res<GameSections>>,
    site: StationSite,
    views: ShipSections,
    q_holds: Query<(&ShipInventory, &ShipCredits)>,
    q_bindings: Query<SectionBindingRow, With<SectionMarker>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<ShipCameraMarker>>,
    q_viewport: Query<(&ComputedNode, &UiGlobalTransform), With<ShipViewportMarker>>,
    mut preview: ResMut<ShipPlacementPreview>,
    mut cache: Local<GhostFitCache>,
) {
    let mut hovered = None;
    let mut placement = None;
    let blocks: Vec<ShipSectionView> = if runtime.active {
        views.collect()
    } else {
        Vec::new()
    };
    let hit = (|| {
        let pointer = runtime.pointer?;
        let (camera, camera_pose) = q_camera.single().ok()?;
        let (node, transform) = q_viewport.single().ok()?;
        // The camera re-derives its target size in `PostUpdate`; until it has
        // the view's size it unprojects into the old one.
        if camera.physical_target_size() != Some(node.size().round().as_uvec2()) {
            return None;
        }
        let ray = view_ray(pointer, camera, camera_pose, node, transform)?;
        pick_ship_block(ray, &blocks)
    })();
    if let Some((entity, hit)) = hit {
        hovered = Some(entity);
        let resolved = if site.gate(StationServiceType::Refit) {
            site.resolve(StationServiceType::Refit)
        } else {
            None
        };
        if let (Some(prototype), Some(resolved), Some(items), Some(sections)) =
            (runtime.held.as_ref(), resolved, items, sections)
        {
            let offered = sections.get_section(prototype).unwrap_or_else(|| {
                panic!("place: the section catalogue has no part '{prototype}'")
            });
            if let Some(section) = resolved.selected_section(Some(entity)) {
                let (placed, live) = resolved.refit_design();
                let index = resolved.design_index(section);
                // The targets read the whole design and every block, so they
                // are kept while the section, the design and the part stay.
                let mut hasher = DefaultHasher::new();
                resolved.fit_key(section, usize::MAX).hash(&mut hasher);
                prototype.hash(&mut hasher);
                let targets_key = hasher.finish();
                if cache.targets_key != Some(targets_key) {
                    cache.targets = ghost_targets(&placed, index, entity, &blocks, offered);
                    cache.targets_key = Some(targets_key);
                }
                let target = &placed[index];
                let sockets = &target.config.base.link_points;
                let free = nearest_link_point(
                    target.position,
                    target.rotation,
                    sockets,
                    cache.targets.iter().copied(),
                    hit,
                );
                if let Some(free) = free {
                    let code = design_section_code(resolved.design, &section.id)
                        .expect("a live design section has a code");
                    let partner = resolved
                        .partner
                        .and_then(|(partner, _)| q_holds.get(partner).ok())
                        .map(|(stock, credits)| (stock, credits.0));
                    let (anchor, normal) =
                        link_point_pose(target.position, target.rotation, &sockets[free]);
                    let mount = runtime.mount;
                    let pose = section_add_pose(&placed, index, free, offered, mount)
                        .ok()
                        .map(|(position, rotation)| {
                            Transform::from_translation(position).with_rotation(rotation)
                        });
                    let mut hasher = DefaultHasher::new();
                    resolved.fit_key(section, free).hash(&mut hasher);
                    prototype.hash(&mut hasher);
                    mount.hash(&mut hasher);
                    let key = hasher.finish();
                    if cache.key != Some(key) {
                        // The docked station counts too: a part that would
                        // sink into it is refused at the ghost.
                        let outside = site.partner_colliders(&resolved);
                        cache.fit = Some(section_add_fit(
                            &placed, &live, index, free, offered, mount, &outside,
                        ));
                        cache.key = Some(key);
                    }
                    let fit = cache.fit.clone().expect("the cache holds the keyed fit");
                    let (own, own_cr) = q_holds
                        .get(resolved.player)
                        .expect("the player ship carries a ShipInventory and ShipCredits");
                    let bound = live_bindings(&resolved, &q_bindings);
                    let name = &offered.base.name;
                    let (verdict, payment) = match resolved
                        .plan_add(&items, offered, fit, &bound, own, own_cr.0, partner)
                    {
                        Ok(plan) => (
                            Ok(format!(
                                "{name} on {code}, {}",
                                rebuild_price_text(&plan.payment)
                            )),
                            Some(plan.payment),
                        ),
                        Err(refusal) => {
                            (Err(add_refusal_text(&items, &code, name, &refusal)), None)
                        }
                    };
                    placement = Some(ShipPlacement {
                        section: section.id.clone(),
                        point: sockets[free].id.clone(),
                        prototype: prototype.clone(),
                        mount,
                        pose,
                        anchor,
                        normal,
                        verdict,
                        payment,
                    });
                }
            }
        }
    }
    if preview.hovered != hovered {
        preview.hovered = hovered;
    }
    if preview.placement != placement {
        preview.placement = placement;
    }
}

/// Put the part the section under the pointer was built from in hand on
/// `ship_pick` (Q), as the editor's pipette does, at the default
/// [`MountPose`]. Only at a Refit station, where a part in hand can be
/// placed; elsewhere the note line says so. Read off the desk only: the pad
/// has no pointer, and its half of the key turns the view.
#[expect(
    clippy::too_many_arguments,
    reason = "one key read against the pane, the hovered block and the site"
)]
pub(crate) fn pick_ship_part(
    pause: Res<State<PauseStates>>,
    pane: Res<InterfacePaneType>,
    bindings: Res<InputBindings>,
    sources: InputSources,
    preview: Res<ShipPlacementPreview>,
    site: StationSite,
    mut runtime: ResMut<ShipRuntime>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    if *pause.get() != PauseStates::Interface
        || *pane != InterfacePaneType::Ship
        || runtime.rebind_armed()
        || !bindings
            .get("ship_pick")
            .is_some_and(|action| sources.just_pressed_desk(action))
    {
        return;
    }
    let Some(hovered) = preview.hovered else {
        return;
    };
    let resolved = if site.gate(StationServiceType::Refit) {
        site.resolve(StationServiceType::Refit)
    } else {
        None
    };
    let Some(resolved) = resolved else {
        runtime.note = Some((
            "Pick: dock at a Refit station to place parts".to_string(),
            2.5,
        ));
        return;
    };
    let Some(section) = resolved.selected_section(Some(hovered)) else {
        return;
    };
    let part = &section.config.base;
    if part.hide_in_editor {
        runtime.note = Some((format!("Pick: {} is not in the catalogue", part.name), 2.5));
        return;
    }
    if runtime.held.as_deref() != Some(part.id.as_str()) {
        runtime.held = Some(part.id.clone());
        runtime.mount = MountPose::default();
        play_menu_select(&mut commands, bank.as_deref());
    }
}

/// Draw the solved placement: the part's block at its pose with a valid or
/// refused outline, and the callout with its price or refusal beside it.
#[expect(
    clippy::too_many_arguments,
    reason = "the ghost reconciles a block, its outline and a callout from one solve"
)]
#[expect(
    clippy::type_complexity,
    reason = "the ghost, its outline and the callout are each one filtered query"
)]
pub(crate) fn sync_ship_ghost(
    mut commands: Commands,
    runtime: Res<ShipRuntime>,
    preview: Res<ShipPlacementPreview>,
    sections: Option<Res<GameSections>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    mut q_ghost: Query<(Entity, &ShipGhostBlock, &mut Transform)>,
    mut q_outline: Query<(&ChildOf, &mut MeshMaterial3d<StandardMaterial>), With<ShipGhostOutline>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<ShipCameraMarker>>,
    q_viewport: Query<(Entity, &ComputedNode), With<ShipViewportMarker>>,
    mut q_callout: Query<(&mut Node, &mut Visibility, &Children), With<ShipGhostCallout>>,
    mut q_text: Query<(&mut Text, &mut ThemedText)>,
) {
    let placement = preview.placement.as_ref();
    let wanted = placement.and_then(|placement| {
        placement.pose.map(|pose| {
            (
                placement.prototype.as_str(),
                pose,
                placement.verdict.is_ok(),
            )
        })
    });
    let outline = |valid: bool| {
        if valid {
            runtime.mat_ghost_go.clone()
        } else {
            runtime.mat_ghost_no.clone()
        }
    };

    let mut kept = false;
    for (entity, ghost, mut transform) in &mut q_ghost {
        match wanted {
            Some((prototype, pose, valid)) if ghost.prototype == prototype && !kept => {
                if *transform != pose {
                    *transform = pose;
                }
                if let Some(material) = outline(valid) {
                    for (owner, mut current) in &mut q_outline {
                        if owner.parent() == entity && current.0 != material {
                            current.0 = material.clone();
                        }
                    }
                }
                kept = true;
            }
            _ => commands.entity(entity).despawn(),
        }
    }
    if let (false, Some((prototype, pose, valid)), Some(sections), Some(mut meshes), Some(root)) =
        (kept, wanted, sections, meshes, runtime.scene_root)
    {
        if let Some(offered) = sections.get_section(prototype) {
            let full = (offered
                .base
                .collider
                .unwrap_or_default()
                .aabb_half_extents()
                * 2.0)
                .max(Vec3::splat(0.2));
            let fill = full * SHIP_BLOCK_FILL_SCALE;
            let mut ghost = commands.spawn((
                ShipGhostBlock {
                    prototype: prototype.to_string(),
                },
                Name::new("ShipGhost"),
                Mesh3d(meshes.add(Cuboid::new(fill.x, fill.y, fill.z))),
                pose,
                RenderLayers::layer(SHIP_LAYER),
                ChildOf(root),
            ));
            if let Some(material) = runtime
                .mat_blocks
                .get(SectionIconType::of(offered.kind.class()) as usize)
            {
                ghost.insert(MeshMaterial3d(material.clone()));
            }
            if let (Some(edges), Some(material)) = (&runtime.edge_mesh, outline(valid)) {
                ghost.with_children(|ghost| {
                    ghost.spawn((
                        ShipGhostOutline,
                        Mesh3d(edges.clone()),
                        MeshMaterial3d(material),
                        Transform::from_scale(full),
                        RenderLayers::layer(SHIP_LAYER),
                    ));
                });
            }
        }
    }

    let Ok((viewport, node)) = q_viewport.single() else {
        return;
    };
    let Ok((mut callout, mut visibility, children)) = q_callout.single_mut() else {
        commands.spawn((
            ShipGhostCallout,
            Name::new(SHIP_GHOST_CALLOUT),
            Node {
                position_type: PositionType::Absolute,
                padding: UiRect::axes(Val::Px(6.0), Val::Px(3.0)),
                max_width: Val::Px(360.0),
                ..default()
            },
            BackgroundColor(Color::NONE),
            ThemedFill::alpha(UiColor::Void, 0.85),
            Visibility::Hidden,
            Pickable::IGNORE,
            // Above the section badges, its siblings, which would cover its
            // text on a dense hull.
            ZIndex(1),
            ChildOf(viewport),
            children![(themed_label("", 12.0, UiColor::Body), Pickable::IGNORE)],
        ));
        return;
    };
    let projected =
        placement
            .zip(q_camera.single().ok())
            .and_then(|(placement, (camera, camera_pose))| {
                let point = placement
                    .pose
                    .map_or(placement.anchor, |pose| pose.translation);
                let to_logical = node.inverse_scale_factor();
                camera
                    .world_to_viewport(camera_pose, point)
                    .ok()
                    .map(|p| (placement, p * to_logical))
            });
    let Some((placement, at)) = projected else {
        visibility.set_if_neq(Visibility::Hidden);
        return;
    };
    let left = Val::Px(at.x + CALLOUT_OFFSET_PX.x);
    let top = Val::Px(at.y + CALLOUT_OFFSET_PX.y);
    if callout.left != left || callout.top != top {
        callout.left = left;
        callout.top = top;
    }
    visibility.set_if_neq(Visibility::Inherited);
    let (line, color) = match &placement.verdict {
        Ok(line) => (line, UiColor::Nominal),
        Err(line) => (line, UiColor::Danger),
    };
    for child in children.iter() {
        if let Ok((mut text, mut themed)) = q_text.get_mut(child) {
            if text.0 != *line {
                text.0 = line.clone();
            }
            if themed.color != color {
                themed.color = color;
            }
        }
    }
}

/// Where ship-local point `point` shows in the window, in logical pixels,
/// while the Ship pane draws its view: the pixel a driven run points at to
/// aim the ghost there. `None` when the view or its camera is not up yet or
/// the point is behind the camera.
pub fn ship_view_window_px(world: &World, point: Vec3) -> Option<Vec2> {
    let mut cameras =
        world.try_query_filtered::<(&Camera, &GlobalTransform), With<ShipCameraMarker>>()?;
    let (camera, camera_pose) = cameras.single(world).ok()?;
    let mut viewports = world
        .try_query_filtered::<(&ComputedNode, &UiGlobalTransform), With<ShipViewportMarker>>()?;
    let (node, transform) = viewports.single(world).ok()?;
    let local = camera.world_to_viewport(camera_pose, point).ok()?;
    let min = transform.translation - node.size() * 0.5;
    Some((min + local) * node.inverse_scale_factor())
}

/// The placement the ghost shows this frame, while a part in hand is aimed
/// over the view: what a diagnostics overlay draws and prints.
pub fn ship_placement(world: &World) -> Option<&ShipPlacement> {
    world
        .get_resource::<ShipPlacementPreview>()?
        .placement
        .as_ref()
}

/// The section block under the pointer this frame, while the pointer is over
/// the view: what `ship_pick` takes the part of.
pub fn ship_hovered_section(world: &World) -> Option<Entity> {
    world.get_resource::<ShipPlacementPreview>()?.hovered
}

/// The design section id and socket id the ghost is aimed at, and whether a
/// click there would add, while a part is in hand over the view: what a
/// driven run checks before it clicks. `None` while no placement is aimed.
pub fn ship_ghost_aim(world: &World) -> Option<(String, String, bool)> {
    let placement = world
        .get_resource::<ShipPlacementPreview>()?
        .placement
        .as_ref()?;
    Some((
        placement.section.clone(),
        placement.point.clone(),
        placement.verdict.is_ok(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(entity: Entity, at: Vec3, half: Vec3) -> ShipSectionView {
        ShipSectionView {
            entity,
            code: String::new(),
            kind: SectionClass::Hull,
            name: String::new(),
            local: Transform::from_translation(at)
                .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
            half_extents: half,
            link_points: Vec::new(),
            health: None,
            ammo: None,
            bindings: None,
            inactive: false,
            zero_health: false,
            disabled: false,
        }
    }

    #[test]
    fn a_ray_picks_the_nearest_block_it_enters_on_its_rotated_box() {
        let near = Entity::from_raw_u32(1).expect("a valid index");
        let far = Entity::from_raw_u32(2).expect("a valid index");
        // A long box turned a quarter about Y: 4 long on X in its own frame,
        // so 4 deep on Z and 1 wide on X in the scene.
        let views = [
            block(far, Vec3::new(10.0, 0.0, 0.0), Vec3::new(4.0, 1.0, 1.0)),
            block(near, Vec3::ZERO, Vec3::new(4.0, 1.0, 1.0)),
        ];
        let along_x = Ray3d::new(Vec3::new(-20.0, 0.0, 0.0), Dir3::X);
        let (entity, hit) = pick_ship_block(along_x, &views).expect("the ray enters a block");
        assert_eq!(entity, near);
        assert!(hit.distance(Vec3::new(-1.0, 0.0, 0.0)) < 1e-4, "hit {hit}");

        let past_the_turned_box = Ray3d::new(Vec3::new(-20.0, 0.0, 3.0), Dir3::X);
        let (entity, hit) =
            pick_ship_block(past_the_turned_box, &views).expect("the ray enters a block");
        assert_eq!(entity, near, "the turned box reaches 4 along Z");
        assert!(hit.distance(Vec3::new(-1.0, 0.0, 3.0)) < 1e-4, "hit {hit}");

        let wide = Ray3d::new(Vec3::new(-20.0, 0.0, 6.0), Dir3::X);
        assert!(pick_ship_block(wide, &views).is_none());
    }

    fn cube_view(entity: Entity, at: Vec3, half: Vec3) -> ShipSectionView {
        ShipSectionView {
            local: Transform::from_translation(at),
            ..block(entity, at, half)
        }
    }

    fn part(id: &str, size: Vec3, link_points: Vec<LinkPoint>) -> SectionConfig {
        SectionConfig {
            base: BaseSectionConfig {
                id: id.to_string(),
                collider: Some(SectionCollider::Cuboid { size }),
                link_points,
                ..default()
            },
            kind: SectionKind::Hull(HullSectionConfig::default()),
        }
    }

    /// A cube closed in by five cubes and, on +X, a socketless tank smaller
    /// than a cell: a ray past the tank enters the inner cube, whose one free
    /// socket opens into the tank, so the ghost aims nowhere. A ray onto an
    /// outer cube aims at that cube's open face.
    #[test]
    fn a_ray_through_an_interior_section_never_aims_the_ghost_at_it() {
        let cube = part("cube", Vec3::ONE, unit_cube_link_points());
        let tank = part("tank", Vec3::splat(0.6), Vec::new());
        let at = [
            Vec3::ZERO,
            Vec3::NEG_X,
            Vec3::Y,
            Vec3::NEG_Y,
            Vec3::Z,
            Vec3::NEG_Z,
            Vec3::X,
        ];
        let design: Vec<RefitPlacedSection<'_>> = at
            .iter()
            .enumerate()
            .map(|(index, &position)| RefitPlacedSection {
                position,
                rotation: Quat::IDENTITY,
                config: if index == 6 { &tank } else { &cube },
            })
            .collect();
        let entity = |index: usize| Entity::from_raw_u32(index as u32 + 1).expect("a valid index");
        let views: Vec<ShipSectionView> = design
            .iter()
            .enumerate()
            .map(|(index, section)| {
                let half = if index == 6 { 0.3 } else { 0.5 };
                cube_view(entity(index), section.position, Vec3::splat(half))
            })
            .collect();

        let past_the_tank = Ray3d::new(Vec3::new(5.0, 0.4, 0.0), Dir3::NEG_X);
        let (inner, _) = pick_ship_block(past_the_tank, &views).expect("the ray enters a block");
        assert_eq!(inner, entity(0), "the ray slips past the tank");
        assert_eq!(free_link_points(&design, 0).len(), 1, "only +X is free");
        assert!(
            ghost_targets(&design, 0, inner, &views, &cube).is_empty(),
            "the inner cube is never a target"
        );

        let onto_the_top = Ray3d::new(Vec3::new(0.0, 5.0, 0.0), Dir3::NEG_Y);
        let (top, hit_top) = pick_ship_block(onto_the_top, &views).expect("the ray enters a block");
        assert_eq!(top, entity(2));
        let targets = ghost_targets(&design, 2, top, &views, &cube);
        let aimed = nearest_link_point(
            Vec3::Y,
            Quat::IDENTITY,
            &cube.base.link_points,
            targets,
            hit_top,
        )
        .expect("the top cube's open face is a target");
        assert_eq!(cube.base.link_points[aimed].id, "positive_y");
    }
}
