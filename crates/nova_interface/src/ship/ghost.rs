//! The Ship pane's placement ghost: the catalogue part in hand, shown as a
//! block at the free socket nearest the pointer, with its price or refusal
//! in a callout beside it, and the click that adds it.
//!
//! One solve a frame ([`update_ship_placement`]) feeds the ghost block, the
//! callout and the click, so a click adds what is on screen. The pointer's
//! ray picks the block under it ([`pick_ship_block`]); the nearest free socket
//! of that section ([`free_link_points`], [`nearest_link_point`]) is the
//! point; [`section_add_pose`] gives the ghost's pose and [`section_add_fit`]
//! with the payment and binding plan gives the verdict. A click on a valid
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
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_ui::{
    theme::UiColor,
    widget::{ThemedFill, ThemedText},
};

use super::{
    rebuild::StationSite,
    refit::{add_refusal_text, live_bindings, SectionAddCommand, SectionBindingRow},
    scene::{ShipCameraMarker, ShipRuntime, ShipViewportMarker, SHIP_BLOCK_FILL_SCALE},
    sections::{design_section_code, ShipSectionView, ShipSections},
    SHIP_LAYER,
};
use crate::{
    icons::SectionIconType,
    pane::{play_menu_select, themed_label},
};

/// What a click on the view would add, solved under the pointer.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ShipPlacement {
    /// The design section id the part mounts on.
    pub(crate) section: String,
    /// The socket id of that section the part mates.
    pub(crate) point: String,
    /// The catalogue part in hand.
    pub(crate) prototype: String,
    /// The part's pose in ship-local space, or `None` when it has no socket
    /// to mount by.
    pub(crate) pose: Option<Transform>,
    /// Where the callout points, in ship-local space.
    pub(crate) anchor: Vec3,
    /// The price line of a valid add, or the refusal line.
    pub(crate) verdict: Result<String, String>,
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

/// The fit of one part at one point, kept while the ship, the point and the
/// part stay the same: a fit reads the socket graph and the exit lanes of
/// the whole ship, too much to redo every frame.
#[derive(Default)]
pub(crate) struct GhostFitCache {
    key: Option<u64>,
    fit: Option<Result<(Vec3, Quat), SectionAddFaultType>>,
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
        let Some(placement) = &preview.placement else {
            return;
        };
        match &placement.verdict {
            Ok(_) => {
                adds.write(SectionAddCommand {
                    section: placement.section.clone(),
                    point: placement.point.clone(),
                    prototype: placement.prototype.clone(),
                });
                play_menu_select(&mut commands, bank.as_deref());
            }
            Err(refusal) => runtime.note = Some((refusal.clone(), 2.5)),
        }
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

/// The block `ray` enters first among `views`, and where it enters it, in
/// the scene's ship-local space. A block is its section's full collider box
/// at its local pose, the box its outline draws.
pub(crate) fn pick_ship_block(ray: Ray3d, views: &[ShipSectionView]) -> Option<(Entity, Vec3)> {
    views
        .iter()
        .filter_map(|view| {
            let to_local = view.local.compute_affine().inverse();
            let origin = to_local.transform_point3(ray.origin);
            let direction = Dir3::new(to_local.transform_vector3(*ray.direction)).ok()?;
            let half = view.half_extents.max(Vec3::splat(0.1));
            let distance = RayCast3d::new(origin, direction, f32::MAX)
                .aabb_intersection_at(&Aabb3d::new(Vec3::ZERO, half))?;
            let hit = view.local.transform_point(origin + *direction * distance);
            Some((view.entity, hit, ray.origin.distance_squared(hit)))
        })
        .min_by(|a, b| a.2.total_cmp(&b.2))
        .map(|(entity, hit, _)| (entity, hit))
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
    let hit = (|| {
        if !runtime.active {
            return None;
        }
        let pointer = runtime.pointer?;
        let (camera, camera_pose) = q_camera.single().ok()?;
        let (node, transform) = q_viewport.single().ok()?;
        // The camera re-derives its target size in `PostUpdate`; until it has
        // the view's size it unprojects into the old one.
        if camera.physical_target_size() != Some(node.size().round().as_uvec2()) {
            return None;
        }
        let ray = view_ray(pointer, camera, camera_pose, node, transform)?;
        pick_ship_block(ray, &views.collect())
    })();
    if let Some((entity, point)) = hit {
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
            let section = resolved.selected_section(Some(entity));
            let partner = resolved
                .partner
                .and_then(|(partner, _)| q_holds.get(partner).ok())
                .map(|(stock, credits)| (stock, credits.0));
            if let Some(section) = section {
                let code = design_section_code(resolved.design, &section.id)
                    .expect("a live design section has a code");
                let (placed, live) = resolved.refit_design();
                let index = resolved.design_index(section);
                let target = &placed[index];
                let sockets = &target.config.base.link_points;
                let free = nearest_link_point(
                    target.position,
                    target.rotation,
                    sockets,
                    free_link_points(&placed, index),
                    point,
                );
                placement = Some(match free {
                    None => ShipPlacement {
                        section: section.id.clone(),
                        point: String::new(),
                        prototype: prototype.clone(),
                        pose: None,
                        anchor: point,
                        verdict: Err(format!("add: {code} has no free point")),
                    },
                    Some(free) => {
                        let (anchor, _) =
                            link_point_pose(target.position, target.rotation, &sockets[free]);
                        let pose = section_add_pose(&placed, index, free, offered).ok().map(
                            |(position, rotation)| {
                                Transform::from_translation(position).with_rotation(rotation)
                            },
                        );
                        let mut hasher = DefaultHasher::new();
                        resolved.fit_key(section, free).hash(&mut hasher);
                        prototype.hash(&mut hasher);
                        let key = hasher.finish();
                        if cache.key != Some(key) {
                            cache.fit = Some(section_add_fit(&placed, &live, index, free, offered));
                            cache.key = Some(key);
                        }
                        let fit = cache.fit.clone().expect("the cache holds the keyed fit");
                        let (own, own_cr) = q_holds
                            .get(resolved.player)
                            .expect("the player ship carries a ShipInventory and ShipCredits");
                        let bound = live_bindings(&resolved, &q_bindings);
                        let name = &offered.base.name;
                        let verdict = match resolved
                            .plan_add(&items, offered, fit, &bound, own, own_cr.0, partner)
                        {
                            Ok(plan) => {
                                Ok(format!("{name} on {code}: {} cr", plan.payment.price_cr))
                            }
                            Err(refusal) => Err(add_refusal_text(&items, &code, name, &refusal)),
                        };
                        ShipPlacement {
                            section: section.id.clone(),
                            point: sockets[free].id.clone(),
                            prototype: prototype.clone(),
                            pose,
                            anchor,
                            verdict,
                        }
                    }
                });
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
}
