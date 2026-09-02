//! The CRT illusion: the barrel-warp `UiMaterial`, and the render-to-texture
//! rig that gives it something to sample.
//!
//! The terminal subtree is drawn by its own camera into an image on
//! [`NOVA_OS_RTT_LAYER`], and the on-screen surface samples that image through
//! the shader - so warp and scanlines apply to real, live UI. Pointer events
//! land on the flat surface and are forwarded into the warped subtree.
//!
//! Touch this module when changing the screen distortion or the RTT wiring.

use bevy::{
    asset::uuid::Uuid,
    camera::{ImageRenderTarget, NormalizedRenderTarget},
    input::ButtonState,
    picking::{
        hover::{HoverMap, Hovered},
        pointer::{
            Location, PointerAction, PointerButton, PointerId, PointerInput, PointerLocation,
        },
    },
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType, TextureFormat},
    shader::ShaderRef,
    ui_render::prelude::{MaterialNode, UiMaterial},
};

use super::{components::*, style::*};

#[derive(Asset, AsBindGroup, TypePath, Clone, Debug)]
pub(crate) struct NovaOsCrtMaterial {
    #[uniform(0)]
    pub(crate) data: NovaOsCrtUniform,
    /// The offscreen image holding the rendered terminal content. The sampling
    /// shader reads it to bloom the glyphs and barrel-warp the content. A default
    /// (white 1x1) handle in headless rigs keeps the material valid.
    #[texture(1)]
    #[sampler(2)]
    pub(crate) source: Handle<Image>,
}

#[derive(ShaderType, Clone, Debug)]
pub(crate) struct NovaOsCrtUniform {
    pub(crate) tint: LinearRgba,
    /// The CRT panel's pixel size, updated each frame by [`animate_nova_os_crt`]
    /// from the screen node's [`ComputedNode`] so the scanlines/slot-mask + bloom
    /// taps track the real screen size. Zero until the first layout pass feeds it.
    pub(crate) resolution: Vec2,
    pub(crate) scanline_strength: f32,
    pub(crate) vignette_strength: f32,
    pub(crate) glow_strength: f32,
    pub(crate) grain_strength: f32,
    /// Real-time seconds, updated each frame by [`animate_nova_os_crt`] so the
    /// grain shimmers gently.
    pub(crate) time: f32,
    /// Rounded-corner radius in screen pixels. A UI `MaterialNode` is NOT
    /// clipped by its node's [`BorderRadius`], so the shader masks its own
    /// corners to the screen's rounding (no green bleed past the rounded edge).
    /// Zero disables the mask (headless/other rigs).
    pub(crate) corner_radius: f32,
    /// Barrel-distortion amount (0 = flat) - bows the sampled content.
    pub(crate) warp: f32,
    /// Bloom strength (halo of the bright green glyphs).
    pub(crate) bloom: f32,
    /// Power level 0..1 fed from [`NovaOsOpenness`]: 1 full raster, 0 collapsed to
    /// a dying line/dot. Drives the CRT power-on/off collapse.
    pub(crate) power: f32,
    /// Extra brightness multiply (1.0 neutral). Reserved for task 214617's BRIGHT
    /// knob. Appended last so the field order still matches the WGSL struct.
    pub(crate) brightness: f32,
    /// Degauss envelope 0..1: pulsed to 1 on an app
    /// launch/exit/switch by [`super::shell::sync_nova_os_app_ui`] via [`NovaOsDegauss`] and
    /// decayed back to 0 by [`animate_nova_os_crt`]. Drives the shader's wobble +
    /// flash. Appended last so the field order still matches the WGSL struct
    /// (trailing `f32` after `brightness` - no alignment hole,
    /// `shader-uniform-field-order-must-match-wgsl`).
    pub(crate) degauss: f32,
    /// Overscan pull applied AFTER the barrel warp, from [`NOVA_OS_CRT_OVERSCAN`].
    /// Lives in the uniform rather than as a WGSL constant
    /// so the shader and [`nova_os_crt_screen_to_image_uv`] share one definition.
    /// Appended last so the field order still matches the WGSL struct.
    pub(crate) overscan: f32,
}

