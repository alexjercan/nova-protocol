// NOVA OS CRT overlay: scanlines, phosphor tint and edge darkening for the
// drawer's Bevy UI monitor. Kept derivative-free so it stays safe on WebGL2.

#import bevy_ui::ui_vertex_output::UiVertexOutput

struct NovaOsCrtMaterial {
    // Straight-alpha phosphor tint.
    tint: vec4<f32>,
    // Darkening applied by horizontal scanlines.
    scanline_strength: f32,
    // Edge darkening toward the screen corners.
    vignette_strength: f32,
    // Soft centre-peaked phosphor bulge that gives the screen its "volume".
    glow_strength: f32,
    // Sparse square phosphor grain.
    grain_strength: f32,
}

@group(1) @binding(0)
var<uniform> material: NovaOsCrtMaterial;

// A phosphor CRT overlay tuned to match `nova_os_terminal_poc.html`: a soft
// green glow that peaks at the centre and fades out (the HTML
// `radial-gradient(ellipse at center, rgba(54,255,121,0.18) ...)` volume) plus a
// vignette that darkens the outer corners, together giving the flat panel a
// bulged CRT feel, over a lively square-phosphor grain. The glow is kept LOW so
// it reads as volume, not the pale wash the old 0.13 glow filmed over the text.
@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    // Slightly elliptical distance so the vignette hugs the corners, not a circle.
    let centered = (uv - vec2<f32>(0.5, 0.5)) * vec2<f32>(1.0, 0.82);
    let dist = length(centered);

    // Subtle horizontal scanlines.
    let scan = select(1.0, 1.0 - material.scanline_strength, fract(uv.y * 240.0) < 0.5);

    // Centre volume: a soft green bulge, brightest at the middle, gone by the
    // edges - this is the gradient that makes the glass look curved.
    let glow = (1.0 - smoothstep(0.0, 0.85, dist)) * material.glow_strength;

    // Edge-only vignette: fully transparent through the readable centre, then
    // darkening toward the corners.
    let vignette = smoothstep(0.46, 0.98, dist) * material.vignette_strength;

    // Sparse square phosphor grain, with the occasional brighter spark cell so
    // the screen texture reads as "alive" rather than a flat film.
    let grain_cell = floor(uv * vec2<f32>(420.0, 236.0));
    let grain_hash = fract(sin(dot(grain_cell, vec2<f32>(12.9898, 78.233))) * 43758.5453);
    let grain = (grain_hash - 0.5) * material.grain_strength;
    let spark = step(0.986, grain_hash) * material.grain_strength * 1.6;

    // Phosphor film: uniform tint modulated by scanlines, plus the centre glow
    // and the grain/spark texture.
    let tint_alpha = material.tint.a * scan + glow + abs(grain) * 0.6 + spark;
    let edge_alpha = clamp(vignette, 0.0, 0.9);
    let rgb = material.tint.rgb * max(tint_alpha + spark, 0.0);
    // Straight-alpha over the terminal content.
    return vec4<f32>(rgb, clamp(tint_alpha + edge_alpha, 0.0, 0.92));
}
