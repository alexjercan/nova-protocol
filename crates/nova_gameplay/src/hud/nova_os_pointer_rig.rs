//! Live-tree rig for the NOVA OS's forwarded pointer (task 20260730-123039).
//!
//! Test-only support module. It stands up the real RTT composite - an offscreen
//! image, its dedicated UI camera, the content root that renders through it, and
//! the window-space sampling surface that displays it - then drives the REAL
//! `forward_nova_os_pointer` from a window cursor position and lets bevy's own
//! `ui_picking` decide what was hit.
//!
//! Why a live tree rather than asserting on the mapping helper: the bug this rig
//! was born for is that a click at a known place on the glass selected the wrong
//! thing (or nothing). Only the whole chain - window rect -> screen uv -> CRT
//! mapping -> image pixels -> UI stack -> `Activate` - produces that answer. A
//! helper-level assert would have passed happily against a pointer that agreed
//! with itself.
//!
//! The rig deliberately knows NOTHING about `nova_os_crt_screen_to_image_uv`:
//! callers say where on the glass they click and which image point the CRT is
//! showing there, and the second number comes from a transcription of the WGSL
//! ([`shader_sample_uv_reference`] / [`shader_draws_at`] below), never from the
//! production helper under test.

use bevy::{
    asset::{AssetPlugin, Assets},
    camera::{
        visibility::RenderLayers, Camera, ComputedCameraValues, ImageRenderTarget, RenderTarget,
        RenderTargetInfo,
    },
    image::{Image, TextureAtlasLayout},
    input::{
        mouse::{MouseButton, MouseButtonInput},
        ButtonState,
    },
    math::{UVec2, Vec2},
    picking::pointer::{Location, PointerLocation},
    prelude::*,
    text::TextPlugin,
    ui::UiPlugin,
    ui_widgets::ButtonPlugin,
    window::{PrimaryWindow, Window, WindowResolution},
};

use super::nova_os::{
    forward_nova_os_pointer, nova_os_image_target, nova_os_new_target_image, nova_os_pointer_id,
    NovaOsForwardedPointerMarker, NovaOsImageContentRootMarker, NovaOsRtt,
    NovaOsSamplingSurfaceMarker, NOVA_OS_RTT_LAYER,
};

/// Agreement budget between the pointer's mapping and the shader's, in image
/// pixels, over the whole screen. Half a pixel: below the level anything can be
/// clicked at, and far below the 12 px blips this bug lost.
pub(super) const CRT_MAP_BUDGET_PX: f32 = 0.5;

/// Independent transcription of `assets/shaders/nova_os_crt.wgsl`'s sample-UV
/// chain: the power-collapse remap, then `barrel()`, then the overscan pull. It
/// answers "which image texel does the CRT DISPLAY under this point on the
/// glass?", which is what the forwarded pointer has to agree with.
///
/// Written from the shader source, NOT from `nova_os_crt_screen_to_image_uv`, so
/// a production helper that drifts from the picture cannot satisfy it
/// (`test-must-not-reuse-the-formula-under-test`). The degauss shear is
/// deliberately absent - see that helper's docs.
pub(super) fn shader_sample_uv_reference(uv: Vec2, warp: f32, overscan: f32, power: f32) -> Vec2 {
    let sample_uv = shader_collapse_remap(uv, power);
    // fn barrel(uv, amount) { centered * (1.0 + amount * dot(centered, centered)) }
    let centered = sample_uv - Vec2::splat(0.5);
    let warped_raw = Vec2::splat(0.5) + centered * (1.0 + warp * centered.length_squared());
    // let warped = (warped_raw - 0.5) * material.overscan + 0.5;
    (warped_raw - Vec2::splat(0.5)) * overscan + Vec2::splat(0.5)
}

