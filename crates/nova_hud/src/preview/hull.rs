//! The 3D view of one hull, lit and slowly turning: the New Game role
//! step's hull and each "Ships for sale" or "Escorts for hire" card's.
//!
//! A view is a scene on its own render layer: a root holding a camera that
//! renders into the view's image, and a turntable whose model node centres
//! the hull. What hangs from the model node is the host's: the station cards
//! mirror a live hull's meshes there, the menu spawns a design's preview
//! sections. One key and one rim light per host, on every live view's layer
//! of that host and no other, light the hulls; the world's lights do not
//! reach them. Every turntable whose camera renders turns.

use bevy::{
    camera::{visibility::RenderLayers, ImageRenderTarget, RenderTarget},
    prelude::*,
};
use nova_gameplay::prelude::new_render_target_image;
use nova_ship::prelude::DesignSection;
use nova_ui::theme::{ActiveUiTheme, UiColor};

/// Glob-import surface: `use nova_hud::preview::hull::prelude::*`.
pub mod prelude {
    pub use super::{
        hull_preview_framing, orbit_eye, spawn_hull_preview_scene, sync_hull_preview_lights,
        HullPreviewCamera, HullPreviewLight, HullPreviewOwnerType, HullPreviewScene,
        HullPreviewTurntable, HULL_PREVIEW_PX, SHIP_RADIUS_MAX, SHIP_RADIUS_MIN,
    };
}

/// The size a card's image opens at, in pixels: the card's width and half
/// of it. A host that fits the view to its node resizes it.
pub const HULL_PREVIEW_PX: UVec2 = UVec2::new(320, 160);

/// Orbit-radius zoom clamp floor (world units from the centre): close enough
/// to read one section.
pub const SHIP_RADIUS_MIN: f32 = 3.0;
/// Orbit-radius zoom clamp ceiling: the fixed reach of a hull.
pub const SHIP_RADIUS_MAX: f32 = 400.0;

/// The camera's bearing about the hull and its height above it, radians:
/// the Ship pane's opening view, bow to the left.
const HULL_PREVIEW_THETA: f32 = 0.7;
const HULL_PREVIEW_PHI: f32 = 0.5;

/// The eye's distance from the hull's centre over the hull's reach. 2.6
/// fits the reach's whole sphere in the height of a square view, as the Ship
/// pane does; a hull fills well under that sphere, so the wide view comes in
/// closer.
const HULL_PREVIEW_FIT: f32 = 2.2;

/// How fast a hull turns about its up axis, radians per real second: one
/// turn in about half a minute.
const HULL_PREVIEW_TURN: f32 = 0.2;

/// The key and rim lights' bearings and illuminance (lux) and the cameras'
/// ambient brightness: the editor's part turntable rig, so a hull reads as
/// it does on the editor's stage.
const HULL_PREVIEW_KEY: (Vec3, f32) = (Vec3::new(-6.0, 8.0, 6.0), 9_000.0);
const HULL_PREVIEW_RIM: (Vec3, f32) = (Vec3::new(5.0, -2.0, -7.0), 2_500.0);
const HULL_PREVIEW_AMBIENT: f32 = 220.0;

/// The camera eye offset from the focus for a given orbit, on a Y-up sphere.
pub fn orbit_eye(radius: f32, theta: f32, phi: f32) -> Vec3 {
    let horizontal = radius * phi.cos();
    Vec3::new(
        horizontal * theta.sin(),
        radius * phi.sin(),
        horizontal * theta.cos(),
    )
}

/// Which host a view's lights belong to. Each host keeps its own pair on the
/// union of its live views' layers, so the menu's views and the station
/// cards' never relight each other.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HullPreviewOwnerType {
    /// The Station pane's hull cards.
    Station,
    /// The main menu's New Game role step.
    Menu,
}

/// Marks a view's camera.
#[derive(Component)]
pub struct HullPreviewCamera;

/// Marks a view's turntable: it turns while the view's camera renders.
#[derive(Component)]
pub struct HullPreviewTurntable;

/// Marks the lights that light one host's views.
#[derive(Component)]
pub struct HullPreviewLight(pub HullPreviewOwnerType);

