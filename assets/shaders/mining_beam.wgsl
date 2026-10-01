// The mining beam: an unlit, additive tube from the emitter face to the hit.
//
// The mesh is an uncapped cylinder along +Y, stretched to the beam's length,
// so uv.y runs 0 at the emitter face to 1 at the hit. The tube is drawn as a
// bright core inside a soft halo: how squarely the view meets the surface,
// against how squarely it could at this view of the axis, stands in for the
// distance from the beam's axis on screen. Bands flow from the face to the
// rock, and each pulse sends one bright slug down the beam while the whole
// beam flares and fades back.

#import bevy_pbr::{
    forward_io::VertexOutput,
    mesh_view_bindings::{globals, view},
}

#ifdef TONEMAP_IN_SHADER
#import bevy_core_pipeline::tonemapping::tone_mapping
#endif

struct MiningBeamMaterial {
    // The halo's colour, HDR.
    glow: vec4<f32>,
    // The core's colour, HDR.
    core: vec4<f32>,
    // The beam's direction, face to hit, in world space.
    axis: vec3<f32>,
    // The beam's length in engine units, so the bands keep their spacing on
    // any length.
    length: f32,
    // 1 on the frame a pulse lands, falling to 0.
    flash: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> material: MiningBeamMaterial;

// Bands per engine unit, and how fast they run toward the rock.
const BAND_DENSITY: f32 = 1.6;
const BAND_SPEED: f32 = 5.0;
// The slug's half width, in engine units.
const SLUG_WIDTH: f32 = 0.5;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let to_eye = normalize(view.world_position.xyz - in.world_position.xyz);
    // The tube's normal is square to its axis, so it can meet the eye at
    // most as squarely as the eye meets the axis side on. Looking down the
    // beam, both are small; dividing keeps the core on the axis there too.
    let side_on = sqrt(max(1.0 - pow(dot(material.axis, to_eye), 2.0), 1e-4));
    let facing = min(abs(dot(normalize(in.world_normal), to_eye)) / side_on, 1.0);
    // 0 on the axis, 1 at the silhouette.
    let off_axis = sqrt(max(1.0 - facing * facing, 0.0));

    let along = in.uv.y * material.length;
    let t = globals.time;

    // A bright line down the middle, widened by a pulse.
    let core_width = 0.28 + 0.22 * material.flash;
    let core = 1.0 - smoothstep(0.0, core_width, off_axis);
    // The halo falls off to nothing at the silhouette.
    let halo = pow(1.0 - off_axis, 2.2);

    // Flowing bands, soft-edged, with a fast shimmer that never goes dark.
    let band = 0.5 + 0.5 * sin((along * BAND_DENSITY - t * BAND_SPEED) * 6.2831853);
    let bands = 0.7 + 0.45 * band * band;
    let shimmer = 0.92 + 0.08 * sin(t * 53.0 + along * 3.1);

    // The slug: at the face as the pulse lands, at the rock as the flash ends.
    let slug_at = (1.0 - material.flash) * material.length;
    let slug = material.flash * exp(-pow((along - slug_at) / SLUG_WIDTH, 2.0));

    // Fade in off the emitter lens and hold full strength into the rock.
    let ends = smoothstep(0.0, 0.04, along);

    let flare = 1.0 + 1.4 * material.flash;
    var color = material.glow.rgb * halo * bands * flare
        + material.core.rgb * core * (shimmer + 2.5 * slug) * flare;
    color *= ends;

#ifdef TONEMAP_IN_SHADER
    color = tone_mapping(vec4(color, 1.0), view.color_grading).rgb;
#endif
    // Bevy draws `AlphaMode::Add` with the premultiplied-alpha blend, so an
    // alpha of 0 keeps everything behind the beam and adds the beam on top.
    return vec4(color, 0.0);
}
