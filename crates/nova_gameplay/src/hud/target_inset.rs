//! Target inset: a corner HUD panel showing a live, magnified render-to-texture
//! close-up of the currently focused/locked enemy ship, so the player can see
//! which section the fine-lock is selecting (and watch it take damage / explode
//! scope-style) instead of squinting at sub-pixel markers at range
//! (task 20260710-104421; design in
//! docs/spikes/20260710-104011-target-inset-view.md, Option A).
//!
//! Three pieces, all thin consumers of the existing targeting state
//! (input/targeting.rs) - this module adds no new targeting mechanics:
//!
//! - A second `Camera3d` that renders the live scene into an `Image` via the
//!   standalone [`RenderTarget`] component (the Bevy 0.19 RTT path, distinct
//!   from a second window-targeting camera, which blacks out the scene). The
//!   probe in task 20260710-104421 confirmed RTT coexists with the main
//!   camera's per-camera post-processing + skybox and trips none of the
//!   marker-filtered `Single<Camera>` queries.
//! - A corner [`ImageNode`] panel showing that texture, spawned with the player
//!   HUD (hud/mod.rs observers) and shown only while a lock is focused.
//! - An in-scene emissive overlay on the fine-locked section, so the selection
//!   reads in BOTH the main view and the inset with no projection code.
//!
//! The inset camera spawns/despawns and the panel shows/hides with the focus
//! dwell ([`SpaceshipPlayerLockFocus::focused_on`]); the camera is posed each
//! frame on the locked ship's [`live_structure_anchor`] from a scope-like
//! player-relative bearing.

use avian3d::prelude::ComputedCenterOfMass;
use bevy::{camera::RenderTarget, prelude::*, render::render_resource::TextureFormat};

use crate::prelude::*;

pub mod prelude {
    pub use super::{
        target_inset_hud, TargetInsetCameraMarker, TargetInsetHighlightAssets,
        TargetInsetHighlightMarker, TargetInsetHudMarker, TargetInsetHudPlugin,
        TargetInsetRenderTarget,
    };
}

/// Square resolution (px) of the offscreen render texture. Small on purpose:
/// the inset renders the scene a second time, so it stays cheap.
const INSET_TEXTURE_PX: u32 = 512;

/// On-screen size (px) of the inset panel.
const INSET_PANEL_PX: f32 = 256.0;

/// Panel inset from the screen corner (px).
const INSET_MARGIN_PX: f32 = 12.0;

/// Panel border thickness (px).
const INSET_BORDER_PX: f32 = 2.0;

/// Panel border tint: the hot-metal lock red the component markers use
/// (`hud/component_lock.rs` MARKER_SELECTED_COLOR), so the inset reads as part
/// of the targeting family.
const INSET_BORDER_COLOR: Color = Color::srgba(1.0, 0.45, 0.3, 0.95);

/// Inset camera background (no skybox on the inset: a dark clear makes the
/// locked ship stand out, and avoids plumbing the scenario cubemap handle into
/// gameplay). A deep near-black blue.
const INSET_CLEAR_COLOR: Color = Color::srgb(0.02, 0.02, 0.035);

/// Half the size of a section's unit box (`Collider::cuboid(1,1,1)` /
/// `Cuboid::new(1,1,1)`, sections/base_section.rs): the framing radius pads the
/// section-center spread by this so the hull edge, not its center, frames.
const SECTION_HALF_EXTENT: f32 = 0.5;

/// Camera pull-back as a multiple of the target's framing radius: how much of
/// the panel the ship fills. A feel knob.
const INSET_FRAME_PADDING: f32 = 2.2;

/// Floor on the inset camera distance (world units), so a tiny single-section
/// wreck does not clip into the near plane. A feel knob.
const INSET_MIN_DISTANCE: f32 = 6.0;

/// Camera elevation as a fraction of its distance: a slight top-down tilt so
/// the hull reads instead of an edge-on silhouette. A feel knob.
const INSET_ELEVATION: f32 = 0.3;

