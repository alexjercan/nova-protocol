// NOVA OS CRT overlay: scanlines, phosphor tint and edge darkening for the
// drawer's Bevy UI monitor. Kept derivative-free so it stays safe on WebGL2.

#import bevy_ui::ui_vertex_output::UiVertexOutput

struct NovaOsCrtMaterial {
    // Straight-alpha phosphor tint.
    tint: vec4<f32>,
    // Darkening applied by horizontal scanlines.
    scanline_strength: f32,
    // Edge darkening at the screen corners.
    vignette_strength: f32,
    // Subtle centre glow.
    glow_strength: f32,
    // Sparse square phosphor grain.
    grain_strength: f32,
}

@group(1) @binding(0)
var<uniform> material: NovaOsCrtMaterial;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let centered = uv - vec2<f32>(0.5, 0.5);
    let dist = length(centered);

    let scan = select(1.0, 1.0 - material.scanline_strength, fract(uv.y * 240.0) < 0.5);
    let corner_falloff = smoothstep(0.18, 0.76, dist);
    let vignette = pow(corner_falloff, 1.26) * material.vignette_strength;
    let glow = (1.0 - smoothstep(0.0, 0.68, dist)) * material.glow_strength;
    let grain_cell = floor(uv * vec2<f32>(120.0, 68.0));
    let grain_hash = fract(sin(dot(grain_cell, vec2<f32>(12.9898, 78.233))) * 43758.5453);
    let grain = (grain_hash - 0.5) * material.grain_strength;
    let spark = step(0.985, grain_hash) * material.grain_strength * 0.95;

    let tint_alpha = material.tint.a * scan + glow + abs(grain) * 0.32 + spark * 0.55;
    let edge_alpha = clamp(vignette, 0.0, 0.96);
    let rgb = material.tint.rgb * max(tint_alpha + grain + spark, 0.0);
    return vec4<f32>(rgb * (1.0 - edge_alpha), clamp(tint_alpha + edge_alpha, 0.0, 0.96));
}