impl Default for NovaOsCrtMaterial {
    fn default() -> Self {
        Self {
            data: NovaOsCrtUniform {
                tint: NOVA_OS_CRT_TINT,
                resolution: Vec2::ZERO,
                scanline_strength: NOVA_OS_CRT_SCANLINE_STRENGTH,
                vignette_strength: NOVA_OS_CRT_VIGNETTE_STRENGTH,
                glow_strength: NOVA_OS_CRT_GLOW_STRENGTH,
                grain_strength: NOVA_OS_CRT_GRAIN_STRENGTH,
                time: 0.0,
                corner_radius: NOVA_OS_SCREEN_RADIUS_PX,
                warp: NOVA_OS_CRT_WARP,
                bloom: NOVA_OS_CRT_BLOOM,
                // Start collapsed; the openness driver blooms it on.
                power: 0.0,
                brightness: 1.0,
                // Idle: no degauss until an app launch/exit pulses it.
                degauss: 0.0,
                overscan: NOVA_OS_CRT_OVERSCAN,
            },
            source: Handle::default(),
        }
    }
}

impl UiMaterial for NovaOsCrtMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/nova_os_crt.wgsl".into()
    }
}
// Render-to-texture CRT pipeline.
//
// The terminal-content subtree renders to an offscreen image via a dedicated UI
// camera; the screen node then displays that image through the sampling
// `NovaOsCrtMaterial` (bloom + barrel warp + the whole CRT treatment). Interaction
// is preserved by FORWARDING a custom pointer whose location targets the image
// (bevy 0.19 `ui_picking` matches pointers to cameras by render target), plus a
// hover-mirror system because `bevy_picking::update_is_hovered` only tracks the
// mouse pointer.

/// The dedicated UI camera + content subtree live on this render layer so the
/// image camera draws ONLY the terminal UI, never stray world 2D sprites (the
/// render-scale upscale sprite sits on the default layer 0).
pub(crate) const NOVA_OS_RTT_LAYER: usize = 20;
/// Camera order for the offscreen pass: well before the window/UI cameras so the
/// sampled image is ready when the screen surface reads it.
pub(crate) const NOVA_OS_RTT_CAMERA_ORDER: isize = -20;

#[derive(Component)]
pub(crate) struct NovaOsImageCameraMarker;

#[derive(Component)]
pub(crate) struct NovaOsImageContentRootMarker;

/// The screen-node surface that samples the offscreen image through the CRT shader.
#[derive(Component)]
pub(crate) struct NovaOsSamplingSurfaceMarker;

#[derive(Component)]
pub(crate) struct NovaOsForwardedPointerMarker;

/// Handles/entities of the live NOVA OS's RTT pipeline. Present only on
/// render-capable builds (an `Assets<Image>` + `Assets<NovaOsCrtMaterial>` exist);
/// absent headless, where the terminal renders directly on the screen node.
#[derive(Resource)]
pub(crate) struct NovaOsRtt {
    pub(crate) image: Handle<Image>,
    pub(crate) camera: Entity,
    pub(crate) content_root: Entity,
    pub(crate) pointer: Entity,
}

/// Stable id for the forwarded pointer (one NOVA OS at a time).
///
/// Public because it is the only handle on the pointer that reaches the
/// offscreen tree: bevy's own [`HoverMap`] is keyed by [`PointerId`], so this is
/// what lets a caller ask "did the click get THROUGH the glass?" rather than
/// "did it land on the glass?" - two answers a run driving the monitor has to
/// tell apart.
pub fn nova_os_pointer_id() -> PointerId {
    PointerId::Custom(Uuid::from_u128(0x0BADC0DE_CAFE_1234_5678_9ABCDEF01234))
}

