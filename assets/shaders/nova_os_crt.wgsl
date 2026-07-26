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
    // Sparse square phosphor grain.
    grain_strength: f32,
}

@group(1) @binding(0)
var<uniform> material: NovaOsCrtMaterial;

// A phosphor CRT overlay tuned to match `nova_os_terminal_poc.html`: the centre
// stays almost fully transparent so the terminal text underneath reads crisp,
// while a soft vignette darkens only the outer edges (like the HTML
// `radial-gradient(ellipse at center, transparent 56%, rgba(0,0,0,0.42) 100%)`).
// There is deliberately NO centre glow: the previous version added a
// centre-peaked green haze over exactly where the text lives, washing it out.
@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    // Slightly elliptical distance so the vignette hugs the corners, not a circle.
    let centered = (uv - vec2<f32>(0.5, 0.5)) * vec2<f32>(1.0, 0.82);
    let dist = length(centered);

    // Subtle horizontal scanlines.
    let scan = select(1.0, 1.0 - material.scanline_strength, fract(uv.y * 240.0) < 0.5);

    // Edge-only vignette: fully transparent through the readable centre, then
    // darkening toward the corners.
    let vignette = smoothstep(0.46, 0.98, dist) * material.vignette_strength;

    // Sparse, faint phosphor grain texture.
    let grain_cell = floor(uv * vec2<f32>(480.0, 270.0));
    let grain_hash = fract(sin(dot(grain_cell, vec2<f32>(12.9898, 78.233))) * 43758.5453);
    let grain = (grain_hash - 0.5) * material.grain_strength;

    // Faint uniform phosphor film, modulated by the scanlines.
    let tint_alpha = material.tint.a * scan + abs(grain) * 0.3;
    let edge_alpha = clamp(vignette, 0.0, 0.9);
    let rgb = material.tint.rgb * tint_alpha;
    // Straight-alpha over the terminal content: near-transparent green in the
    // centre, near-black in the corners.
    return vec4<f32>(rgb, clamp(tint_alpha + edge_alpha, 0.0, 0.92));
}
