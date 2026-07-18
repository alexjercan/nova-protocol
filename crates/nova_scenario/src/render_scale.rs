//! Render-scale lever: draw the scenario view into a reduced-resolution
//! offscreen target and upscale it to the window (task 20260718-004723).
//!
//! The v0.7.0 frame-time baseline (`tasks/20260716-123551`) found web to be the
//! one over-budget target and, unlike the discrete-GPU native path, **fill /
//! overhead-bound with almost no headroom**: the same scenes that render at
//! ~19-21 ms natively cost ~34-42 ms in the browser. On a fill-bound path the
//! strongest lever is dropping the number of pixels shaded - it buys more than
//! the existing particle/scatter toggles. This module is that lever.
//!
//! ## How
//!
//! [`GraphicsBudget::render_scale`](nova_gameplay::prelude::GraphicsBudget) is a
//! fraction the scenario view is drawn at. At `1.0` (Medium/High) nothing here
//! fires - the scenario camera renders straight to the window, exactly as
//! before, so the crisp tiers pay zero cost. Below `1.0` (Low) [`reconcile_render_scale`]:
//!
//! 1. creates an offscreen [`Image`] sized `render_scale * window_physical`,
//! 2. points every [`ScenarioCameraMarker`] camera at that image AND marks it
//!    the [`IsDefaultUiCamera`], so the 3D world **and** the HUD render into the
//!    same reduced image - one coordinate space, so screen-space projection
//!    (target markers, lock reticles) stays aligned; the whole frame is scaled
//!    up as a unit,
//! 3. spawns a single blit [`Camera2d`] (the only window camera in this mode)
//!    that draws a full-window sprite of the image, isolated on
//!    [`UPSCALE_LAYER`] so the world camera never sees the sprite and the blit
//!    camera never sees the world.
//!
//! The lever is a pure function of `GraphicsBudget` + window size, so switching
//! quality live (settings menu) or resizing the window reconciles idempotently,
//! and tearing back down to `1.0` restores the direct-to-window path. It is not
//! web-only: native Low downscales too (the user asked for the lever on both;
//! the win is just largest on the constrained web target).
//!
//! ## Why the whole frame, HUD included
//!
//! Rendering the HUD into the reduced image (rather than crisp on the blit
//! camera) keeps the world and the UI in one coordinate space, so the existing
//! world->screen projection needs no render-scale awareness, and it maximizes
//! the win on a fill-bound target (HUD overdraw is real cost too). The price is
//! a slightly softer HUD on Low - an accepted trade for the lowest preset,
//! whose whole job is playability over crispness.

use bevy::{
    camera::{visibility::RenderLayers, RenderTarget},
    prelude::*,
    render::render_resource::TextureFormat,
    ui::IsDefaultUiCamera,
    window::PrimaryWindow,
};
use nova_gameplay::prelude::GraphicsBudget;

use crate::loader::prelude::ScenarioCameraMarker;

/// The [`RenderLayers`] the upscale blit lives on, isolated from the default
/// layer (0) the scenario world renders on: the world Camera3d (no explicit
/// layer, so layer 0) never sees the full-window blit sprite, and the blit
/// Camera2d (this layer only) never sees the world.
const UPSCALE_LAYER: usize = 1;

/// Camera order for the blit camera. The scenario camera defaults to order 0 and
/// the HUD target-inset camera to -1, so `1` runs the blit last, after the
/// offscreen image it samples has been rendered this frame.
const UPSCALE_CAMERA_ORDER: isize = 1;

/// Adds the render-scale reconcile. Registered by [`crate::NovaScenarioPlugin`]
/// only when rendering (the lever is a no-op without a window/GPU).
pub struct RenderScalePlugin;

impl Plugin for RenderScalePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RenderScaleState>();
        app.add_systems(Update, reconcile_render_scale);
    }
}

/// The live render-scale setup, so the reconcile only rebuilds on an actual
/// change (missing/wrong-sized target, or a quality/window flip) rather than
/// churning the render graph every frame.
#[derive(Resource, Default)]
struct RenderScaleState {
    /// The offscreen target the scenario view renders into, when downscaling.
    image: Option<Handle<Image>>,
    /// The size `image` was created at, to detect window/scale changes.
    size: UVec2,
    /// The blit camera entity, so teardown despawns exactly the one we spawned.
    upscale_camera: Option<Entity>,
}