/// The shader's answer to "is anything DRAWN at this point on the glass, and if
/// so from which image texel?" - i.e. [`shader_sample_uv_reference`] plus the two
/// gates the fragment multiplies its output by:
///
/// ```wgsl
/// if cy < 0.0 || cy > 1.0 || cx < 0.0 || cx > 1.0 { collapsed = 1.0; }
/// let in_bounds = f32(warped.x >= 0.0 && ... && warped.y <= 1.0);
/// rgb = (rgb + analog_add + rim_add) * in_bounds * (1.0 - collapsed);
/// ```
///
/// These are two SEPARATE tests and neither implies the other (review R2.1):
/// barrel-then-overscan is a net contraction here (0.93 against a barrel factor
/// under 1.06), so a `cx` just past 1 still lands inside `[0,1]` after warping.
/// A reference that checked only `in_bounds` disagreed with the (correct)
/// production helper at 186 grid points at power 0.15 and 814 at 0.35 on a
/// 201x201 grid - invisible at 17x17 purely by luck of the sampling.
pub(super) fn shader_draws_at(uv: Vec2, warp: f32, overscan: f32, power: f32) -> Option<Vec2> {
    let sample_uv = shader_collapse_remap(uv, power);
    let collapsed = !(0.0..=1.0).contains(&sample_uv.x) || !(0.0..=1.0).contains(&sample_uv.y);
    let warped = shader_sample_uv_reference(uv, warp, overscan, power);
    let in_bounds = (0.0..=1.0).contains(&warped.x) && (0.0..=1.0).contains(&warped.y);
    (!collapsed && in_bounds).then_some(warped)
}

/// The shader's power-collapse remap, transcribed ONCE (review R3.1) - both the
/// sample-UV reference and the draws-here reference read it, so a future shader
/// change to an edge or the epsilon cannot update one copy and leave the other
/// stale while the pair still looks self-consistent.
///
/// ```wgsl
/// let open_h = smoothstep(0.0, 0.65, material.power);
/// let open_w = smoothstep(0.0, 0.28, material.power);
/// let cy = (in.uv.y - 0.5) / max(open_h, 0.0008) + 0.5;
/// let cx = (in.uv.x - 0.5) / max(open_w, 0.0008) + 0.5;
/// ```
fn shader_collapse_remap(uv: Vec2, power: f32) -> Vec2 {
    let smoothstep = |edge1: f32, x: f32| {
        let t = (x / edge1).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    };
    let open_h = smoothstep(0.65, power);
    let open_w = smoothstep(0.28, power);
    Vec2::new(
        (uv.x - 0.5) / open_w.max(0.0008) + 0.5,
        (uv.y - 0.5) / open_h.max(0.0008) + 0.5,
    )
}

/// A grid of screen-local UVs covering centre, edges and corners.
///
/// Fine enough to land inside the narrow band where the shader's `collapsed` and
/// `in_bounds` gates disagree (review R2.1): a 17x17 grid missed it entirely at
/// every swept power, which is exactly how a reference that modelled only one of
/// the two gates passed.
pub(super) fn crt_uv_grid() -> Vec<Vec2> {
    let mut grid = Vec::new();
    const STEPS: usize = 200;
    for iy in 0..=STEPS {
        for ix in 0..=STEPS {
            grid.push(Vec2::new(
                ix as f32 / STEPS as f32,
                iy as f32 / STEPS as f32,
            ));
        }
    }
    grid
}

/// The image PIXEL the CRT displays under a point on the glass, per the shader
/// reference. This is how a test places content where it intends to click it.
pub(super) fn image_px_shown_at(glass_uv: Vec2) -> Vec2 {
    shader_uv_at(glass_uv) * RIG_IMAGE.as_vec2()
}

/// Where on the glass a given image PIXEL appears, i.e. the numeric inverse of
/// [`image_px_shown_at`]. Solved by fixed-point iteration on the shader
/// reference, so a test can aim at something whose position it only knows in
/// image space without ever calling the production mapping.
///
/// (The forward map is a mild contraction here - overscan 0.93 times a barrel
/// factor under 1.06 - so the plain iteration converges in a handful of steps.)
pub(super) fn glass_uv_showing(image_px: Vec2) -> Vec2 {
    let target_uv = image_px / RIG_IMAGE.as_vec2();
    let mut uv = target_uv;
    for _ in 0..64 {
        let error = target_uv - shader_uv_at(uv);
        if error.abs().max_element() < 1e-7 {
            return uv;
        }
        uv += error;
    }
    panic!("no point on the glass shows image px {image_px:?} - is it off the picture?");
}

