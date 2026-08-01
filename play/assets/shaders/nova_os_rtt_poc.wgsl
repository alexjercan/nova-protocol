// PROTOTYPE sampling CRT shader for task 20260726-193233.
//
// Unlike the overlay `nova_os_crt.wgsl` (a straight-alpha film that CANNOT read
// the text behind it), this material SAMPLES an offscreen image that holds the
// rendered terminal content, so it can do the two things the overlay never
// could: bloom the bright green glyphs, and barrel-warp the content itself.
// Kept derivative-free (no `dpdx`/`fwidth`) and fixed-tap so it stays WebGL2-safe
// - the prototype's job is to prove that bloom + warp are affordable there.

#import bevy_ui::ui_vertex_output::UiVertexOutput

struct PocCrtUniform {
    // Offscreen image pixel size, fed each frame (for texel-sized bloom taps and
    // resolution-aware scanlines). Zero before the target exists.
    resolution: vec2<f32>,
    // Real-time seconds, for the grain shimmer.
    time: f32,
    // Barrel-distortion amount (0 = flat). Warps the sample UV so the CONTENT
    // bows, the crisp curvature CSS could only fake.
    warp: f32,
    // Bloom strength (halo of the bright green glyphs).
    bloom: f32,
    // Scanline darkening.
    scanline: f32,
}

@group(1) @binding(0) var<uniform> material: PocCrtUniform;
@group(1) @binding(1) var source_texture: texture_2d<f32>;
@group(1) @binding(2) var source_sampler: sampler;

const TWO_PI: f32 = 6.28318530718;
const SCANLINE_PITCH_PX: f32 = 3.0;

fn hash21(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

// Barrel warp: push UVs outward from centre by r^2 so the middle stays put and
// the edges bow. Returns the warped sample coordinate.
fn barrel(uv: vec2<f32>, amount: f32) -> vec2<f32> {
    let centered = uv - vec2<f32>(0.5, 0.5);
    let r2 = dot(centered, centered);
    return vec2<f32>(0.5, 0.5) + centered * (1.0 + amount * r2);
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let res_x = select(1280.0, material.resolution.x, material.resolution.x > 1.0);
    let res_y = select(720.0, material.resolution.y, material.resolution.y > 1.0);
    let texel = vec2<f32>(1.0 / res_x, 1.0 / res_y);

    // Warp the content. Anything that lands outside the panel reads as the black
    // beyond the tube edge.
    let warped = barrel(in.uv, material.warp);
    let in_bounds = f32(warped.x >= 0.0 && warped.x <= 1.0 && warped.y >= 0.0 && warped.y <= 1.0);

    var base = textureSample(source_texture, source_sampler, warped);

    // Fixed-tap derivative-free bloom: a small separable-ish gather of the source
    // luminance around the sample, weighted by a Gaussian. Because the phosphor
    // content is bright green on near-black, sampling the raw colour and adding
    // it back as a halo blooms the glyphs. 13 taps (centre + 3 rings x 4) stays
    // cheap on WebGL2.
    let offs = array<vec2<f32>, 12>(
        vec2<f32>( 1.0,  0.0), vec2<f32>(-1.0,  0.0), vec2<f32>( 0.0,  1.0), vec2<f32>( 0.0, -1.0),
        vec2<f32>( 2.0,  0.0), vec2<f32>(-2.0,  0.0), vec2<f32>( 0.0,  2.0), vec2<f32>( 0.0, -2.0),
        vec2<f32>( 1.5,  1.5), vec2<f32>(-1.5,  1.5), vec2<f32>( 1.5, -1.5), vec2<f32>(-1.5, -1.5),
    );
    let wts = array<f32, 12>(
        0.12, 0.12, 0.12, 0.12,
        0.06, 0.06, 0.06, 0.06,
        0.05, 0.05, 0.05, 0.05,
    );
    var halo = vec3<f32>(0.0, 0.0, 0.0);
    for (var i = 0; i < 12; i = i + 1) {
        let s = textureSample(source_texture, source_sampler, warped + offs[i] * texel * 2.0);
        halo = halo + s.rgb * wts[i];
    }
    base = vec4<f32>(base.rgb + halo * material.bloom, base.a);

    // Soft resolution-aware scanlines.
    let scan_line = 0.5 - 0.5 * cos(in.uv.y * res_y / SCANLINE_PITCH_PX * TWO_PI);
    let scan = 1.0 - material.scanline * scan_line;

    // Edge vignette.
    let centered = (in.uv - vec2<f32>(0.5, 0.5)) * vec2<f32>(1.0, 0.82);
    let dist = length(centered);
    let vignette = 1.0 - smoothstep(0.46, 0.98, dist) * 0.6;

    // Gentle analog grain.
    let anim = material.time * 9.0;
    let blend = fract(anim);
    let cell = floor(in.uv * vec2<f32>(900.0, 520.0));
    let fine = mix(hash21(cell + floor(anim)), hash21(cell + floor(anim) + 1.0), blend);
    let grain = (fine - 0.5) * 0.05;

    let rgb = (base.rgb * scan * vignette) + grain;
    return vec4<f32>(rgb * in_bounds, 1.0);
}