/// Marks the render-scale blit camera (the full-window Camera2d).
#[derive(Component)]
struct RenderScaleUpscaleCamera;

/// Marks the full-window sprite that displays the offscreen target.
#[derive(Component)]
struct RenderScaleUpscaleSprite;

/// Reconcile the render-scale setup against the current [`GraphicsBudget`] and
/// window size. Idempotent: it only mutates on a real diff, so any ordering of
/// quality changes, scenario (re)loads and window resizes converges.
#[allow(clippy::type_complexity)]
fn reconcile_render_scale(
    mut commands: Commands,
    budget: Res<GraphicsBudget>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
    mut state: ResMut<RenderScaleState>,
    mut q_scenario_cam: Query<
        (Entity, &mut RenderTarget, Has<IsDefaultUiCamera>),
        With<ScenarioCameraMarker>,
    >,
    mut q_sprite: Query<&mut Sprite, With<RenderScaleUpscaleSprite>>,
    q_sprite_entity: Query<Entity, With<RenderScaleUpscaleSprite>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let physical = window.physical_size();

    // Downscale only when the preset asks for it AND there is a scenario camera
    // to redirect (never in the menu/editor, whose cameras are not scenario
    // cameras - they keep full resolution). A zero-axis window (minimized, or
    // not yet sized) is left untouched: recreating a zero-area target is a fatal
    // wgpu allocation.
    let want_downscale = !budget.is_native_resolution()
        && !q_scenario_cam.is_empty()
        && physical.x > 0
        && physical.y > 0;

    if !want_downscale {
        teardown_render_scale(
            &mut commands,
            &mut state,
            &mut q_scenario_cam,
            &q_sprite_entity,
        );
        return;
    }

    let desired = budget.render_target_size(physical);

    // (Re)create the offscreen target when it is missing, was dropped, or the
    // window/scale changed since it was made.
    let need_new_target = match &state.image {
        Some(handle) => state.size != desired || !images.contains(handle),
        None => true,
    };
    if need_new_target {
        let handle = create_scaled_target(&mut images, desired);
        state.image = Some(handle);
        state.size = desired;
    }
    let image = state.image.clone().expect("target ensured just above");

    // Point every scenario camera at the offscreen target and make it the
    // default UI camera (so the HUD renders into the same reduced image).
    for (entity, mut target, is_default_ui) in q_scenario_cam.iter_mut() {
        if !targets_image(&target, &image) {
            *target = RenderTarget::Image(image.clone().into());
        }
        if !is_default_ui {
            commands.entity(entity).insert(IsDefaultUiCamera);
        }
    }

    // Ensure the blit camera + full-window sprite exist (spawned once, then kept
    // in sync below).
    if state.upscale_camera.is_none() {
        let camera = commands
            .spawn((
                Name::new("Render-Scale Upscale Camera"),
                Camera2d,
                Camera {
                    order: UPSCALE_CAMERA_ORDER,
                    ..default()
                },
                RenderLayers::layer(UPSCALE_LAYER),
                RenderScaleUpscaleCamera,
            ))
            .id();
        commands.spawn((
            Name::new("Render-Scale Upscale Sprite"),
            Sprite {
                image: image.clone(),
                custom_size: Some(window.size()),
                ..default()
            },
            RenderLayers::layer(UPSCALE_LAYER),
            RenderScaleUpscaleSprite,
        ));
        state.upscale_camera = Some(camera);
    }

    // Keep the sprite pointed at the live target and sized to the window, so a
    // window resize or a target recreation both flow through here. Both writes
    // are diff-guarded so a steady frame does not mark the `Sprite` changed.
    let sprite_size = Some(window.size());
    for mut sprite in q_sprite.iter_mut() {
        if sprite.image != image {
            sprite.image = image.clone();
        }
        if sprite.custom_size != sprite_size {
            sprite.custom_size = sprite_size;
        }
    }
}

