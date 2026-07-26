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
}

@group(1) @binding(0)
var<uniform> material: NovaOsCrtMaterial;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let centered = uv - vec2<f32>(0.5, 0.5);
    let dist = length(centered);

    let scan = select(1.0, 1.0 - material.scanline_strength, fract(uv.y * 240.0) < 0.5);
    let vignette = smoothstep(0.42, 0.76, dist) * material.vignette_strength;
    let glow = (1.0 - smoothstep(0.0, 0.72, dist)) * material.glow_strength;

    let tint_alpha = material.tint.a * scan + glow;
    let edge_alpha = vignette;
    let rgb = material.tint.rgb * max(tint_alpha, 0.0);
    return vec4<f32>(rgb * (1.0 - edge_alpha), clamp(tint_alpha + edge_alpha, 0.0, 0.78));
}