/// Scale of the emissive highlight shell around the selected section's unit
/// box: slightly larger so it reads as an outline glow rather than replacing
/// the section. A feel knob.
const HIGHLIGHT_SCALE: f32 = 1.14;

/// Marker for the inset panel root (the `ImageNode`).
#[derive(Component, Debug, Clone, Reflect)]
pub struct TargetInsetHudMarker;

/// Marker for the offscreen inset camera. Deliberately carries none of the
/// scene-camera markers (SpaceshipCameraController, ScenarioCameraMarker,
/// WASDCameraController, ScreenIndicatorCamera, SfxListenerMarker), so it trips
/// no marker-filtered `Single<Camera>` query and is not a projection/audio
/// camera - just an RTT source.
#[derive(Component, Debug, Clone, Reflect)]
pub struct TargetInsetCameraMarker;

/// Marker for the emissive overlay spawned as a child of the fine-locked
/// section.
#[derive(Component, Debug, Clone, Reflect)]
pub struct TargetInsetHighlightMarker;

/// The section a highlight overlay belongs to (its parent), so the reconcile
/// can match live overlays against the current selection.
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct TargetInsetHighlightOf(pub Entity);

/// The render-target image the inset camera draws into and the panel displays.
/// `None` until the player HUD sets it up (`Assets<Image>` exists at runtime,
/// not necessarily at plugin build).
#[derive(Resource, Debug, Clone, Default, Deref, DerefMut)]
pub struct TargetInsetRenderTarget(pub Option<Handle<Image>>);

/// Shared mesh + material for the section highlight shell, built once with the
/// player HUD so the reconcile allocates nothing per selection change.
#[derive(Resource, Debug, Clone)]
pub struct TargetInsetHighlightAssets {
    pub mesh: Handle<Mesh>,
    pub material: Handle<StandardMaterial>,
}

/// The emissive shell material: an unlit, additive-looking translucent red that
/// blooms in both the main view and the inset. Double-sided with no culling so
/// the shell reads as a glow around the section rather than a solid block.
pub fn highlight_material() -> StandardMaterial {
    StandardMaterial {
        base_color: Color::srgba(1.0, 0.35, 0.25, 0.22),
        emissive: LinearRgba::rgb(3.0, 0.7, 0.4),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        double_sided: true,
        cull_mode: None,
        ..default()
    }
}

/// Create the offscreen render target. Rgba8Unorm storage with an Rgba8UnormSrgb
/// view is the Bevy 0.19 RTT convention (3d/render_to_texture example);
/// `new_target_texture` sets the RENDER_ATTACHMENT | TEXTURE_BINDING | COPY_DST
/// usages.
pub fn create_render_target(images: &mut Assets<Image>) -> Handle<Image> {
    let image = Image::new_target_texture(
        INSET_TEXTURE_PX,
        INSET_TEXTURE_PX,
        TextureFormat::Rgba8Unorm,
        Some(TextureFormat::Rgba8UnormSrgb),
    );
    images.add(image)
}

/// The inset panel bundle: a corner-anchored node showing the render target,
/// starting Hidden (the focus-driven reconcile reveals it). `Chrome` tier +
/// `HudSelfDrivenVisibility` so it follows the HUD level yet the focus reconcile
/// owns its moment-to-moment visibility (the gravity-sphere pattern).
pub fn target_inset_hud(image: Handle<Image>) -> impl Bundle {
    (
        Name::new("TargetInsetHUD"),
        TargetInsetHudMarker,
        HudTier::Chrome,
        HudSelfDrivenVisibility,
        Node {
            position_type: PositionType::Absolute,
            // Top-right corner: clear of the objectives column (mid-right), the
            // keybind hints (bottom-left) and the dev inspector overlay
            // (top-left). A feel knob.
            right: Val::Px(INSET_MARGIN_PX),
            top: Val::Px(INSET_MARGIN_PX),
            width: Val::Px(INSET_PANEL_PX),
            height: Val::Px(INSET_PANEL_PX),
            border: UiRect::all(Val::Px(INSET_BORDER_PX)),
            ..default()
        },
        BorderColor::all(INSET_BORDER_COLOR),
        ImageNode::new(image),
        Visibility::Hidden,
    )
}

