//! The 3D view on each "Ships for sale" card: the parked hull drawn as the
//! Ship pane draws a ship, one block per section tinted by its family and
//! framed by its outline, built from the hull's design.
//!
//! Each card's view has its own scene on its own render layer and its own
//! camera rendering into the card's image. The scene is made when the card
//! first lists the hull and despawned when the hull leaves the list or the
//! list goes. The cameras render only while the Station pane is on screen,
//! so a closed pane costs nothing.

use bevy::{
    camera::{visibility::RenderLayers, ImageRenderTarget, RenderTarget},
    prelude::*,
};
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_ui::theme::{ActiveUiTheme, UiColor};

use super::StationHullRows;
use crate::{
    icons::SectionIconType,
    pane::{interface_shown, InterfacePaneType},
    ship::{cuboid_edges, SHIP_BLOCK_FILL_SCALE, SHIP_RADIUS_MAX, SHIP_RADIUS_MIN},
    terminal::NovaOsCloseTransition,
    viewer::{orbit_eye, unlit},
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

/// Marks the image node on a hull card that shows the hull's view.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct HullPreviewSlot {
    /// The parked hull it shows.
    pub(crate) hull: Entity,
}

/// Marks a card view's camera.
#[derive(Component)]
pub(crate) struct HullPreviewCamera;

/// The card views alive, one per listed hull.
#[derive(Resource, Default)]
pub(crate) struct HullPreviews(Vec<HullPreview>);

/// One card view: the hull it shows, its slot, the scene root that owns
/// its blocks and camera, and the image the camera renders into.
struct HullPreview {
    hull: Entity,
    slot: usize,
    root: Entity,
    image: Handle<Image>,
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

/// Spawn the view of `sections` in `slot`: a block and an outline per
/// section and a camera rendering into a new image.
fn spawn_hull_preview(
    commands: &mut Commands,
    assets: (
        &mut Assets<Image>,
        &mut Assets<Mesh>,
        &mut Assets<StandardMaterial>,
    ),
    theme: &ActiveUiTheme,
    sections: &[DesignSection],
    slot: usize,
    active: bool,
) -> (Entity, Handle<Image>) {
    let (images, meshes, materials) = assets;
    let layer = RenderLayers::layer(HULL_PREVIEW_LAYER + slot);
    let image = images.add(new_render_target_image(HULL_PREVIEW_PX));
    let (centroid, radius) = hull_preview_framing(sections);
    let fills = SectionIconType::ALL
        .map(|icon| materials.add(unlit(theme.color_alpha(icon.color(), icon.block_alpha()))));
    let outline = materials.add(unlit(theme.color_alpha(UiColor::Primary, 0.95)));
    let cube = meshes.add(Cuboid::from_length(1.0));
    let edges = meshes.add(cuboid_edges());
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
        Transform::from_translation(
            centroid + orbit_eye(radius, HULL_PREVIEW_THETA, HULL_PREVIEW_PHI),
        )
        .looking_at(centroid, Vec3::Y),
        layer.clone(),
        ChildOf(root),
    ));
    for section in sections {
        let full = (collider_of(section) * 2.0).max(Vec3::splat(0.2));
        let fill = fills[SectionIconType::of(section.config.kind.class()) as usize].clone();
        commands
            .spawn((
                Mesh3d(cube.clone()),
                MeshMaterial3d(fill),
                Transform {
                    translation: section.position,
                    rotation: section.rotation,
                    scale: full * SHIP_BLOCK_FILL_SCALE,
                },
                layer.clone(),
                ChildOf(root),
            ))
            .with_children(|block| {
                block.spawn((
                    Mesh3d(edges.clone()),
                    MeshMaterial3d(outline.clone()),
                    // Undo the fill's shrink, so the outline frames the full box.
                    Transform::from_scale(Vec3::splat(SHIP_BLOCK_FILL_SCALE.recip())),
                    layer.clone(),
                ));
            });
    }
    (root, image)
}

/// Keep one view per hull the "Ships for sale" cards list: make the view of
/// a newly listed hull in the lowest free slot, despawn the view of a hull
/// no longer listed, hand each card's image node its view's image, and run
/// the cameras only while the Station pane is on screen.
#[expect(
    clippy::too_many_arguments,
    reason = "one system reading the cards, the parked hulls, the pane and the render assets"
)]
pub(crate) fn sync_hull_previews(
    mut commands: Commands,
    mut previews: ResMut<HullPreviews>,
    pause: Res<State<PauseStates>>,
    close: Res<NovaOsCloseTransition>,
    pane: Res<InterfacePaneType>,
    theme: Res<ActiveUiTheme>,
    assets: Option<(
        ResMut<Assets<Image>>,
        ResMut<Assets<Mesh>>,
        ResMut<Assets<StandardMaterial>>,
    )>,
    q_rows: Query<&StationHullRows>,
    q_designs: Query<&ShipDesignSections, With<ParkedHull>>,
    mut q_slots: Query<(&HullPreviewSlot, &mut ImageNode)>,
    mut q_cameras: Query<(&mut Camera, &ChildOf), With<HullPreviewCamera>>,
) {
    let Some((mut images, mut meshes, mut materials)) = assets else {
        return;
    };
    let listed: Vec<Entity> = q_rows.iter().flat_map(StationHullRows::hulls).collect();
    previews.0.retain(|preview| {
        let keep = listed.contains(&preview.hull);
        if !keep {
            commands.entity(preview.root).despawn();
        }
        keep
    });
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
        let (root, image) = spawn_hull_preview(
            &mut commands,
            (&mut images, &mut meshes, &mut materials),
            &theme,
            &design.0,
            slot,
            active,
        );
        previews.0.push(HullPreview {
            hull,
            slot,
            root,
            image,
        });
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