pub(crate) fn nova_os_image_target(image: &Handle<Image>) -> NormalizedRenderTarget {
    NormalizedRenderTarget::Image(ImageRenderTarget {
        handle: image.clone(),
        scale_factor: 1.0,
    })
}

pub(crate) fn nova_os_new_target_image(size: UVec2) -> Image {
    Image::new_target_texture(
        size.x.max(1),
        size.y.max(1),
        TextureFormat::Rgba8UnormSrgb,
        None,
    )
}

/// Keep the offscreen image sized to the screen node's physical pixels and the
/// content root sized to match, so window resizes / relayouts never show a
/// stretched frame (mirrors `render_scale.rs`). Deactivate the offscreen pass and
/// hide the content while the NOVA OS is fully closed so it costs nothing.
pub(crate) fn reconcile_nova_os_target(
    rtt: Option<Res<NovaOsRtt>>,
    mut images: ResMut<Assets<Image>>,
    q_screen: Query<&ComputedNode, With<NovaOsScreenMarker>>,
    q_openness: Query<&NovaOsOpenness, With<NovaOsRootMarker>>,
    mut q_camera: Query<(&mut Camera, &mut Projection), With<NovaOsImageCameraMarker>>,
    mut q_root: Query<(&mut Node, &mut Visibility), With<NovaOsImageContentRootMarker>>,
) {
    let Some(rtt) = rtt else {
        return;
    };
    let camera = rtt.camera;
    let Ok(computed) = q_screen.single() else {
        return;
    };
    // ComputedNode.size is physical pixels; the image target renders 1:1 at that
    // size (scale_factor 1.0), so content laid out at the image logical size lines
    // up with the sampled surface.
    let desired = computed.size().round().as_uvec2().max(UVec2::ONE);
    let open = q_openness.iter().next().map(|o| o.0).unwrap_or(0.0);

    let needs_resize = images
        .get(&rtt.image)
        .map(|img| img.size() != desired)
        .unwrap_or(true);
    if needs_resize {
        if let Some(mut img) = images.get_mut(&rtt.image) {
            img.resize(bevy::render::render_resource::Extent3d {
                width: desired.x,
                height: desired.y,
                depth_or_array_layers: 1,
            });
        }
        // Force the camera to re-derive its target info after the swap
        // (`bevy-camera-ignores-runtime-rendertarget-swap`).
        if let Ok((_, mut projection)) = q_camera.get_mut(camera) {
            projection.set_changed();
        }
    }

    if let Ok((mut cam, _)) = q_camera.get_mut(camera) {
        // No point rendering the offscreen pass when the NOVA OS is fully closed.
        cam.is_active = open > f32::EPSILON;
    }
    if let Ok((mut node, mut vis)) = q_root.single_mut() {
        // This system's only gate is `resource_exists::<NovaOsRtt>`, which holds
        // from ship spawn to despawn - so it runs every frame the player is flying,
        // over a subtree of hundreds of `Text` children. An unguarded `Node` write
        // re-lays out all of them for a size that almost never changes.
        let (width, height) = (Val::Px(desired.x as f32), Val::Px(desired.y as f32));
        if node.width != width || node.height != height {
            node.width = width;
            node.height = height;
        }
        vis.set_if_neq(if open > f32::EPSILON {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
}

/// Forward the real mouse cursor onto the offscreen image so the terminal UI
/// stays hoverable/clickable through the sampled surface: map the cursor into the
/// screen node's rect, through the CRT composite's screen->image mapping and into
/// image pixels, write the custom pointer's location, and mirror mouse button
/// presses as `PointerInput`.
///
/// The mapping is [`nova_os_crt_screen_to_image_uv`], the SAME chain the shader
/// displays with, fed the same uniforms - so the pointer lands on the thing the
/// player sees under the cursor. NOTE: applying the barrel INVERSE, or skipping
/// the overscan, puts clicks up to 27 px from their target at the screen corners.
///
/// The buttons are read off [`WindowEvent`], the same stream
/// `bevy_picking::input::mouse_pick_events` builds the MOUSE pointer's presses
/// from, so the two pointers cannot disagree about whether a button went down.
/// `bevy_winit` also writes a concrete `MouseButtonInput` twin for every real
/// click, which is close enough to be tempting and wrong: a SYNTHESIZED click
/// writes only the half picking reads, so a forwarder reading the twin went
/// dead under every driven run while looking perfectly correct by hand
/// (task 20260804-134347).
pub(crate) fn forward_nova_os_pointer(
    rtt: Option<Res<NovaOsRtt>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut window_events: MessageReader<bevy::window::WindowEvent>,
    q_surface: Query<(&ComputedNode, &UiGlobalTransform), With<NovaOsSamplingSurfaceMarker>>,
    q_openness: Query<&NovaOsOpenness, With<NovaOsRootMarker>>,
    mut q_pointer: Query<&mut PointerLocation, With<NovaOsForwardedPointerMarker>>,
    mut pointer_inputs: MessageWriter<PointerInput>,
    images: Res<Assets<Image>>,
) {
    let Some(rtt) = rtt else {
        return;
    };
    let Ok(mut loc) = q_pointer.get_mut(rtt.pointer) else {
        return;
    };
    let image_size = images
        .get(&rtt.image)
        .map(|i| i.size().as_vec2())
        .unwrap_or(Vec2::ONE);
    // The same openness `animate_nova_os_crt` feeds the shader's `power` uniform,
    // so a half-collapsed raster is clicked where it is actually drawn. Read it
    // exactly as that system does - `iter().next()`, not `single()`, which would
    // fall back to a full raster on the very frame a second shell entity makes
    // the shader pick the first one's openness (review R1.2). Absent on rigs
    // without the shell entity: treat that as a full raster.
    let power = q_openness.iter().next().map(|o| o.0).unwrap_or(1.0);

    let cursor = windows.single().ok().and_then(|w| w.cursor_position());
    let surface = q_surface.single().ok();
    let in_image = match (cursor, surface) {
        (Some(cursor), Some((node, xf))) => {
            nova_os_glass_local_uv(cursor, node, xf).and_then(|l| {
                nova_os_crt_screen_to_image_uv(l, NOVA_OS_CRT_WARP, NOVA_OS_CRT_OVERSCAN, power)
                    .map(|uv| uv * image_size)
            })
        }
        _ => None,
    };

    // Park off-image when the cursor is not over the panel so nothing is hovered.
    let position = in_image.unwrap_or(Vec2::splat(-1000.0));
    loc.location = Some(Location {
        target: nova_os_image_target(&rtt.image),
        position,
    });

    // Mirror mouse buttons onto the forwarded pointer (only meaningful over the
    // panel; harmless otherwise since the position is parked off-image).
    let id = nova_os_pointer_id();
    for ev in window_events.read().filter_map(|event| match event {
        bevy::window::WindowEvent::MouseButtonInput(input) => Some(input),
        _ => None,
    }) {
        let button = match ev.button {
            MouseButton::Left => PointerButton::Primary,
            MouseButton::Right => PointerButton::Secondary,
            MouseButton::Middle => PointerButton::Middle,
            _ => continue,
        };
        let action = match ev.state {
            ButtonState::Pressed => PointerAction::Press(button),
            ButtonState::Released => PointerAction::Release(button),
        };
        pointer_inputs.write(PointerInput::new(
            id,
            Location {
                target: nova_os_image_target(&rtt.image),
                position,
            },
            action,
        ));
    }
}

/// Where on the offscreen image the CRT composite DISPLAYS the content sitting
/// under screen-local uv `uv` - the mapping the forwarded pointer must follow to
/// land on the thing the player is actually looking at.
///
/// This is a mirror of the sample-UV chain in `assets/shaders/nova_os_crt.wgsl`'s
/// fragment: the power-collapse remap, then `barrel()`, then the overscan pull
/// back toward centre. Both `warp` and `overscan` are uniforms filled from
/// [`NOVA_OS_CRT_WARP`] / [`NOVA_OS_CRT_OVERSCAN`], so there is ONE definition of
/// each constant and the pointer cannot silently drift from the picture.
///
/// Not a mirror of the degauss shear: that is a sub-frame transient (a decaying
/// horizontal wobble peaking at 6 px, pulsed only by an app launch/exit/switch and
/// gone within [`NOVA_OS_DEGAUSS_DURATION`]), and chasing it would make the
/// pointer jitter during the very moments the content is being replaced anyway.
/// Returns `None` when the glass shows no picture at `uv` - the tube-black
/// margin outside the collapsing raster while the CRT powers on or off. Nothing
/// is displayed there, so nothing may be clicked there either.
pub(crate) fn nova_os_crt_screen_to_image_uv(
    uv: Vec2,
    warp: f32,
    overscan: f32,
    power: f32,
) -> Option<Vec2> {
    // Power-on/off raster collapse: the picture squeezes toward the centre scan
    // line, so the glass shows a SMALLER window onto the same image.
    let open_h = nova_os_smoothstep(NOVA_OS_CRT_POWER_OPEN_H, power);
    let open_w = nova_os_smoothstep(NOVA_OS_CRT_POWER_OPEN_W, power);
    let sample = Vec2::new(
        (uv.x - 0.5) / open_w.max(NOVA_OS_CRT_POWER_EPSILON) + 0.5,
        (uv.y - 0.5) / open_h.max(NOVA_OS_CRT_POWER_EPSILON) + 0.5,
    );
    if sample.cmplt(Vec2::ZERO).any() || sample.cmpgt(Vec2::ONE).any() {
        return None;
    }

    // Barrel: push outward from centre by r^2, then overscan: pull the bowed
    // result back in so it lands inside the picture.
    let centred = sample - Vec2::splat(0.5);
    let bowed = centred * (1.0 + warp * centred.length_squared());
    let warped = bowed * overscan + Vec2::splat(0.5);
    (!warped.cmplt(Vec2::ZERO).any() && !warped.cmpgt(Vec2::ONE).any()).then_some(warped)
}

/// The shader's `smoothstep(0.0, edge1, x)`.
pub(crate) fn nova_os_smoothstep(edge1: f32, x: f32) -> f32 {
    let t = (x / edge1).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Which screen-local uv the CRT composite displays `uv` of the offscreen image
/// at - the inverse of [`nova_os_crt_screen_to_image_uv`], and what a caller
/// aiming at something it can only locate in IMAGE space needs.
///
/// The overscan pull and the collapse remap are plain scales, so both invert in
/// closed form. The barrel is radial - `|bowed| = r * (1 + warp * r^2)` for
/// `r = |centred|`, strictly increasing for a non-negative `warp` - so the
/// radius is a Newton solve on that cubic, whose derivative `3*warp*r^2 + 1` is
/// never below 1 and therefore converges from any start.
///
/// `None` when the CRT displays that image point nowhere. Two ways to earn
/// that, and only two: the point is not part of the picture at all, or the
/// barrel bowed it past the drawn rim and the overscan left it under the bezel -
/// the outer few percent of the image, which the tube crops on purpose.
///
/// The raster collapse is NOT one of them. It squeezes where the picture is
/// drawn toward the centre scan line without cropping it, so a half-powered tube
/// still shows every image point, just crowded into a band - which is why the
/// only gate here is on the pre-collapse `sample`.
pub(crate) fn nova_os_crt_image_uv_to_screen(
    uv: Vec2,
    warp: f32,
    overscan: f32,
    power: f32,
) -> Option<Vec2> {
    if uv.cmplt(Vec2::ZERO).any() || uv.cmpgt(Vec2::ONE).any() {
        return None;
    }
    let bowed = (uv - Vec2::splat(0.5)) / overscan;
    let bowed_radius = bowed.length();
    let centred = if bowed_radius <= f32::EPSILON {
        Vec2::ZERO
    } else {
        bowed * (nova_os_unbow_radius(bowed_radius, warp) / bowed_radius)
    };

    // The rim gets slack because the unbow is a numeric solve: a point the
    // forward mapping put EXACTLY on the picture's edge comes back a rounding
    // step outside it, and an exact compare would make the outermost row of the
    // terminal the one thing on the glass nobody can aim at.
    let sample = centred + Vec2::splat(0.5);
    if sample.cmplt(Vec2::splat(-NOVA_OS_RIM_SLACK)).any()
        || sample.cmpgt(Vec2::splat(1.0 + NOVA_OS_RIM_SLACK)).any()
    {
        return None;
    }
    let open_h = nova_os_smoothstep(NOVA_OS_CRT_POWER_OPEN_H, power);
    let open_w = nova_os_smoothstep(NOVA_OS_CRT_POWER_OPEN_W, power);
    // No second gate: a `sample` inside the picture and an `open` of at most 1
    // put this inside the glass by construction.
    Some(Vec2::new(
        centred.x * open_w.max(NOVA_OS_CRT_POWER_EPSILON) + 0.5,
        centred.y * open_h.max(NOVA_OS_CRT_POWER_EPSILON) + 0.5,
    ))
}

/// Slack on the picture's rim, in uv - a tenth of an image pixel at any size the
/// monitor is ever rendered at, and far under the half-pixel the forwarded
/// pointer is held to.
const NOVA_OS_RIM_SLACK: f32 = 1e-4;

/// Solve `r * (1 + warp * r^2) = bowed_radius` for `r >= 0` by Newton from
/// `bowed_radius` itself - an over-estimate for `warp >= 0`, which is the only
/// side the tube is ever tuned to.
fn nova_os_unbow_radius(bowed_radius: f32, warp: f32) -> f32 {
    let mut radius = bowed_radius;
    for _ in 0..NOVA_OS_UNBOW_STEPS {
        let error = radius * (1.0 + warp * radius * radius) - bowed_radius;
        if error.abs() <= NOVA_OS_UNBOW_TOLERANCE {
            break;
        }
        radius -= error / (1.0 + 3.0 * warp * radius * radius);
    }
    radius
}

/// Newton budget for [`nova_os_unbow_radius`]. The solve is quadratic and starts
/// within a few percent, so it lands in a handful of steps; the cap is a
/// non-convergence backstop, not the expected cost. The tolerance is in uv, i.e.
/// well under a millionth of the picture's width.
const NOVA_OS_UNBOW_STEPS: usize = 16;
const NOVA_OS_UNBOW_TOLERANCE: f32 = 1e-7;

/// Where in the WINDOW the point at screen-local uv `local` of the sampling
/// surface sits, and the bounds-checked inverse that answers which `local` a
/// window cursor is over.
///
/// One definition of the surface rect for both directions. A [`ComputedNode`]
/// carries PHYSICAL pixels and [`Window::cursor_position`] reports LOGICAL ones,
/// so the rect is scaled back through [`ComputedNode::inverse_scale_factor`]
/// before either is compared with the other; skipping that reads right only at
/// scale factor 1 - and puts every click a factor of two out on a HiDPI display.
pub(crate) fn nova_os_glass_window_px(
    local: Vec2,
    node: &ComputedNode,
    xf: &UiGlobalTransform,
) -> Vec2 {
    let (min, size) = nova_os_glass_rect(node, xf);
    min + local * size
}

/// [`nova_os_glass_window_px`]'s inverse: `None` when the cursor is off the
/// glass entirely, which parks the forwarded pointer instead of clamping it onto
/// an edge nobody is pointing at.
pub(crate) fn nova_os_glass_local_uv(
    cursor: Vec2,
    node: &ComputedNode,
    xf: &UiGlobalTransform,
) -> Option<Vec2> {
    let (min, size) = nova_os_glass_rect(node, xf);
    let local = (cursor - min) / size;
    (!local.cmplt(Vec2::ZERO).any() && !local.cmpgt(Vec2::ONE).any()).then_some(local)
}

/// The sampling surface's top-left and size in LOGICAL window pixels. The size
/// is floored at one pixel so an unlaid-out surface divides to a finite uv
/// rather than a NaN that reads as "off the glass" only by luck of the compare.
fn nova_os_glass_rect(node: &ComputedNode, xf: &UiGlobalTransform) -> (Vec2, Vec2) {
    let scale = node.inverse_scale_factor();
    let size = (node.size() * scale).max(Vec2::ONE);
    (xf.translation * scale - size * 0.5, size)
}

/// Where to put the real cursor so the forwarded pointer lands on `image_px` of
/// the NOVA OS's offscreen image - [`forward_nova_os_pointer`] run backwards,
/// against the live surface rect, image size and CRT power.
///
/// A UI node behind the image camera reports its rect in IMAGE pixels, which is
/// a space no window cursor can be placed in: something has to undo the warp
/// before a caller can point at it. Without this, a driven run can only click
/// what happens to sit in window space, and the whole terminal - every widget
/// past the glass - is unreachable except by triggering its observer directly,
/// which is precisely the shortcut that lets the pointer chain rot untested
/// (task 20260804-134347).
///
/// `None` when there is no live monitor to aim at, or when the CRT displays that
/// image point nowhere: off the picture, or swallowed by the raster collapse
/// mid-power-on. A caller must treat that as "not clickable yet", never as a
/// coordinate to clamp.
pub fn nova_os_window_px_showing(world: &World, image_px: Vec2) -> Option<Vec2> {
    let image = world.get_resource::<NovaOsRtt>()?.image.clone();
    let image_size = world
        .get_resource::<Assets<Image>>()
        .and_then(|images| images.get(&image))
        .map(|image| image.size().as_vec2())?;
    // The forwarder's own fallback: a rig with no shell entity is a full raster.
    let power = nova_os_openness(world).unwrap_or(1.0);
    let (node, xf) = {
        let mut query = world
            .try_query_filtered::<(&ComputedNode, &UiGlobalTransform), With<NovaOsSamplingSurfaceMarker>>(
            )?;
        let (node, xf) = query.single(world).ok()?;
        (*node, *xf)
    };

    let local = nova_os_crt_image_uv_to_screen(
        image_px / image_size,
        NOVA_OS_CRT_WARP,
        NOVA_OS_CRT_OVERSCAN,
        power,
    )?;
    Some(nova_os_glass_window_px(local, &node, &xf))
}

/// How far the monitor's raster has opened: 0 fully closed, 1 flush and drawing
/// the whole picture. `None` when no NOVA OS shell exists (it is spawned with
/// the player ship and despawned with it).
///
/// The advance condition a driven run needs between "the computer opened" and
/// "a click on the glass means anything": the raster collapse is a live uniform,
/// so until this reads 1 the CRT shows a squeezed window onto the image and
/// [`nova_os_window_px_showing`] answers for a picture that is still moving. A
/// dwell would only be a guess at the same moment.
///
/// Takes `&World`, so it can back a read-only predicate. That costs an entity
/// walk - the openness lives on one entity and this cannot pre-build a query -
/// which is a scripted run's price to pay, not a per-frame system's.
pub fn nova_os_openness(world: &World) -> Option<f32> {
    world
        .iter_entities()
        .find_map(|entity| entity.get::<NovaOsOpenness>().map(|openness| openness.0))
}

/// Power levels at which the raster collapse finishes opening vertically and
/// horizontally, and the floor that keeps the divide finite - the shader's
/// `smoothstep(0.0, 0.65, power)` / `smoothstep(0.0, 0.28, power)` / `max(_,
/// 0.0008)`. Constants, not literals, because the mapping mirrors them.
pub(crate) const NOVA_OS_CRT_POWER_OPEN_H: f32 = 0.65;
pub(crate) const NOVA_OS_CRT_POWER_OPEN_W: f32 = 0.28;
pub(crate) const NOVA_OS_CRT_POWER_EPSILON: f32 = 0.0008;

/// `bevy_picking::update_is_hovered` only mirrors the MOUSE pointer into `Hovered`
/// components, so replicate its ancestor walk for our forwarded pointer - else the
/// terminal's `Hovered`-gated wheel scroll would go dead through the image.
///
/// CRUCIALLY, this only manages `Hovered` on entities rendered THROUGH the image
/// (descendants of the content root). Window-space UI - the chin knobs
/// (task 214617), menus, any `Button` - keep the `Hovered` the MOUSE pointer's
/// `update_is_hovered` owns; touching them here would force `Hovered(false)` every
/// frame the NOVA OS is open (the forwarded pointer's HoverMap targets the image,
/// never the window), fighting the real cursor.
pub(crate) fn mirror_nova_os_hover(
    rtt: Option<Res<NovaOsRtt>>,
    hover_map: Option<Res<HoverMap>>,
    parents: Query<&ChildOf>,
    mut hovers: Query<(Entity, &Hovered)>,
    mut commands: Commands,
) {
    let Some(rtt) = rtt else {
        return;
    };
    let Some(hover_map) = hover_map else {
        return;
    };
    if hovers.is_empty() {
        return;
    }
    let mut hovered_set = bevy::platform::collections::HashSet::new();
    if let Some(hits) = hover_map.get(&nova_os_pointer_id()) {
        for entity in hits.keys() {
            hovered_set.insert(*entity);
            hovered_set.extend(parents.iter_ancestors(*entity));
        }
    }
    for (entity, hovered) in hovers.iter_mut() {
        // Only entities under the offscreen content root are served by the
        // forwarded pointer; never touch window-space `Hovered`.
        let through_image = entity == rtt.content_root
            || parents
                .iter_ancestors(entity)
                .any(|a| a == rtt.content_root);
        if !through_image {
            continue;
        }
        let is_hovering = hovered_set.contains(&entity);
        if hovered.get() != is_hovering {
            commands.entity(entity).insert(Hovered(is_hovering));
        }
    }
}
/// Feed real-time seconds and the panel pixel size into the CRT material each
/// frame: `time` drives the grain shimmer and `resolution` (from the overlay
/// node's [`ComputedNode`]) makes the scanlines/slot-mask resolution-aware. Real
/// time because the sim clock is frozen while the computer is open.
pub(crate) fn animate_nova_os_crt(
    time: Res<Time<Real>>,
    settings: Res<NovaOsMonitorSettings>,
    mut degauss: ResMut<NovaOsDegauss>,
    mut materials: ResMut<Assets<NovaOsCrtMaterial>>,
    q_openness: Query<&NovaOsOpenness, With<NovaOsRootMarker>>,
    q_surface: Query<
        (&MaterialNode<NovaOsCrtMaterial>, &ComputedNode),
        With<NovaOsSamplingSurfaceMarker>,
    >,
) {
    let seconds = time.elapsed_secs();
    // Bleed the degauss pulse down by real time (the sim clock is frozen while the
    // computer is open) and feed its 0..1 envelope to the shader.
    if degauss.remaining > 0.0 {
        degauss.remaining = (degauss.remaining - time.delta_secs()).max(0.0);
    }
    let degauss_env = degauss.envelope();
    // Feed the eased openness in as the CRT power level: the shader blooms the
    // raster on from a line and collapses it to a dying dot on close.
    let power = q_openness.iter().next().map(|o| o.0).unwrap_or(1.0);
    // The BRIGHT/SCAN chin knobs drive the brightness multiply and scanline
    // depth uniforms.
    let brightness = settings.brightness();
    let scanline_strength = settings.scanline_strength();
    for (node, computed) in &q_surface {
        if let Some(mut material) = materials.get_mut(&node.0) {
            material.data.time = seconds;
            material.data.resolution = computed.size;
            material.data.power = power;
            material.data.brightness = brightness;
            material.data.scanline_strength = scanline_strength;
            material.data.degauss = degauss_env;
        }
    }
}