#[derive(Default)]
pub struct TargetInsetHudPlugin;

impl Plugin for TargetInsetHudPlugin {
    fn build(&self, app: &mut App) {
        debug!("TargetInsetHudPlugin: build");

        app.init_resource::<TargetInsetRenderTarget>();
        app.register_type::<TargetInsetHudMarker>();
        app.register_type::<TargetInsetCameraMarker>();
        app.register_type::<TargetInsetHighlightMarker>();
        app.register_type::<TargetInsetHighlightOf>();

        app.add_systems(
            Update,
            (drive_inset_camera, sync_section_highlight).in_set(super::NovaHudSystems),
        );
    }
}

/// The scope-like inset camera pose: sit on the line from the target toward the
/// player (so the panel shows the face the player is shooting), pulled back by
/// the framing distance and lifted a little for a readable hull angle.
fn inset_camera_pose(target_anchor: Vec3, player_anchor: Vec3, radius: f32) -> Transform {
    let bearing = (player_anchor - target_anchor)
        .try_normalize()
        .unwrap_or(Vec3::Z);
    let distance = (radius * INSET_FRAME_PADDING).max(INSET_MIN_DISTANCE);
    let eye = target_anchor + bearing * distance + Vec3::Y * (distance * INSET_ELEVATION);
    Transform::from_translation(eye).looking_at(target_anchor, Vec3::Y)
}

/// Framing radius of the locked ship: the farthest section center from the
/// anchor, padded by a section half-extent so the hull edge frames. Falls back
/// to the half-extent when the ship has no live sections (should not happen
/// while focused, but keeps the pose finite).
fn ship_framing_radius(
    target: Entity,
    anchor: Vec3,
    q_sections: &Query<
        (&ChildOf, &GlobalTransform),
        (With<SectionMarker>, Without<TargetInsetCameraMarker>),
    >,
) -> f32 {
    let spread = q_sections
        .iter()
        .filter(|(ChildOf(parent), _)| *parent == target)
        .map(|(_, gt)| gt.translation().distance(anchor))
        .fold(0.0_f32, f32::max);
    spread + SECTION_HALF_EXTENT
}

/// The inset camera bundle. Order -1 renders it before the main (order 0)
/// window camera into its own image target. Carries `PostProcessingCamera` so
/// its tonemapping/bloom look matches the main view (thruster glow, explosions);
/// no skybox (see [`INSET_CLEAR_COLOR`]).
fn inset_camera_bundle(image: Handle<Image>, pose: Transform) -> impl Bundle {
    (
        Name::new("Target Inset Camera"),
        TargetInsetCameraMarker,
        Camera3d::default(),
        Camera {
            order: -1,
            clear_color: ClearColorConfig::Custom(INSET_CLEAR_COLOR),
            ..default()
        },
        RenderTarget::Image(image.into()),
        pose,
        PostProcessingCamera,
    )
}