/// One hull view: the scene root that owns its camera and turntable, the
/// model node the hull hangs from, and the image the camera renders into.
#[derive(Clone, Debug)]
pub struct HullPreviewScene {
    /// The root, whose despawn takes the whole scene.
    pub root: Entity,
    /// The camera, a child of the root.
    pub camera: Entity,
    /// The node that turns.
    pub turntable: Entity,
    /// The node the hull's meshes hang from, at the hull's centre.
    pub model: Entity,
    /// The camera's target.
    pub image: Handle<Image>,
}

/// Where a hull's view looks from and at, in the hull's own frame: the
/// centre of its sections and an eye far enough out to fit the furthest
/// one, as the Ship pane frames a ship.
pub fn hull_preview_framing(sections: &[DesignSection]) -> (Vec3, f32) {
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

/// Spawn the view of a hull of `sections` on `layer`: a root holding the
/// camera at `order`, which renders into a new image, and a turntable whose
/// model node centres the hull. The hull's meshes are the host's to hang
/// from the model node.
pub fn spawn_hull_preview_scene(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    theme: &ActiveUiTheme,
    sections: &[DesignSection],
    layer: RenderLayers,
    order: isize,
    active: bool,
) -> HullPreviewScene {
    let image = images.add(new_render_target_image(HULL_PREVIEW_PX));
    let (centroid, radius) = hull_preview_framing(sections);
    let root = commands
        .spawn((
            Name::new("HullPreview"),
            Transform::default(),
            Visibility::Visible,
        ))
        .id();
    let camera = commands
        .spawn((
            HullPreviewCamera,
            Camera3d::default(),
            Camera {
                order,
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
        ))
        .id();
    let turntable = commands
        .spawn((
            HullPreviewTurntable,
            Transform::default(),
            Visibility::Inherited,
            ChildOf(root),
        ))
        .id();
    let model = commands
        .spawn((
            Transform::from_translation(-centroid),
            Visibility::Inherited,
            ChildOf(turntable),
        ))
        .id();
    HullPreviewScene {
        root,
        camera,
        turntable,
        model,
        image,
    }
}

/// Put `owner`'s key and rim lights on `layers`, the union of its live
/// views' layers: spawn them with the first view, despawn them with the
/// last (`layers` empty).
pub fn sync_hull_preview_lights(
    commands: &mut Commands,
    owner: HullPreviewOwnerType,
    layers: &RenderLayers,
    q_lights: &mut Query<(Entity, &HullPreviewLight, &mut RenderLayers)>,
) {
    let mut owned = q_lights
        .iter_mut()
        .filter(|(_, light, _)| light.0 == owner)
        .peekable();
    if *layers == RenderLayers::none() {
        for (light, _, _) in owned {
            commands.entity(light).despawn();
        }
        return;
    }
    if owned.peek().is_none() {
        for (name, (bearing, illuminance)) in [
            ("HullPreviewKeyLight", HULL_PREVIEW_KEY),
            ("HullPreviewRimLight", HULL_PREVIEW_RIM),
        ] {
            commands.spawn((
                Name::new(name),
                HullPreviewLight(owner),
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
    for (_, _, mut light_layers) in owned {
        if *light_layers != *layers {
            *light_layers = layers.clone();
        }
    }
}

/// Turn every turntable whose view's camera renders, by real time: the
/// interface and the menu both hold the virtual clock.
pub(crate) fn turn_hull_previews(
    time: Res<Time<Real>>,
    q_cameras: Query<(&Camera, &ChildOf), With<HullPreviewCamera>>,
    mut q_turntables: Query<(&mut Transform, &ChildOf), With<HullPreviewTurntable>>,
) {
    let turn = Quat::from_rotation_y(HULL_PREVIEW_TURN * time.delta_secs());
    for (mut turntable, child_of) in &mut q_turntables {
        let rendering = q_cameras
            .iter()
            .any(|(camera, parent)| camera.is_active && parent.parent() == child_of.parent());
        if rendering {
            turntable.rotate(turn);
        }
    }
}
