//! Rendered pictures for the Assets tiles.
//!
//! One asset at a time: a display copy of its sections stands on a layer of its
//! own, a camera waits for the meshes to stream in, frames what they measure,
//! takes the picture into an image and leaves. A copy that never measures is a
//! failure, which costs that tile its picture and nothing else: the tile keeps
//! its kind glyph.

use std::{
    collections::{hash_map::DefaultHasher, HashMap, HashSet, VecDeque},
    hash::{Hash, Hasher},
};

use bevy::{
    app::Propagate,
    camera::{primitives::Aabb, visibility::RenderLayers, ImageRenderTarget, RenderTarget},
    prelude::*,
};
use nova_gameplay::prelude::new_render_target_image;
use nova_scenario::prelude::{SectionSource, ShipDesign};
use nova_ship::prelude::{insert_preview_section, GameSections, PreviewRole, SectionConfig};

use crate::ExampleStates;

/// The layer the copy and its camera stand on. No other view reads it.
const THUMB_LAYER: usize = 27;

/// The picture's edge, in pixels: twice the tile's, so it holds at UI scale 2.
const THUMB_PX: u32 = 112;

/// Far from the range: a stage ray or a physics query never meets a copy.
const STUDIO: Vec3 = Vec3::new(0.0, -60_000.0, 0.0);

/// The same three-quarter look the gallery's tiles present.
const VIEW: Vec3 = Vec3::new(0.62, 0.42, 0.66);

/// How much of the frame the part leaves as margin.
const FIT: f32 = 1.12;

/// How many frames a measure must hold before the copy counts as loaded. A scene
/// streams in over several frames, and a ship's sections arrive one by one.
const STILL_FRAMES: u32 = 8;

/// The frames the camera runs before its picture is kept.
const SHOOT_FRAMES: u32 = 3;

/// The frames a copy gets to show anything at all.
const GIVE_UP_FRAMES: u32 = 240;

/// The sections of one picture, each where its ship puts it.
pub(crate) type ThumbnailParts = Vec<(SectionConfig, Transform)>;

/// What a picture is OF: changing the item changes the key, which retakes it.
pub(crate) fn thumbnail_key(id: &str, parts: &ThumbnailParts) -> u64 {
    let mut hasher = DefaultHasher::new();
    id.hash(&mut hasher);
    format!("{parts:?}").hash(&mut hasher);
    hasher.finish()
}

/// The parts of a design, placed. A prototype the catalog cannot find is left
/// out: lint reports it, and the picture shows the rest.
pub(crate) fn design_parts(design: &ShipDesign, catalog: &GameSections) -> ThumbnailParts {
    design
        .sections
        .iter()
        .filter_map(|placed| {
            let config = match &placed.source {
                SectionSource::Inline(config) => config.clone(),
                SectionSource::Prototype { id, .. } => catalog.get_section(id)?.clone(),
            };
            let at = Transform::from_translation(placed.position).with_rotation(placed.rotation);
            Some((config, at))
        })
        .collect()
}

/// The copy on the stage now.
struct Shoot {
    key: u64,
    root: Entity,
    camera: Entity,
    image: Handle<Image>,
    frames: u32,
    still: u32,
    measured: Option<(Vec3, f32)>,
    shooting: Option<u32>,
}

/// The pictures taken, and the ones still to take.
#[derive(Resource, Default)]
pub(crate) struct AssetThumbnails {
    ready: HashMap<u64, Handle<Image>>,
    failed: HashSet<u64>,
    queue: VecDeque<(u64, ThumbnailParts)>,
    shoot: Option<Shoot>,
}

impl AssetThumbnails {
    /// The picture under `key`, once it is taken.
    pub(crate) fn picture(&self, key: u64) -> Option<&Handle<Image>> {
        self.ready.get(&key)
    }

    /// Ask for a picture of `parts`, unless it was taken or tried.
    pub(crate) fn request(&mut self, key: u64, parts: ThumbnailParts) {
        let known = self.ready.contains_key(&key)
            || self.failed.contains(&key)
            || self.shoot.as_ref().is_some_and(|shoot| shoot.key == key)
            || self.queue.iter().any(|(queued, _)| *queued == key);
        if !known && !parts.is_empty() {
            self.queue.push_back((key, parts));
        }
    }

    /// Drop the work in hand. Its entities leave with the editor state.
    pub(crate) fn stop(&mut self) {
        self.queue.clear();
        self.shoot = None;
    }
}