/// Spawn/despawn the inset camera and show/hide the panel with the focus dwell,
/// and pose the camera on the locked ship each frame while focused. One
/// idempotent system (like the component-marker reconcile) so every ordering of
/// lock/focus/section changes converges; folding the lifecycle and the pose
/// together avoids a one-frame default-pose flash on spawn.
#[allow(clippy::type_complexity)]
fn drive_inset_camera(
    mut commands: Commands,
    lock: Res<SpaceshipPlayerTargetLock>,
    focus: Res<SpaceshipPlayerLockFocus>,
    hud_visibility: Res<super::HudVisibility>,
    render_target: Res<TargetInsetRenderTarget>,
    q_anchor: Query<(&Transform, Option<&ComputedCenterOfMass>), Without<TargetInsetCameraMarker>>,
    q_player: Query<
        (&Transform, Option<&ComputedCenterOfMass>),
        (
            With<SpaceshipRootMarker>,
            With<PlayerSpaceshipMarker>,
            Without<TargetInsetCameraMarker>,
        ),
    >,
    q_sections: Query<
        (&ChildOf, &GlobalTransform),
        (With<SectionMarker>, Without<TargetInsetCameraMarker>),
    >,
    mut q_camera: Query<(Entity, &mut Transform), With<TargetInsetCameraMarker>>,
    mut q_panel: Query<&mut Visibility, With<TargetInsetHudMarker>>,
) {
    // The inset exists only while the focus dwell is complete on the current
    // lock, the HUD is showing chrome (so hiding the HUD also stops the second
    // render, not just the panel), and the target still resolves to a real
    // anchor. The inset panel is Chrome tier, so gating here keeps the RTT
    // camera and the (tier-hidden) panel consistent.
    let framed = match **lock {
        Some(target) if hud_visibility.shows(HudTier::Chrome) && focus.focused_on(target) => {
            q_anchor
                .get(target)
                .ok()
                .map(|(transform, com)| (target, live_structure_anchor(transform, com)))
        }
        _ => None,
    };

    let Some((target, target_anchor)) = framed else {
        // Not focused (or the target vanished): hide the panel and tear the
        // camera down so the scene is not rendered a second time for nothing.
        for mut visibility in &mut q_panel {
            visibility.set_if_neq(Visibility::Hidden);
        }
        for (camera, _) in &q_camera {
            commands.entity(camera).despawn();
        }
        return;
    };

    let player_anchor = q_player
        .iter()
        .next()
        .map(|(transform, com)| live_structure_anchor(transform, com))
        // No player anchor (teardown): fall back to a fixed bearing so the pose
        // stays finite rather than degenerate.
        .unwrap_or(target_anchor + Vec3::Z);
    let radius = ship_framing_radius(target, target_anchor, &q_sections);
    let pose = inset_camera_pose(target_anchor, player_anchor, radius);

    for mut visibility in &mut q_panel {
        visibility.set_if_neq(Visibility::Visible);
    }

    if let Ok((_, mut transform)) = q_camera.single_mut() {
        *transform = pose;
    } else if let Some(image) = render_target.0.clone() {
        commands.spawn(inset_camera_bundle(image, pose));
    }
}

/// Keep exactly one emissive highlight overlay on the fine-locked section, and
/// none otherwise. The selection is already focus-gated by the targeting layer
/// (`SpaceshipPlayerComponentLock.section` is only `Some` while focused), so
/// this reconcile just mirrors it; a detached/despawned section drops its
/// overlay (a despawned section takes its child overlay with it, but a
/// re-selected sibling still needs the stale one cleared).
fn sync_section_highlight(
    mut commands: Commands,
    component: Res<SpaceshipPlayerComponentLock>,
    assets: Option<Res<TargetInsetHighlightAssets>>,
    q_highlights: Query<(Entity, &TargetInsetHighlightOf), With<TargetInsetHighlightMarker>>,
    q_sections: Query<(), With<SectionMarker>>,
) {
    // Only highlight a section that still exists (attached or inactive-in-place
    // both keep the SectionMarker; despawn/detach removes it).
    let selected = component
        .section
        .filter(|section| q_sections.get(*section).is_ok());

    // Drop overlays that no longer match the selection.
    for (overlay, of) in &q_highlights {
        if selected != Some(**of) {
            commands.entity(overlay).despawn();
        }
    }

    let Some(section) = selected else {
        return;
    };
    let Some(assets) = assets else {
        // Assets not built yet (no player HUD): nothing to spawn with.
        return;
    };
    let already = q_highlights.iter().any(|(_, of)| **of == section);
    if !already {
        commands
            .entity(section)
            .with_child(highlight_bundle(&assets, section));
    }
}

