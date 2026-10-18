//! The 3D view on each "Ships for sale" and "Escorts for hire" card: the
//! listed hull's own model, plating and greebles, lit and slowly turning.
//!
//! The hull itself is a real ship docked at the station or coasting beside
//! it, on the world's render layer. Each card's view is a shared hull view
//! (`nova_hud::preview::hull`) on its own render layer, holding a mirror of
//! every mesh the hull draws: the same mesh and material handles posed in
//! the hull's frame ([`mirror_hull_previews`]). The scene is made when a
//! card first lists the hull and despawned when the hull leaves the lists.
//! The cameras render, and the mirrors follow, only while the Station pane
//! is on screen, so a closed pane costs nothing.

use bevy::{
    camera::visibility::RenderLayers,
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use nova_gameplay::prelude::*;
pub(crate) use nova_hud::prelude::HULL_PREVIEW_PX;
use nova_hud::prelude::{
    spawn_hull_preview_scene, sync_hull_preview_lights, HullPreviewCamera, HullPreviewLight,
    HullPreviewOwnerType, HullPreviewScene,
};
use nova_ship::prelude::*;
use nova_ui::theme::ActiveUiTheme;

use super::{StationEscortRows, StationHullRows};
use crate::{
    pane::{interface_shown, InterfacePaneType},
    terminal::NovaOsCloseTransition,
};

/// The render layer of the first card's scene; card slot `n` draws on
/// `HULL_PREVIEW_LAYER + n`, so no camera sees another card's hull. Above
/// the interface's own layers (20 to 22) and the menu's (23).
const HULL_PREVIEW_LAYER: usize = 26;

/// The first card's camera order; slot `n` renders at
/// `HULL_PREVIEW_CAMERA_ORDER - n`. Before the interface's render-to-texture
/// pass (-20), like the Ship pane's camera (-31) and the map's (-30).
const HULL_PREVIEW_CAMERA_ORDER: isize = -40;

/// Marks the image node on a hull card that shows the hull's view.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct HullPreviewSlot {
    /// The listed hull it shows.
    pub(crate) hull: Entity,
}

/// The card views alive, one per listed hull.
#[derive(Resource, Default)]
pub(crate) struct HullPreviews(Vec<HullPreview>);

/// One card view: the hull it shows, its slot, its scene and each mirror by
/// the hull mesh it copies.
struct HullPreview {
    hull: Entity,
    slot: usize,
    scene: HullPreviewScene,
    mirrors: HashMap<Entity, Entity>,
}

/// The render layer card slot `slot` draws on.
fn preview_layer(slot: usize) -> RenderLayers {
    RenderLayers::layer(HULL_PREVIEW_LAYER + slot)
}

/// Keep one view per hull the cards list: make the view of a newly listed
/// hull in the lowest free slot, despawn the view of a hull no longer
/// listed, keep the lights on every live slot's layer, hand each card's
/// image node its view's image, and run the cameras only while the Station
/// pane is on screen.
#[expect(
    clippy::too_many_arguments,
    reason = "one system reading the cards, the listed hulls, the pane and the render assets"
)]
pub(crate) fn sync_hull_previews(
    mut commands: Commands,
    mut previews: ResMut<HullPreviews>,
    pause: Res<State<PauseStates>>,
    close: Res<NovaOsCloseTransition>,
    pane: Res<InterfacePaneType>,
    theme: Res<ActiveUiTheme>,
    images: Option<ResMut<Assets<Image>>>,
    q_hull_rows: Query<&StationHullRows>,
    q_escort_rows: Query<&StationEscortRows>,
    q_designs: Query<&ShipDesignSections, With<ParkedHull>>,
    mut q_slots: Query<(&HullPreviewSlot, &mut ImageNode)>,
    mut q_cameras: Query<(&mut Camera, &ChildOf), With<HullPreviewCamera>>,
    mut q_lights: Query<(Entity, &HullPreviewLight, &mut RenderLayers)>,
) {
    let Some(mut images) = images else {
        return;
    };
    let listed: Vec<Entity> = q_hull_rows
        .iter()
        .flat_map(StationHullRows::hulls)
        .chain(q_escort_rows.iter().flat_map(StationEscortRows::hulls))
        .collect();
    let before = previews.0.len();
    previews.0.retain(|preview| {
        let keep = listed.contains(&preview.hull);
        if !keep {
            commands.entity(preview.scene.root).despawn();
        }
        keep
    });
    let mut changed = previews.0.len() != before;
    let active = interface_shown(&pause, &close) && *pane == InterfacePaneType::Station;
    for hull in listed {
        if previews.0.iter().any(|preview| preview.hull == hull) {
            continue;
        }
        let Ok(design) = q_designs.get(hull) else {
            continue;
        };
        let slot = (0..)
            .find(|slot| previews.0.iter().all(|preview| preview.slot != *slot))
            .expect("an unbounded range holds a free slot");
        let scene = spawn_hull_preview_scene(
            &mut commands,
            &mut images,
            &theme,
            &design.0,
            preview_layer(slot),
            HULL_PREVIEW_CAMERA_ORDER - slot as isize,
            active,
        );
        previews.0.push(HullPreview {
            hull,
            slot,
            scene,
            mirrors: HashMap::new(),
        });
        changed = true;
    }
    if changed {
        let layers = previews
            .0
            .iter()
            .fold(RenderLayers::none(), |layers, preview| {
                layers.union(&preview_layer(preview.slot))
            });
        sync_hull_preview_lights(
            &mut commands,
            HullPreviewOwnerType::Station,
            &layers,
            &mut q_lights,
        );
    }
    for (slot, mut node) in &mut q_slots {
        if let Some(preview) = previews.0.iter().find(|preview| preview.hull == slot.hull) {
            if node.image != preview.scene.image {
                node.image = preview.scene.image.clone();
            }
        }
    }
    // This pane's cameras only: the main menu's role step turns a hull view
    // of its own.
    for (mut camera, child_of) in &mut q_cameras {
        let owned = previews
            .0
            .iter()
            .any(|preview| preview.scene.root == child_of.parent());
        if owned && camera.is_active != active {
            camera.is_active = active;
        }
    }
}