/// Restore the direct-to-window path: scenario cameras back to the window, blit
/// camera + sprite despawned, target dropped. A no-op when nothing is set up
/// (the steady state on Medium/High).
#[allow(clippy::type_complexity)]
fn teardown_render_scale(
    commands: &mut Commands,
    state: &mut RenderScaleState,
    q_scenario_cam: &mut Query<
        (Entity, &mut RenderTarget, Has<IsDefaultUiCamera>),
        With<ScenarioCameraMarker>,
    >,
    q_sprite_entity: &Query<Entity, With<RenderScaleUpscaleSprite>>,
) {
    if state.image.is_none() && state.upscale_camera.is_none() {
        return;
    }

    // Reset the target via Commands (not the immediate `&mut`) so it lands with
    // the blit despawn and the IsDefaultUiCamera removal in one apply - otherwise
    // this frame would render the scenario straight to the window while the
    // not-yet-despawned blit draws its stale image on top (a 1-frame glitch on a
    // live Low->High switch).
    for (entity, target, is_default_ui) in q_scenario_cam.iter_mut() {
        if !matches!(*target, RenderTarget::Window(_)) {
            commands.entity(entity).insert(RenderTarget::default());
        }
        if is_default_ui {
            commands.entity(entity).remove::<IsDefaultUiCamera>();
        }
    }
    if let Some(camera) = state.upscale_camera.take() {
        commands.entity(camera).despawn();
    }
    for sprite in q_sprite_entity.iter() {
        commands.entity(sprite).despawn();
    }
    state.image = None;
    state.size = UVec2::ZERO;
}

/// Whether `target` renders to `handle`.
fn targets_image(target: &RenderTarget, handle: &Handle<Image>) -> bool {
    matches!(target, RenderTarget::Image(image) if &image.handle == handle)
}

/// Create the offscreen render target. Rgba8UnormSrgb with the default view (no
/// view-format override): the same WebGL2-safe target the HUD inset uses (see
/// [`crate`]'s `nova_gameplay::hud::target_inset::create_render_target`), so a
/// reduced-resolution frame never trips `DownlevelFlags::VIEW_FORMATS` on the
/// weak web GPUs this lever exists for. `new_target_texture` sets the
/// RENDER_ATTACHMENT | TEXTURE_BINDING | COPY_DST usages.
fn create_scaled_target(images: &mut Assets<Image>, size: UVec2) -> Handle<Image> {
    let image = Image::new_target_texture(
        size.x.max(1),
        size.y.max(1),
        TextureFormat::Rgba8UnormSrgb,
        None,
    );
    images.add(image)
}

#[cfg(test)]
mod tests {
    use bevy::asset::AssetPlugin;
    use nova_gameplay::prelude::GraphicsQuality;

    use super::*;