/// One emissive shell child, scaled just past the section's unit box.
fn highlight_bundle(assets: &TargetInsetHighlightAssets, section: Entity) -> impl Bundle {
    (
        Name::new("TargetInsetHighlight"),
        TargetInsetHighlightMarker,
        TargetInsetHighlightOf(section),
        Mesh3d(assets.mesh.clone()),
        MeshMaterial3d(assets.material.clone()),
        Transform::from_scale(Vec3::splat(HIGHLIGHT_SCALE)),
    )
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    // -- camera lifecycle --

    /// Build the focused rig: panel + player + focused target with `n` sections.
    fn rig(n: usize) -> (World, Entity) {
        let mut world = World::new();
        world.insert_resource(super::super::HudVisibility::All);
        world.insert_resource(TargetInsetRenderTarget(Some(Handle::default())));
        world.spawn((Name::new("panel"), TargetInsetHudMarker, Visibility::Hidden));
        world.spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_xyz(0.0, 0.0, 0.0),
        ));
        let target = world
            .spawn((SpaceshipRootMarker, Transform::from_xyz(0.0, 0.0, -50.0)))
            .id();
        for i in 0..n {
            world.spawn((
                SectionMarker,
                ChildOf(target),
                GlobalTransform::from_xyz(0.0, 0.0, -50.0 + i as f32),
            ));
        }
        world.insert_resource(SpaceshipPlayerTargetLock(Some(target)));
        world.insert_resource(SpaceshipPlayerLockFocus {
            target: Some(target),
            seconds: f32::MAX,
        });
        (world, target)
    }

    fn camera_count(world: &mut World) -> usize {
        world
            .query_filtered::<(), With<TargetInsetCameraMarker>>()
            .iter(world)
            .count()
    }

    fn panel_visibility(world: &mut World) -> Visibility {
        *world
            .query_filtered::<&Visibility, With<TargetInsetHudMarker>>()
            .iter(world)
            .next()
            .expect("panel exists")
    }

    #[test]
    fn camera_and_panel_appear_only_while_focused() {
        let (mut world, target) = rig(3);

        world.run_system_once(drive_inset_camera).unwrap();
        assert_eq!(camera_count(&mut world), 1, "focused: one inset camera");
        assert_eq!(
            panel_visibility(&mut world),
            Visibility::Visible,
            "focused: panel shown"
        );

        // Delivery guard: losing the dwell tears the camera down and hides the
        // panel (the positive state above proves the assertion can differ).
        world.resource_mut::<SpaceshipPlayerLockFocus>().seconds = 0.0;
        world.run_system_once(drive_inset_camera).unwrap();
        assert_eq!(camera_count(&mut world), 0, "unfocused: camera despawned");
        assert_eq!(
            panel_visibility(&mut world),
            Visibility::Hidden,
            "unfocused: panel hidden"
        );

        let _ = target;
    }

    #[test]
    fn camera_does_not_duplicate_across_frames() {
        let (mut world, _) = rig(3);
        world.run_system_once(drive_inset_camera).unwrap();
        world.run_system_once(drive_inset_camera).unwrap();
        world.run_system_once(drive_inset_camera).unwrap();
        assert_eq!(
            camera_count(&mut world),
            1,
            "the reconcile keeps exactly one inset camera"
        );
    }

    #[test]
    fn camera_clears_when_the_lock_changes_before_the_dwell() {
        let (mut world, _) = rig(3);
        world.run_system_once(drive_inset_camera).unwrap();
        assert_eq!(camera_count(&mut world), 1);

        // A fresh lock with no completed dwell: the inset must vanish.
        let other = world
            .spawn((SpaceshipRootMarker, Transform::default()))
            .id();
        world.insert_resource(SpaceshipPlayerTargetLock(Some(other)));
        world.insert_resource(SpaceshipPlayerLockFocus {
            target: Some(other),
            seconds: 0.0,
        });
        world.run_system_once(drive_inset_camera).unwrap();
        assert_eq!(camera_count(&mut world), 0);
        assert_eq!(panel_visibility(&mut world), Visibility::Hidden);
    }

    #[test]
    fn camera_absent_while_hud_chrome_is_hidden() {
        let (mut world, _) = rig(3);

        // Focused, but the HUD is hiding chrome (Minimal drops Chrome; None
        // drops everything): the inset panel is tier-hidden, so the second
        // render must not run either.
        world.insert_resource(super::super::HudVisibility::None);
        world.run_system_once(drive_inset_camera).unwrap();
        assert_eq!(
            camera_count(&mut world),
            0,
            "hidden chrome: no inset camera renders while the panel is hidden"
        );
        assert_eq!(panel_visibility(&mut world), Visibility::Hidden);

        // Delivery guard: showing chrome again brings the inset back, so the
        // assertion above is really gated on visibility.
        world.insert_resource(super::super::HudVisibility::All);
        world.run_system_once(drive_inset_camera).unwrap();
        assert_eq!(camera_count(&mut world), 1);
        assert_eq!(panel_visibility(&mut world), Visibility::Visible);
    }

    // -- highlight --

    /// Build a world for the highlight reconcile: highlight assets (default
    /// handles, no real assets needed) + a focused target with `n` sections.
    fn highlight_rig(n: usize) -> (World, Vec<Entity>) {
        let mut world = World::new();
        world.insert_resource(TargetInsetHighlightAssets {
            mesh: Handle::default(),
            material: Handle::default(),
        });
        let target = world.spawn(SpaceshipRootMarker).id();
        let sections: Vec<Entity> = (0..n)
            .map(|_| world.spawn((SectionMarker, ChildOf(target))).id())
            .collect();
        world.insert_resource(SpaceshipPlayerComponentLock::default());
        (world, sections)
    }

    fn highlight_targets(world: &mut World) -> Vec<Entity> {
        let mut v: Vec<Entity> = world
            .query_filtered::<&TargetInsetHighlightOf, With<TargetInsetHighlightMarker>>()
            .iter(world)
            .map(|of| **of)
            .collect();
        v.sort();
        v
    }

    #[test]
    fn highlight_follows_the_component_lock_and_reverts() {
        let (mut world, sections) = highlight_rig(3);
        let (a, b) = (sections[0], sections[1]);

        // Nothing selected: no overlay.
        world.run_system_once(sync_section_highlight).unwrap();
        assert!(highlight_targets(&mut world).is_empty());

        // Select a: exactly one overlay, on a.
        world.resource_mut::<SpaceshipPlayerComponentLock>().section = Some(a);
        world.run_system_once(sync_section_highlight).unwrap();
        assert_eq!(highlight_targets(&mut world), vec![a]);

        // Move to b: the a overlay is dropped, one overlay on b.
        world.resource_mut::<SpaceshipPlayerComponentLock>().section = Some(b);
        world.run_system_once(sync_section_highlight).unwrap();
        assert_eq!(highlight_targets(&mut world), vec![b]);
    }

    #[test]
    fn highlight_does_not_duplicate_across_frames() {
        let (mut world, sections) = highlight_rig(2);
        world.resource_mut::<SpaceshipPlayerComponentLock>().section = Some(sections[0]);
        world.run_system_once(sync_section_highlight).unwrap();
        world.run_system_once(sync_section_highlight).unwrap();
        assert_eq!(highlight_targets(&mut world), vec![sections[0]]);
    }

    #[test]
    fn highlight_clears_when_its_section_dies() {
        let (mut world, sections) = highlight_rig(2);
        let a = sections[0];
        world.resource_mut::<SpaceshipPlayerComponentLock>().section = Some(a);
        world.run_system_once(sync_section_highlight).unwrap();
        assert_eq!(highlight_targets(&mut world), vec![a]);

        // The section despawns (destroyed): its overlay child goes with it, and
        // the selection no longer resolves, so the reconcile settles to empty.
        world.despawn(a);
        world.run_system_once(sync_section_highlight).unwrap();
        assert!(highlight_targets(&mut world).is_empty());
    }
}