/// The box around everything `root` draws, as a centre and a bounding radius.
fn measure(
    root: Entity,
    children: &Query<&Children>,
    bounds: &Query<(&Aabb, &GlobalTransform)>,
) -> Option<(Vec3, f32)> {
    let mut low = Vec3::splat(f32::MAX);
    let mut high = Vec3::splat(f32::MIN);
    let mut stack = vec![root];
    let mut found = false;
    while let Some(entity) = stack.pop() {
        if let Ok((aabb, transform)) = bounds.get(entity) {
            let (centre, half) = (Vec3::from(aabb.center), Vec3::from(aabb.half_extents));
            for corner in 0..8 {
                let sign = Vec3::new(
                    if corner & 1 == 0 { -1.0 } else { 1.0 },
                    if corner & 2 == 0 { -1.0 } else { 1.0 },
                    if corner & 4 == 0 { -1.0 } else { 1.0 },
                );
                let world = transform.transform_point(centre + half * sign);
                low = low.min(world);
                high = high.max(world);
            }
            found = true;
        }
        if let Ok(held) = children.get(entity) {
            stack.extend(held.iter());
        }
    }
    let radius = (high - low).length() / 2.0;
    (found && radius > f32::EPSILON).then(|| ((low + high) / 2.0, radius))
}

/// Stand the next copy up, and walk the one on stage through its frames.
pub(crate) fn drive_thumbnails(
    mut commands: Commands,
    mut thumbnails: ResMut<AssetThumbnails>,
    mut images: ResMut<Assets<Image>>,
    children: Query<&Children>,
    bounds: Query<(&Aabb, &GlobalTransform)>,
    mut cameras: Query<(&mut Camera, &mut Transform, &Projection)>,
) {
    let thumbnails = &mut *thumbnails;
    if thumbnails.shoot.is_none() {
        let Some((key, parts)) = thumbnails.queue.pop_front() else {
            return;
        };
        thumbnails.shoot = Some(stand_up(&mut commands, &mut images, key, &parts));
        return;
    }
    let Some(shoot) = &mut thumbnails.shoot else {
        return;
    };
    shoot.frames += 1;
    let finished = if let Some(left) = &mut shoot.shooting {
        *left = left.saturating_sub(1);
        *left == 0
    } else if shoot.frames > GIVE_UP_FRAMES {
        true
    } else {
        let now = measure(shoot.root, &children, &bounds);
        let held = matches!(
            (now, shoot.measured),
            (Some((_, new)), Some((_, old))) if (new - old).abs() <= old * 1e-3
        );
        shoot.still = if held { shoot.still + 1 } else { 0 };
        shoot.measured = now;
        if let (true, Some((centre, radius))) = (shoot.still >= STILL_FRAMES, now) {
            if let Ok((mut camera, mut transform, projection)) = cameras.get_mut(shoot.camera) {
                let fov = match projection {
                    Projection::Perspective(perspective) => perspective.fov,
                    _ => std::f32::consts::FRAC_PI_4,
                };
                let distance = radius * FIT / (fov / 2.0).tan();
                *transform = Transform::from_translation(centre + VIEW.normalize() * distance)
                    .looking_at(centre, Vec3::Y);
                camera.is_active = true;
            }
            shoot.shooting = Some(SHOOT_FRAMES);
        }
        false
    };
    if !finished {
        return;
    }
    let Some(done) = thumbnails.shoot.take() else {
        return;
    };
    commands.entity(done.root).despawn();
    commands.entity(done.camera).despawn();
    if done.shooting.is_some() {
        thumbnails.ready.insert(done.key, done.image);
    } else {
        thumbnails.failed.insert(done.key);
    }
}

/// Spawn the copy, its light, and its camera, which waits switched off.
fn stand_up(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    key: u64,
    parts: &ThumbnailParts,
) -> Shoot {
    let layers = RenderLayers::layer(THUMB_LAYER);
    let image = images.add(new_render_target_image(UVec2::splat(THUMB_PX)));
    let root = commands
        .spawn((
            Name::new("Thumbnail Copy"),
            Propagate(layers.clone()),
            layers.clone(),
            DespawnOnExit(ExampleStates::Editor),
            Transform::from_translation(STUDIO),
            Visibility::Visible,
        ))
        .id();
    for (section, at) in parts {
        let mut part = commands.spawn((
            *at,
            ChildOf(root),
            Pickable {
                should_block_lower: false,
                is_hoverable: false,
            },
        ));
        insert_preview_section(&mut part, section, PreviewRole::Display);
    }
    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::YXZ, 0.7, -0.8, 0.0)),
        layers.clone(),
        ChildOf(root),
    ));
    let camera = commands
        .spawn((
            Name::new("Thumbnail Camera"),
            Camera3d::default(),
            Camera {
                order: -20,
                clear_color: ClearColorConfig::Custom(Color::NONE),
                is_active: false,
                ..default()
            },
            RenderTarget::Image(ImageRenderTarget {
                handle: image.clone(),
                scale_factor: 1.0,
            }),
            AmbientLight {
                brightness: 300.0,
                ..default()
            },
            layers,
            DespawnOnExit(ExampleStates::Editor),
            Transform::from_translation(STUDIO),
        ))
        .id();
    Shoot {
        key,
        root,
        camera,
        image,
        frames: 0,
        still: 0,
        measured: None,
        shooting: None,
    }
}
