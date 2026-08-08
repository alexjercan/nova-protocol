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
    /// NOTE: lives in the uniform rather than as a WGSL constant
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
// ---------------------------------------------------------------------------
// Render-to-texture CRT pipeline.
//
// The terminal-content subtree renders to an offscreen image via a dedicated UI
// camera; the screen node then displays that image through the sampling
// `NovaOsCrtMaterial` (bloom + barrel warp + the whole CRT treatment). Interaction
// is preserved by FORWARDING a custom pointer whose location targets the image
// (bevy 0.19 `ui_picking` matches pointers to cameras by render target), plus a
// hover-mirror system because `bevy_picking::update_is_hovered` only tracks the
// mouse pointer.
// ---------------------------------------------------------------------------

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
pub(crate) fn nova_os_pointer_id() -> PointerId {
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
pub(crate) fn forward_nova_os_pointer(
    rtt: Option<Res<NovaOsRtt>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut mouse_buttons: MessageReader<bevy::input::mouse::MouseButtonInput>,
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
            let size = node.size();
            let min = xf.translation - size * 0.5;
            let local = (cursor - min) / size.max(Vec2::splat(1.0));
            if local.x < 0.0 || local.x > 1.0 || local.y < 0.0 || local.y > 1.0 {
                None
            } else {
                nova_os_crt_screen_to_image_uv(local, NOVA_OS_CRT_WARP, NOVA_OS_CRT_OVERSCAN, power)
                    .map(|uv| uv * image_size)
            }
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
    for ev in mouse_buttons.read() {
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
