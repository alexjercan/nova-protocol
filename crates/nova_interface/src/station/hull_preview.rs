//! The 3D view on each "Ships for sale" and "Escorts for hire" card: the
//! listed hull's own model, plating and greebles, lit and slowly turning.
//!
//! The hull itself is a real ship docked at the station or coasting beside
//! it, on the world's render layer. Each card's view has its own scene on
//! its own render layer: a mirror of every mesh the hull draws, the same
//! mesh and material handles posed in the hull's frame
//! ([`mirror_hull_previews`]), and a camera rendering into the card's image.
//! One key and one rim light, on every card's layer and no other, light the
//! mirrors; the world's lights do not reach them. The scene is made when a
//! card first lists the hull and despawned when the hull leaves the lists.
//! The cameras render, and the mirrors follow and turn, only while the
//! Station pane is on screen, so a closed pane costs nothing.

use bevy::{
    camera::{visibility::RenderLayers, ImageRenderTarget, RenderTarget},
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_ui::theme::{ActiveUiTheme, UiColor};

use super::{StationEscortRows, StationHullRows};
use crate::{
    pane::{interface_shown, InterfacePaneType},
    ship::{SHIP_RADIUS_MAX, SHIP_RADIUS_MIN},
    terminal::NovaOsCloseTransition,
    viewer::orbit_eye,
};

/// The render layer of the first card's scene; card slot `n` draws on
/// `HULL_PREVIEW_LAYER + n`, so no camera sees another card's hull. Above
/// the interface's own layers (20 to 22) and the menu's (23).
const HULL_PREVIEW_LAYER: usize = 26;

/// The first card's camera order; slot `n` renders at
/// `HULL_PREVIEW_CAMERA_ORDER - n`. Before the interface's render-to-texture
/// pass (-20), like the Ship pane's camera (-31) and the map's (-30).
const HULL_PREVIEW_CAMERA_ORDER: isize = -40;

/// The size of a card's image, in pixels: the card's width and half of it.
pub(crate) const HULL_PREVIEW_PX: UVec2 = UVec2::new(320, 160);

/// The camera's bearing about the hull and its height above it, radians:
/// the Ship pane's opening view, bow to the left.
const HULL_PREVIEW_THETA: f32 = 0.7;
const HULL_PREVIEW_PHI: f32 = 0.5;

/// The eye's distance from the hull's centre over the hull's reach. 2.6
/// fits the reach's whole sphere in the height of a square view, as the Ship
/// pane does; a hull fills well under that sphere, so the wide card comes in
/// closer.
const HULL_PREVIEW_FIT: f32 = 2.2;

/// How fast a card's hull turns about its up axis, radians per real second:
/// one turn in about half a minute.
const HULL_PREVIEW_TURN: f32 = 0.2;

/// The key and rim lights' bearings and illuminance (lux) and the cameras'
/// ambient brightness: the editor's part turntable rig, so a card's hull
/// reads as it does on the editor's stage.
const HULL_PREVIEW_KEY: (Vec3, f32) = (Vec3::new(-6.0, 8.0, 6.0), 9_000.0);
const HULL_PREVIEW_RIM: (Vec3, f32) = (Vec3::new(5.0, -2.0, -7.0), 2_500.0);
const HULL_PREVIEW_AMBIENT: f32 = 220.0;

/// Marks the image node on a hull card that shows the hull's view.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct HullPreviewSlot {
    /// The listed hull it shows.
    pub(crate) hull: Entity,
}

/// Marks a card view's camera.
#[derive(Component)]
pub(crate) struct HullPreviewCamera;

/// Marks the lights that light every card's scene.
#[derive(Component)]
pub(crate) struct HullPreviewLight;

/// The card views alive, one per listed hull.
#[derive(Resource, Default)]
pub(crate) struct HullPreviews(Vec<HullPreview>);

/// One card view: the hull it shows, its slot, the scene root that owns
/// its camera and turntable, the node the mirrors hang from, the image the
/// camera renders into, and each mirror by the hull mesh it copies.
struct HullPreview {
    hull: Entity,
    slot: usize,
    root: Entity,
    turntable: Entity,
    model: Entity,
    image: Handle<Image>,
    mirrors: HashMap<Entity, Entity>,
}

/// Where a hull's view looks from and at, in the hull's own frame: the
/// centre of its sections and an eye far enough out to fit the furthest
/// one, as the Ship pane frames a ship.
fn hull_preview_framing(sections: &[DesignSection]) -> (Vec3, f32) {
    let centroid = if sections.is_empty() {
        Vec3::ZERO
    } else {
        sections
            .iter()
            .map(|section| section.position)
            .sum::<Vec3>()
            / sections.len() as f32
    };
    let extent = sections
        .iter()
        .map(|section| (section.position - centroid).length() + collider_of(section).max_element())
        .fold(2.0_f32, f32::max);
    (
        centroid,
        (extent * HULL_PREVIEW_FIT).clamp(SHIP_RADIUS_MIN, SHIP_RADIUS_MAX),
    )
}

