//! The texture-compare kit shared by the `compare_*` producers: candidate
//! textures loaded straight off the repo's `art/texture-candidates/` (repeat
//! sampler, so UVs past 1.0 tile instead of clamp-smearing), the focus-swap
//! roster the number keys drive, and the screen-projected subject labels.
//!
//! Candidates load from disk with [`Image::from_buffer`] rather than through
//! the `AssetServer` for two reasons: `art/` is not an asset source (and must
//! not become one - `assets/` ships wholesale into every build, comparison
//! textures must not), and a disk load is the only path that sets the repeat
//! sampler per image without `.meta` sidecars. Included with
//! `#[path = "shared/compare.rs"] mod compare;` - one level down so
//! `catalog_matches_disk` does not read it as an example root (same rule as
//! `screenshots/shared/kit.rs`).

// Each producer includes the whole kit; what one leaves unused the other needs.
#![allow(
    dead_code,
    reason = "one source, many example targets: what one producer leaves unused another needs, so no single build can fulfil an expectation"
)]

use bevy::{
    asset::RenderAssetUsages,
    image::{
        CompressedImageFormats, ImageAddressMode, ImageSampler, ImageSamplerDescriptor, ImageType,
    },
    prelude::*,
};
use nova_protocol::prelude::*;

/// One texture on the compare roster: the on-screen label and the image the
/// focus subject swaps to.
pub struct CompareEntry {
    /// Human-readable name, shown on the subject label and the focus readout.
    pub label: String,
    /// The candidate image (disk-loaded or a `GameAssets` handle).
    pub image: Handle<Image>,
}

/// The candidate roster and which entry the focus subject currently wears.
/// Number keys and arrows write `selected`; [`apply_selection`] reacts to the
/// change, so a script that sets `selected` directly exercises the same path.
#[derive(Resource)]
pub struct CompareRoster {
    /// Roster entries, in key order (`Digit1` selects index 0).
    pub entries: Vec<CompareEntry>,
    /// Index of the entry on the focus subject.
    pub selected: usize,
}

impl CompareRoster {
    /// A roster starting on its first entry.
    pub fn new(entries: Vec<CompareEntry>) -> Self {
        Self {
            entries,
            selected: 0,
        }
    }

    /// The entry the focus subject wears.
    pub fn selected_entry(&self) -> &CompareEntry {
        &self.entries[self.selected]
    }
}

/// Marks the focus subject: the center mesh whose material the keys retexture.
#[derive(Component)]
pub struct FocusSubject;

/// A screen-space label pinned over a world subject (projected every frame, so
/// it survives a WASD free-fly).
#[derive(Component)]
pub struct SubjectLabel {
    /// The world entity the label hovers over.
    pub target: Entity,
    /// World-space offset from the target (roughly its top).
    pub offset: Vec3,
}

/// Marks the top-center readout naming the focus texture.
#[derive(Component)]
pub struct FocusReadout;

/// Fixed label width (px); labels center on the projected point.
const LABEL_WIDTH: f32 = 220.0;

/// A sampler that tiles in every direction. The game default (bevy's
/// `ClampToEdge`) smears the edge texel across every noise-stretched triangle
/// whose planar UVs run past 1.0 - the round-2 sampler trap; candidates load
/// with THIS so the comparison shows the texture, not the trap. The baseline
/// entry keeps the shipped clamp behaviour for contrast.
pub fn repeat_sampler() -> ImageSampler {
    ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        address_mode_w: ImageAddressMode::Repeat,
        ..default()
    })
}

/// Load a repo file (path relative to the repo root) as a repeat-sampled sRGB
/// image. Panics on a missing or undecodable file - a compare with a missing
/// candidate is a broken comparison, not something to render around.
pub fn load_candidate(images: &mut Assets<Image>, repo_path: &str) -> Handle<Image> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(repo_path);
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|error| panic!("compare: read {}: {error}", path.display()));
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("png");
    let image = Image::from_buffer(
        &bytes,
        ImageType::Extension(extension),
        CompressedImageFormats::NONE,
        true,
        repeat_sampler(),
        RenderAssetUsages::default(),
    )
    .unwrap_or_else(|error| panic!("compare: decode {}: {error}", path.display()));
    images.add(image)
}

