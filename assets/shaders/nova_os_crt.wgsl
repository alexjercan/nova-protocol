// NOVA OS CRT overlay: scanlines, phosphor tint and edge darkening for the
// drawer's Bevy UI monitor. Kept derivative-free so it stays safe on WebGL2.

#import bevy_ui::ui_vertex_output::UiVertexOutput

struct NovaOsCrtMaterial {
    // Straight-alpha phosphor tint.
    tint: vec4<f32>,
    // The CRT panel's pixel size, fed each frame, so scanlines/slot-mask track
    // the real screen size instead of a fixed line count. Zero before layout.
    resolution: vec2<f32>,
    // Darkening applied by horizontal scanlines.
    scanline_strength: f32,
    // Edge darkening toward the screen corners.
    vignette_strength: f32,
    // Soft centre-peaked phosphor bulge that gives the screen its "volume".
    glow_strength: f32,
    // Sparse square phosphor grain.
    grain_strength: f32,
    // Real-time seconds, fed each frame so the grain shimmers gently.
    time: f32,
    // Rounded-corner radius in screen pixels. A UI MaterialNode is not clipped
    // by its node's BorderRadius, so the overlay masks its own corners here to
    // the screen's rounding. Zero disables the mask.
    corner_radius: f32,
}

@group(1) @binding(0)
var<uniform> material: NovaOsCrtMaterial;

// A phosphor CRT overlay tuned to match `nova_os_terminal_poc.html`: a soft
// green glow that peaks at the centre and fades out (the HTML
// `radial-gradient(ellipse at center, rgba(54,255,121,0.18) ...)` volume) plus a
// vignette that darkens the outer corners, together giving the flat panel a
// bulged CRT feel, over a lively square-phosphor grain. The glow is kept LOW so
// it reads as volume, not the pale wash the old 0.13 glow filmed over the text.
fn hash21(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

const TWO_PI: f32 = 6.28318530718;
// Device-pixel spacing of the scanlines and the aperture-grille slots.
const SCANLINE_PITCH_PX: f32 = 3.0;
const SLOT_PITCH_PX: f32 = 3.0;
const SLOT_STRENGTH: f32 = 0.03;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    // Slightly elliptical distance so the vignette hugs the corners, not a circle.
    let centered = (uv - vec2<f32>(0.5, 0.5)) * vec2<f32>(1.0, 0.82);
    let dist = length(centered);

    // Panel pixel size (fall back to a sane default before layout feeds it).
    let res_y = select(720.0, material.resolution.y, material.resolution.y > 1.0);
    let res_x = select(1280.0, material.resolution.x, material.resolution.x > 1.0);

    // Soft, resolution-aware scanlines: a smooth cosine trough every few DEVICE
    // pixels, so the line count tracks the real panel size and the soft profile
    // never aliases/moires the way the old hard fixed-240 step did. A whisper of
    // vertical aperture-grille slot-mask adds phosphor-stripe texture.
    let scan_line = 0.5 - 0.5 * cos(uv.y * res_y / SCANLINE_PITCH_PX * TWO_PI);
    let slot = 0.5 - 0.5 * cos(uv.x * res_x / SLOT_PITCH_PX * TWO_PI);
    let scan = 1.0 - material.scanline_strength * scan_line - SLOT_STRENGTH * slot;

    // Centre volume: a bright green bulge, brightest at the middle, fading to the
    // edges. A soft inner core is added on top so the middle of the glass reads
    // clearly brighter (the HTML radial-gradient centre), not just a gentle lift.
    let bulge = (1.0 - smoothstep(0.0, 0.95, dist));
    let core = (1.0 - smoothstep(0.0, 0.34, dist)) * 0.5;
    let glow = (bulge + core) * material.glow_strength;

    // Edge-only vignette: fully transparent through the readable centre, then
    // darkening toward the corners.
    let vignette = smoothstep(0.46, 0.98, dist) * material.vignette_strength;

    // CRT phosphor grain: a fine per-cell green noise (two frequencies so it does
    // not read as a regular checker) plus an occasional brighter spark cell, so
    // the screen looks like lit phosphor dots rather than a flat film. The fine
    // layer is INTERPOLATED between reseeds (~9/s) so the shimmer is analog, not
    // a hard step; the coarse layer stays put as a stable structure underneath.
    let anim = material.time * 9.0;
    let step_lo = floor(anim);
    let step_hi = step_lo + 1.0;
    let blend = fract(anim);
    let cell = floor(uv * vec2<f32>(900.0, 520.0));
    let fine_lo = hash21(cell + vec2<f32>(step_lo, step_lo * 1.7));
    let fine_hi = hash21(cell + vec2<f32>(step_hi, step_hi * 1.7));
    let fine = mix(fine_lo, fine_hi, blend);
    let coarse = hash21(floor(uv * vec2<f32>(300.0, 174.0)));
    let noise = (fine * 0.7 + coarse * 0.3) - 0.5;
    // Grain sits more in the darker regions: quiet in the bright centre (where
    // the text is), stronger toward the vignetted edges. (The overlay cannot
    // sample the glyphs, so this weights by screen position, not text luminance.)
    let grain_weight = mix(0.55, 1.3, smoothstep(0.1, 0.9, dist));
    let grain = noise * material.grain_strength * grain_weight;
    let spark = step(0.992, fine) * material.grain_strength * 2.2 * grain_weight;
    // Per-cell green shade: darker cells lean toward deep phosphor, lit cells
    // toward bright green, so the noise carries colour, not just brightness.
    let shade = mix(vec3<f32>(0.10, 0.62, 0.26), material.tint.rgb, clamp(fine + 0.25, 0.0, 1.0));

    // Phosphor film: uniform tint modulated by scanlines, plus the centre glow
    // and the grain/spark texture.
    let tint_alpha = material.tint.a * scan + glow + abs(grain) * 0.9 + spark;
    let edge_alpha = clamp(vignette, 0.0, 0.9);
    let rgb = shade * max(tint_alpha + spark, 0.0);

    // Rounded-corner mask: clip the overlay to the screen's rounded rectangle so
    // the phosphor film does not bleed past the casing/glass depth-pass rounding
    // (a UI MaterialNode ignores its node's BorderRadius). Rounded-rect SDF in
    // device pixels; disabled when resolution/radius are unset. This carries
    // into 193233's sampling shader.
    var corner_mask = 1.0;
    if material.resolution.x > 1.0 && material.resolution.y > 1.0 && material.corner_radius > 0.0 {
        let half = material.resolution * 0.5;
        let r = min(material.corner_radius, min(half.x, half.y));
        let p = abs(uv * material.resolution - half);
        let q = p - (half - vec2<f32>(r, r));
        let sd = length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - r;
        corner_mask = 1.0 - smoothstep(-1.0, 1.0, sd);
    }

    // Straight-alpha over the terminal content.
    return vec4<f32>(rgb, clamp(tint_alpha + edge_alpha, 0.0, 0.92) * corner_mask);
}