/// Half the section's authored collider box, in its own frame.
fn collider_of(section: &DesignSection) -> Vec3 {
    section
        .config
        .base
        .collider
        .unwrap_or_default()
        .aabb_half_extents()
}

/// Spawn the scene of `slot` for a hull of `sections`: a root holding the
/// camera, which renders into a new image, and a turntable whose model node
/// centres the hull. The mirrors come later, from [`mirror_hull_previews`].
fn spawn_hull_preview(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    theme: &ActiveUiTheme,
    sections: &[DesignSection],
    slot: usize,
    active: bool,
) -> (Entity, Entity, Entity, Handle<Image>) {
    let layer = preview_layer(slot);
    let image = images.add(new_render_target_image(HULL_PREVIEW_PX));
    let (centroid, radius) = hull_preview_framing(sections);
    let root = commands
        .spawn((
            Name::new(format!("HullPreview-{slot}")),
            Transform::default(),
            Visibility::Visible,
        ))
        .id();
    commands.spawn((
        HullPreviewCamera,
        Camera3d::default(),
        Camera {
            order: HULL_PREVIEW_CAMERA_ORDER - slot as isize,
            clear_color: ClearColorConfig::Custom(theme.color(UiColor::Surface)),
            is_active: active,
            ..default()
        },
        RenderTarget::Image(ImageRenderTarget {
            handle: image.clone(),
            scale_factor: 1.0,
        }),
        AmbientLight {
            brightness: HULL_PREVIEW_AMBIENT,
            ..default()
        },
        Transform::from_translation(orbit_eye(radius, HULL_PREVIEW_THETA, HULL_PREVIEW_PHI))
            .looking_at(Vec3::ZERO, Vec3::Y),
        layer,
        ChildOf(root),
    ));
    let turntable = commands
        .spawn((Transform::default(), Visibility::Inherited, ChildOf(root)))
        .id();
    let model = commands
        .spawn((
            Transform::from_translation(-centroid),
            Visibility::Inherited,
            ChildOf(turntable),
        ))
        .id();
    (root, turntable, model, image)
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
    mut q_lights: Query<(Entity, &mut RenderLayers), With<HullPreviewLight>>,
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
            commands.entity(preview.root).despawn();
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
        let (root, turntable, model, image) =
            spawn_hull_preview(&mut commands, &mut images, &theme, &design.0, slot, active);
        previews.0.push(HullPreview {
            hull,
            slot,
            root,
            turntable,
            model,
            image,
            mirrors: HashMap::new(),
        });
        changed = true;
    }
    if changed {
        sync_preview_lights(&mut commands, &previews, &mut q_lights);
    }
    for (slot, mut node) in &mut q_slots {
        if let Some(preview) = previews.0.iter().find(|preview| preview.hull == slot.hull) {
            if node.image != preview.image {
                node.image = preview.image.clone();
            }
        }
    }
    for (mut camera, child_of) in &mut q_cameras {
        let owned = previews
            .0
            .iter()
            .any(|preview| preview.root == child_of.parent());
        if camera.is_active != (active && owned) {
            camera.is_active = active && owned;
        }
    }
}

/// Put the key and rim lights on every live slot's layer: spawn them with
/// the first view, despawn them with the last.
fn sync_preview_lights(
    commands: &mut Commands,
    previews: &HullPreviews,
    q_lights: &mut Query<(Entity, &mut RenderLayers), With<HullPreviewLight>>,
) {
    if previews.0.is_empty() {
        for (light, _) in q_lights.iter() {
            commands.entity(light).despawn();
        }
        return;
    }
    let layers = previews
        .0
        .iter()
        .fold(RenderLayers::none(), |layers, preview| {
            layers.union(&preview_layer(preview.slot))
        });
    if q_lights.is_empty() {
        for (name, (bearing, illuminance)) in [
            ("HullPreviewKeyLight", HULL_PREVIEW_KEY),
            ("HullPreviewRimLight", HULL_PREVIEW_RIM),
        ] {
            commands.spawn((
                Name::new(name),
                HullPreviewLight,
                DirectionalLight {
                    illuminance,
                    shadow_maps_enabled: false,
                    ..default()
                },
                Transform::from_translation(bearing).looking_at(Vec3::ZERO, Vec3::Y),
                layers.clone(),
            ));
        }
        return;
    }
    for (_, mut light_layers) in q_lights.iter_mut() {
        if *light_layers != layers {
            *light_layers = layers.clone();
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
/// gone or hidden goes. Then turn each turntable.
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
    time: Res<Time<Real>>,
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
    mut q_turntables: Query<&mut Transform, Without<HullPreviewMirror>>,
) {
    if !q_cameras.iter().any(|camera| camera.is_active) {
        return;
    }
    let turn = Quat::from_rotation_y(HULL_PREVIEW_TURN * time.delta_secs());
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
                ChildOf(preview.model),
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
        if let Ok(mut turntable) = q_turntables.get_mut(preview.turntable) {
            turntable.rotate(turn);
        }
    }
}