/// Spawn the projected label for one subject.
pub fn spawn_subject_label(commands: &mut Commands, target: Entity, text: &str, offset: Vec3) {
    commands.spawn((
        SubjectLabel { target, offset },
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(13.0),
            ..default()
        },
        TextColor(Color::WHITE),
        TextLayout::justify(Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(LABEL_WIDTH),
            ..default()
        },
    ));
}

/// Spawn the top-center focus readout (filled by [`apply_selection`]).
pub fn spawn_focus_readout(commands: &mut Commands) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                FocusReadout,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(20.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

/// `Digit1`..`Digit9`, in roster order.
const DIGIT_KEYS: [KeyCode; 9] = [
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
];

/// Number keys pick a roster entry for the focus subject; left/right arrows
/// cycle (WASD stays free for the scenario camera).
pub fn select_by_keys(keyboard: Res<ButtonInput<KeyCode>>, mut roster: ResMut<CompareRoster>) {
    let count = roster.entries.len();
    if count == 0 {
        return;
    }
    for (index, key) in DIGIT_KEYS.iter().enumerate().take(count) {
        if keyboard.just_pressed(*key) {
            roster.selected = index;
        }
    }
    if keyboard.just_pressed(KeyCode::ArrowRight) {
        roster.selected = (roster.selected + 1) % count;
    }
    if keyboard.just_pressed(KeyCode::ArrowLeft) {
        roster.selected = (roster.selected + count - 1) % count;
    }
}

/// Apply a roster change: retexture the focus subject's material and rewrite
/// the readout. Runs on `CompareRoster` change detection, which includes the
/// insert frame - so the focus starts dressed without a separate init path.
pub fn apply_selection(
    roster: Res<CompareRoster>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    q_focus: Query<&MeshMaterial3d<StandardMaterial>, With<FocusSubject>>,
    mut q_readout: Query<&mut Text, With<FocusReadout>>,
) {
    if !roster.is_changed() || roster.entries.is_empty() {
        return;
    }
    let entry = roster.selected_entry();
    for material in &q_focus {
        if let Some(mut material) = materials.get_mut(&material.0) {
            material.base_color_texture = Some(entry.image.clone());
        }
    }
    for mut text in &mut q_readout {
        **text = format!(
            "FOCUS: {}   [keys 1-{} pick, arrows cycle]",
            entry.label,
            roster.entries.len()
        );
    }
}

/// Pin each label over its subject by projecting the subject through the
/// scenario camera; labels behind the camera hide.
pub fn position_labels(
    q_camera: Query<(&Camera, &GlobalTransform), With<ScenarioCameraMarker>>,
    q_targets: Query<&GlobalTransform>,
    mut q_labels: Query<(&SubjectLabel, &mut Node, &mut Visibility)>,
) {
    let Ok((camera, camera_transform)) = q_camera.single() else {
        return;
    };
    for (label, mut node, mut visibility) in &mut q_labels {
        let Ok(target) = q_targets.get(label.target) else {
            continue;
        };
        match camera.world_to_viewport(camera_transform, target.translation() + label.offset) {
            Ok(position) => {
                node.left = Val::Px(position.x - LABEL_WIDTH / 2.0);
                node.top = Val::Px(position.y);
                visibility.set_if_neq(Visibility::Inherited);
            }
            Err(_) => {
                visibility.set_if_neq(Visibility::Hidden);
            }
        }
    }
}

/// The comparison stage: an empty scenario carrying only the skybox and the
/// repo's standard three-point light rig, so the subjects render under the
/// game's real camera, sky and lighting. Subjects are spawned by the example
/// itself - what is compared is a material, not a scenario object.
pub fn compare_stage(game_assets: &GameAssets, id: &str, name: &str) -> ScenarioConfig {
    ScenarioConfig {
        description: format!("{name} - side-by-side texture comparison"),
        events: vec![ScenarioEventConfig {
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: ThreePointRig::around("photo", Vec3::ZERO, 1.0).actions(),
        }],
        ..ScenarioConfig::new(
            id.to_string(),
            name.to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}
