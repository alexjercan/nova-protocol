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
//! 2. points every [`ScenarioCameraMarker`] camera at that image, setting the
//!    image target's `scale_factor` so the camera still reports the WINDOW's
//!    logical viewport (`logical = physical / scale_factor`) - the 3D world is
//!    drawn with fewer pixels but world->screen HUD projection
//!    ([`crate::loader`]'s scenario camera feeds `hud::screen_indicator`) stays
//!    in window space, so nothing HUD-side needs render-scale awareness,
//! 3. spawns a single blit [`Camera2d`] targeting the window that draws a
//!    full-window sprite of the image AND is the [`IsDefaultUiCamera`], so the
//!    HUD/menus render crisp over the upscaled world and, crucially, stay
//!    interactive.
//!
//! The lever is a pure function of `GraphicsBudget` + window size, so switching
//! quality live (settings menu) or resizing the window reconciles idempotently,
//! and tearing back down to `1.0` restores the direct-to-window path. It is not
//! web-only: native Low downscales too (the user asked for the lever on both;
//! the win is just largest on the constrained web target).
//!
//! ## Why the UI stays on the window (and the world does not)
//!
//! bevy_ui's `ui_focus_system` only delivers a cursor to a camera whose render
//! target is a WINDOW; a UI camera pointed at an image renders its nodes but
//! never registers a click. So the HUD and menus MUST live on a window camera to
//! stay clickable - here the blit `Camera2d`. Only the 3D world goes into the
//! reduced image; the HUD renders full-resolution on top of the upscale (crisper
//! than baking it into the reduced image, and it keeps the settings menu that
//! toggles this very preset usable on Low). The world->screen projection stays
//! aligned via the image target's `scale_factor` (step 2), not by sharing a
//! coordinate space with the UI.

use bevy::{
    camera::{ImageRenderTarget, RenderTarget},
    prelude::*,
    render::render_resource::TextureFormat,
    ui::IsDefaultUiCamera,
    window::PrimaryWindow,
};
use nova_gameplay::prelude::GraphicsBudget;