/// [`image_px_shown_at`] in uv, at the live warp/overscan and full power.
fn shader_uv_at(glass_uv: Vec2) -> Vec2 {
    shader_sample_uv_reference(
        glass_uv,
        super::nova_os::NOVA_OS_CRT_WARP,
        super::nova_os::NOVA_OS_CRT_OVERSCAN,
        1.0,
    )
}

/// Offscreen image size, and the on-glass size of the sampling surface. The
/// production `reconcile_nova_os_target` keeps these 1:1, so the rig does too -
/// any scale mismatch would hide a mapping error behind a second conversion.
pub(super) const RIG_IMAGE: UVec2 = UVec2::new(1280, 720);

/// The window the glass is inset in. Deliberately larger than the panel and
/// off-centre, so a mapping that quietly assumes the panel is the whole window,
/// or is centred in it, fails here.
pub(super) const RIG_WINDOW: UVec2 = UVec2::new(1600, 900);

/// Top-left of the sampling surface in window pixels. Asymmetric on purpose (see
/// [`RIG_WINDOW`]).
pub(super) const RIG_PANEL_MIN: Vec2 = Vec2::new(210.0, 120.0);

/// The live composite plus the handles a test needs to place content and click.
pub(super) struct NovaOsPointerRig {
    pub app: App,
    /// The subtree that renders THROUGH the image - where terminal/app content
    /// goes. Sized to [`RIG_IMAGE`] with its origin at the image's top-left, so a
    /// child at `Val::Px(p)` sits at image pixel `p`.
    pub content_root: Entity,
}

/// Window pixel for a point on the glass, given in screen-local uv (0,0 top-left
/// of the sampling surface .. 1,1 bottom-right).
pub(super) fn glass_px(uv: Vec2) -> Vec2 {
    RIG_PANEL_MIN + uv * RIG_IMAGE.as_vec2()
}