    /// A headless app with just what the reconcile touches: an `Assets<Image>`
    /// (via `AssetPlugin`), a primary window, the budget resource and the
    /// reconcile system. No renderer needed - the reconcile is pure ECS
    /// (component/asset) bookkeeping.
    fn test_app(quality: GraphicsQuality) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Image>();
        app.insert_resource(GraphicsBudget::for_quality(quality));
        app.init_resource::<RenderScaleState>();
        app.add_systems(Update, reconcile_render_scale);
        app.world_mut().spawn((
            Window {
                resolution: (1280, 720).into(),
                ..default()
            },
            PrimaryWindow,
        ));
        app
    }

    /// Spawn a bare scenario camera (the marker + a window-targeting Camera, as
    /// the loader spawns it).
    fn spawn_scenario_camera(app: &mut App) -> Entity {
        app.world_mut()
            .spawn((ScenarioCameraMarker, Camera3d::default(), Camera::default()))
            .id()
    }

    #[test]
    fn native_resolution_leaves_the_camera_on_the_window() {
        let mut app = test_app(GraphicsQuality::High);
        let cam = spawn_scenario_camera(&mut app);
        app.update();

        // No offscreen target, no blit camera, camera still on the window.
        assert!(app.world().resource::<RenderScaleState>().image.is_none());
        assert!(app
            .world()
            .resource::<RenderScaleState>()
            .upscale_camera
            .is_none());
        let target = app.world().entity(cam).get::<RenderTarget>().unwrap();
        assert!(matches!(target, RenderTarget::Window(_)));
        assert!(app.world().entity(cam).get::<IsDefaultUiCamera>().is_none());
    }

    #[test]
    fn low_downscales_the_scenario_camera_and_spawns_a_blit_camera() {
        let mut app = test_app(GraphicsQuality::Low);
        let cam = spawn_scenario_camera(&mut app);
        app.update();

        let state = app.world().resource::<RenderScaleState>();
        let image = state
            .image
            .clone()
            .expect("Low creates an offscreen target");
        // Sized to render_scale * window (physical == logical at scale factor 1).
        let expected = GraphicsBudget::for_quality(GraphicsQuality::Low)
            .render_target_size(UVec2::new(1280, 720));
        assert_eq!(state.size, expected);
        assert!(state.upscale_camera.is_some());

        // Scenario camera now renders into the image and owns the default UI.
        let target = app.world().entity(cam).get::<RenderTarget>().unwrap();
        assert!(targets_image(target, &image));
        assert!(app.world().entity(cam).get::<IsDefaultUiCamera>().is_some());

        // Exactly one blit camera + one full-window sprite, isolated on the
        // upscale layer.
        let mut cams = app
            .world_mut()
            .query_filtered::<Entity, With<RenderScaleUpscaleCamera>>();
        assert_eq!(cams.iter(app.world()).count(), 1);
        let mut sprites = app
            .world_mut()
            .query_filtered::<&Sprite, With<RenderScaleUpscaleSprite>>();
        let sprite = sprites.iter(app.world()).next().expect("blit sprite");
        assert_eq!(sprite.image, image);
        assert_eq!(sprite.custom_size, Some(Vec2::new(1280.0, 720.0)));
    }

    #[test]
    fn switching_low_to_high_tears_the_setup_back_down() {
        let mut app = test_app(GraphicsQuality::Low);
        let cam = spawn_scenario_camera(&mut app);
        app.update();
        assert!(app.world().resource::<RenderScaleState>().image.is_some());

        // Flip to High: the reconcile must restore the direct-to-window path.
        app.insert_resource(GraphicsBudget::for_quality(GraphicsQuality::High));
        app.update();

        let state = app.world().resource::<RenderScaleState>();
        assert!(state.image.is_none(), "target dropped");
        assert!(state.upscale_camera.is_none(), "blit camera despawned");
        let target = app.world().entity(cam).get::<RenderTarget>().unwrap();
        assert!(matches!(target, RenderTarget::Window(_)));
        assert!(app.world().entity(cam).get::<IsDefaultUiCamera>().is_none());
        let mut sprites = app
            .world_mut()
            .query_filtered::<Entity, With<RenderScaleUpscaleSprite>>();
        assert_eq!(
            sprites.iter(app.world()).count(),
            0,
            "blit sprite despawned"
        );
    }

    #[test]
    fn recreates_the_target_when_the_window_resizes() {
        let mut app = test_app(GraphicsQuality::Low);
        spawn_scenario_camera(&mut app);
        app.update();
        let first = app.world().resource::<RenderScaleState>().size;

        // Shrink the window; the target must follow.
        let mut q = app
            .world_mut()
            .query_filtered::<&mut Window, With<PrimaryWindow>>();
        q.single_mut(app.world_mut()).unwrap().resolution = (640, 360).into();
        app.update();

        let state = app.world().resource::<RenderScaleState>();
        let expected = GraphicsBudget::for_quality(GraphicsQuality::Low)
            .render_target_size(UVec2::new(640, 360));
        assert_eq!(state.size, expected);
        assert_ne!(state.size, first, "resize rebuilt the target");
    }

    #[test]
    fn downscale_needs_a_scenario_camera() {
        // Low preset but no scenario camera (menu/editor): nothing is set up, so
        // those views keep full resolution.
        let mut app = test_app(GraphicsQuality::Low);
        app.update();
        let state = app.world().resource::<RenderScaleState>();
        assert!(state.image.is_none());
        assert!(state.upscale_camera.is_none());
    }
}