use crate::loader::prelude::ScenarioCameraMarker;

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
        (Entity, &mut RenderTarget, &mut Projection),
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

    // Point every scenario camera at the offscreen target. The image target's
    // `scale_factor` is set so the camera reports the WINDOW's logical viewport
    // (`logical = physical / scale_factor`), NOT the reduced image's - so the
    // world->screen HUD projection (`hud::screen_indicator`, which reads
    // `world_to_viewport`/`logical_viewport_size` off this camera) stays in
    // window space even though the frame is drawn with fewer pixels. Crucially,
    // the scenario camera is NOT the default UI camera: bevy_ui only delivers a
    // cursor to WINDOW-targeted cameras (`ui_focus_system`), so UI parented to an
    // image-targeted camera renders but is unclickable - the blit camera (a real
    // window camera, below) owns the UI instead.
    let logical = window.size();
    let scale_factor = if logical.x > 0.0 {
        desired.x as f32 / logical.x
    } else {
        1.0
    };
    let wanted = ImageRenderTarget {
        handle: image.clone(),
        scale_factor,
    };
    for (_entity, mut target, mut projection) in q_scenario_cam.iter_mut() {
        if !matches!(&*target, RenderTarget::Image(current) if *current == wanted) {
            *target = RenderTarget::Image(wanted.clone());
            // bevy's `camera_system` only re-derives a camera's target info when
            // the target CONTENT changes (window resize / image asset event), the
            // camera is added, or its Projection changed - NOT when the
            // `RenderTarget` component is swapped at runtime. Without this the
            // camera keeps the old target's size/scale after a live quality
            // switch (the switch appears to do nothing, then the stale reduced
            // size "sticks" onto the window on the way back). Marking the
            // projection changed forces the re-derive.
            projection.set_changed();
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
                // The blit camera targets the WINDOW (Camera2d's default), so it
                // is the interactive, full-resolution UI camera: bevy_ui feeds
                // the cursor only to a window-targeted camera, and the HUD/menus
                // render crisp over the upscaled world instead of being baked
                // into the reduced image. It renders only the full-window sprite
                // (the sole Camera2d, and Camera3d never draws 2D sprites, so no
                // RenderLayers isolation is needed) plus the UI pass.
                IsDefaultUiCamera,
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
/// (the steady state on Medium/High). The scenario camera never carries
/// `IsDefaultUiCamera` (the blit camera does), so tearing the blit down returns
/// UI to bevy's single-window-camera default - the scenario camera on the window.
fn teardown_render_scale(
    commands: &mut Commands,
    state: &mut RenderScaleState,
    q_scenario_cam: &mut Query<
        (Entity, &mut RenderTarget, &mut Projection),
        With<ScenarioCameraMarker>,
    >,
    q_sprite_entity: &Query<Entity, With<RenderScaleUpscaleSprite>>,
) {
    if state.image.is_none() && state.upscale_camera.is_none() {
        return;
    }

    // Reset the target to the window and mark the projection changed so bevy's
    // `camera_system` re-derives the camera's target info - without that the
    // camera keeps the reduced image's size after switching back to High and
    // renders the window at the stale low resolution (the "High drops a lot"
    // half of the switch bug). Immediate (`&mut`) rather than deferred so the
    // reset and the projection touch land in the same frame; the blit despawns a
    // frame later, so at worst its stale sprite shows for one frame.
    for (_entity, mut target, mut projection) in q_scenario_cam.iter_mut() {
        if !matches!(*target, RenderTarget::Window(_)) {
            *target = RenderTarget::default();
            projection.set_changed();
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

        // Scenario camera renders into the image, with a scale_factor that makes
        // it report the WINDOW's logical viewport (physical/scale_factor = 1280),
        // so HUD world->screen projection stays in window space.
        let target = app.world().entity(cam).get::<RenderTarget>().unwrap();
        let RenderTarget::Image(image_target) = target else {
            panic!("Low points the scenario camera at an image, got {target:?}");
        };
        assert_eq!(image_target.handle, image);
        let want_scale = expected.x as f32 / 1280.0;
        assert!(
            (image_target.scale_factor - want_scale).abs() < 1e-4,
            "image scale_factor {} should report window-logical viewport (want {want_scale})",
            image_target.scale_factor
        );

        // The scenario camera is NOT the UI camera - UI on an image-targeted
        // camera would be unclickable (bevy_ui feeds a cursor only to window
        // cameras). The blit camera owns the UI and targets the window.
        assert!(
            app.world().entity(cam).get::<IsDefaultUiCamera>().is_none(),
            "scenario camera must not be the default UI camera on Low"
        );
        let blit = state.upscale_camera.unwrap();
        assert!(
            app.world()
                .entity(blit)
                .get::<IsDefaultUiCamera>()
                .is_some(),
            "the blit (window) camera is the default UI camera"
        );
        assert!(
            matches!(
                app.world().entity(blit).get::<RenderTarget>(),
                Some(RenderTarget::Window(_))
            ),
            "the UI camera must target the window so clicks register"
        );

        // Exactly one blit camera + one full-window sprite.
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
    fn every_target_switch_marks_the_camera_projection_changed() {
        // The load-bearing fix for live quality switching: bevy's `camera_system`
        // re-derives a camera's target info only when the target content changes,
        // the camera is added, or its Projection changed - NOT when `RenderTarget`
        // is swapped in place. So each switch must touch the Projection, or a live
        // High<->Low change renders at the stale resolution. Spy on the scenario
        // camera's Projection change tick right after the reconcile runs.
        #[derive(Resource, Default)]
        struct ProjChanged(bool);
        fn spy(
            q: Query<Ref<Projection>, With<ScenarioCameraMarker>>,
            mut out: ResMut<ProjChanged>,
        ) {
            out.0 = q.iter().any(|p| p.is_changed());
        }

        let mut app = test_app(GraphicsQuality::High);
        app.init_resource::<ProjChanged>();
        app.add_systems(Update, spy.after(reconcile_render_scale));
        spawn_scenario_camera(&mut app);
        // Frame 1: camera just spawned (Projection is trivially "added"); ignore.
        app.update();

        // High -> Low: reconcile points the camera at the image and must mark the
        // projection changed (the camera was added a frame ago, so a true here is
        // the reconcile's doing, not is_added).
        app.insert_resource(GraphicsBudget::for_quality(GraphicsQuality::Low));
        app.update();
        assert!(
            app.world().resource::<ProjChanged>().0,
            "switching to Low must mark the scenario camera's projection changed"
        );

        // Low -> High: teardown resets the target and must also mark it changed,
        // else the window keeps rendering at the reduced resolution.
        app.insert_resource(GraphicsBudget::for_quality(GraphicsQuality::High));
        app.update();
        assert!(
            app.world().resource::<ProjChanged>().0,
            "switching back to High must mark the scenario camera's projection changed"
        );

        // A steady frame at High (no switch) must NOT keep marking it changed.
        app.update();
        assert!(
            !app.world().resource::<ProjChanged>().0,
            "a steady frame must not churn the projection change tick"
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