/// Build the composite. `MinimalPlugins` + the real UI/text/picking/widget stack,
/// so layout, the UI stack, hit testing and `Activate` are all the shipped ones.
pub(super) fn nova_os_pointer_rig() -> NovaOsPointerRig {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        TransformPlugin,
        bevy::a11y::AccessibilityPlugin,
        // `ui_picking` refuses any node whose `InheritedVisibility` is not
        // computed-true, and nothing else in a MinimalPlugins app propagates it.
        bevy::camera::visibility::VisibilityPlugin,
        bevy::input::InputPlugin,
        bevy::picking::PickingPlugin,
        bevy::picking::InteractionPlugin,
        TextPlugin,
        UiPlugin,
        // `Button` -> `Activate` lives here; in the game it arrives with
        // DefaultPlugins' `UiWidgetsPlugins`
        // (`rtt-ui-select-via-activate-not-interaction`). Only the button half is
        // added: the group's text-input plugin wants IME messages and an
        // `InputFocus` resource this rig has no window backend to supply.
        ButtonPlugin,
    ));
    app.init_asset::<Image>().init_asset::<TextureAtlasLayout>();
    // `VisibilityPlugin`'s bounds pass reads the mesh collections, which in the
    // game arrive with the render plugins this rig has no use for.
    app.init_asset::<Mesh>()
        .init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();

    // The primary window: `ui_picking` normalizes a camera's window target
    // against it, and `forward_nova_os_pointer` reads the cursor from it.
    app.world_mut().spawn((
        Window {
            resolution: WindowResolution::new(RIG_WINDOW.x, RIG_WINDOW.y),
            ..default()
        },
        PrimaryWindow,
    ));

    // The window camera the on-glass UI (the sampling surface) lays out against.
    app.world_mut().spawn((
        Camera2d,
        Camera {
            order: 0,
            computed: ComputedCameraValues {
                target_info: Some(RenderTargetInfo {
                    physical_size: RIG_WINDOW,
                    scale_factor: 1.0,
                }),
                ..default()
            },
            ..default()
        },
        IsDefaultUiCamera,
    ));

    // The offscreen image and its dedicated UI camera - the production pipeline
    // from `setup_nova_os`, minus the render plugins that would fill target_info.
    let image = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(nova_os_new_target_image(RIG_IMAGE));
    let camera = app
        .world_mut()
        .spawn((
            Camera2d,
            Camera {
                order: -1,
                computed: ComputedCameraValues {
                    target_info: Some(RenderTargetInfo {
                        physical_size: RIG_IMAGE,
                        scale_factor: 1.0,
                    }),
                    ..default()
                },
                ..default()
            },
            RenderTarget::Image(ImageRenderTarget {
                handle: image.clone(),
                scale_factor: 1.0,
            }),
            RenderLayers::layer(NOVA_OS_RTT_LAYER),
        ))
        .id();
    let content_root = app
        .world_mut()
        .spawn((
            NovaOsImageContentRootMarker,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                width: Val::Px(RIG_IMAGE.x as f32),
                height: Val::Px(RIG_IMAGE.y as f32),
                ..default()
            },
            UiTargetCamera(camera),
            RenderLayers::layer(NOVA_OS_RTT_LAYER),
        ))
        .id();

    // The glass: a window-space node displaying the image. `Pickable::IGNORE` as
    // in production - the surface itself must never swallow a click.
    app.world_mut().spawn((
        NovaOsSamplingSurfaceMarker,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(RIG_PANEL_MIN.x),
            top: Val::Px(RIG_PANEL_MIN.y),
            width: Val::Px(RIG_IMAGE.x as f32),
            height: Val::Px(RIG_IMAGE.y as f32),
            ..default()
        },
        Pickable::IGNORE,
    ));

    let pointer = app
        .world_mut()
        .spawn((
            NovaOsForwardedPointerMarker,
            nova_os_pointer_id(),
            PointerLocation::new(Location {
                target: nova_os_image_target(&image),
                position: Vec2::splat(-1000.0),
            }),
        ))
        .id();
    app.insert_resource(NovaOsRtt {
        image,
        camera,
        content_root,
        pointer,
    });

    // Production schedules the forwarder in `Update`, so its `PointerInput`s are
    // consumed by picking on the FOLLOWING frame. Keep that shape: a rig that
    // ran it in `PreUpdate` would prove a timing the game does not have.
    app.add_systems(Update, forward_nova_os_pointer);

    settle(&mut app);
    NovaOsPointerRig { app, content_root }
}

/// Run enough frames for layout, the UI stack and the pointer's one-frame lag to
/// settle.
pub(super) fn settle(app: &mut App) {
    for _ in 0..4 {
        app.update();
    }
}

/// Move the real mouse to a window pixel and let the forwarded pointer follow.
pub(super) fn move_cursor_to(rig: &mut NovaOsPointerRig, window_px: Vec2) {
    let mut window = rig
        .app
        .world_mut()
        .query_filtered::<&mut Window, With<PrimaryWindow>>()
        .single_mut(rig.app.world_mut())
        .expect("the rig has a primary window");
    window.set_physical_cursor_position(Some(window_px.as_dvec2()));
    settle(&mut rig.app);
}

/// Press and release the left mouse button at `window_px`, exactly as the player
/// does: the cursor moves, then real `MouseButtonInput` messages arrive and
/// `forward_nova_os_pointer` mirrors them onto the image-targeted pointer.
pub(super) fn click_at(rig: &mut NovaOsPointerRig, window_px: Vec2) {
    move_cursor_to(rig, window_px);
    for state in [ButtonState::Pressed, ButtonState::Released] {
        let window = rig
            .app
            .world_mut()
            .query_filtered::<Entity, With<PrimaryWindow>>()
            .single(rig.app.world())
            .expect("the rig has a primary window");
        rig.app.world_mut().write_message(MouseButtonInput {
            button: MouseButton::Left,
            state,
            window,
        });
        settle(&mut rig.app);
    }
}

/// The image pixel the forwarded pointer currently sits on, for recording a miss
/// distance in a failure message.
pub(super) fn pointer_image_px(rig: &NovaOsPointerRig) -> Option<Vec2> {
    rig.app
        .world()
        .get::<PointerLocation>(rig.app.world().resource::<NovaOsRtt>().pointer)
        .and_then(|l| l.location.as_ref().map(|l| l.position))
}