/// The world meshes a hull draws: each descendant with a mesh, a lit
/// material and no render layer of its own, visible in the world.
type HullMeshQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Mesh3d,
        &'static GlobalTransform,
        &'static InheritedVisibility,
        Option<&'static MeshMaterial3d<StandardMaterial>>,
        Option<&'static MeshMaterial3d<SectionCracksMaterial>>,
    ),
    (Without<RenderLayers>, Without<HullPreviewMirror>),
>;

/// Marks a mirror mesh in a card's scene.
#[derive(Component)]
pub(crate) struct HullPreviewMirror;

/// While the Station pane is on screen, keep each card's mirrors on its
/// hull: a mirror per mesh the hull draws, the same mesh and material,
/// posed where the mesh sits in the hull's frame; a mirror whose mesh is
/// gone or hidden goes. The shared preview plugin turns the turntables.
///
/// A mesh with neither a `StandardMaterial` nor a cracks material, such as a
/// thruster plume, is not mirrored: a card shows the hull at rest.
#[expect(
    clippy::type_complexity,
    reason = "the mirror's mesh, materials and pose, read and written"
)]
pub(crate) fn mirror_hull_previews(
    mut commands: Commands,
    mut previews: ResMut<HullPreviews>,
    q_cameras: Query<&Camera, With<HullPreviewCamera>>,
    q_hulls: Query<&GlobalTransform>,
    q_children: Query<&Children>,
    q_meshes: HullMeshQuery,
    mut q_mirrors: Query<
        (
            &mut Mesh3d,
            &mut Transform,
            Option<&mut MeshMaterial3d<StandardMaterial>>,
            Option<&mut MeshMaterial3d<SectionCracksMaterial>>,
        ),
        With<HullPreviewMirror>,
    >,
) {
    if !q_cameras.iter().any(|camera| camera.is_active) {
        return;
    }
    for preview in &mut previews.0 {
        let Ok(hull_frame) = q_hulls.get(preview.hull) else {
            continue;
        };
        let to_hull = hull_frame.affine().inverse();
        let mut seen = HashSet::new();
        for mesh in q_children.iter_descendants(preview.hull) {
            let Ok((handle, at, visible, standard, cracks)) = q_meshes.get(mesh) else {
                continue;
            };
            if !visible.get() || (standard.is_none() && cracks.is_none()) {
                continue;
            }
            seen.insert(mesh);
            let pose = Transform::from_matrix(Mat4::from(to_hull * at.affine()));
            if let Some(mirror) = preview.mirrors.get(&mesh) {
                if let Ok((mut mirror_mesh, mut mirror_pose, mirror_standard, mirror_cracks)) =
                    q_mirrors.get_mut(*mirror)
                {
                    let same_material = match (standard, cracks) {
                        (Some(standard), _) => mirror_standard.is_some_and(|m| m.0 == standard.0),
                        (None, Some(cracks)) => mirror_cracks.is_some_and(|m| m.0 == cracks.0),
                        (None, None) => unreachable!("an unlit mesh is skipped above"),
                    };
                    if same_material {
                        if mirror_mesh.0 != handle.0 {
                            mirror_mesh.0 = handle.0.clone();
                        }
                        if *mirror_pose != pose {
                            *mirror_pose = pose;
                        }
                        continue;
                    }
                }
                commands.entity(*mirror).despawn();
            }
            let mut mirror = commands.spawn((
                HullPreviewMirror,
                Mesh3d(handle.0.clone()),
                pose,
                preview_layer(preview.slot),
                ChildOf(preview.scene.model),
            ));
            match (standard, cracks) {
                (Some(standard), _) => mirror.insert(standard.clone()),
                (None, Some(cracks)) => mirror.insert(cracks.clone()),
                (None, None) => unreachable!("an unlit mesh is skipped above"),
            };
            preview.mirrors.insert(mesh, mirror.id());
        }
        preview.mirrors.retain(|mesh, mirror| {
            let keep = seen.contains(mesh);
            if !keep {
                commands.entity(*mirror).despawn();
            }
            keep
        });
    }
}
