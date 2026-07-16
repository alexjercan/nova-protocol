// Radial "lock-on dwell" ring (task 20260717-004302): a thin annulus that
// fills clockwise from the top as `progress` goes 0 -> 1, drawn on a UI node
// via bevy's UiMaterial pipeline. Deliberately trivial maths (no textures, no
// derivatives) so it is safe on the WebGL2 wasm target.

#import bevy_ui::ui_vertex_output::UiVertexOutput

const PI: f32 = 3.1415926535;
const TAU: f32 = 6.2831853072;

struct LockDwellRingMaterial {
    // Straight-alpha tint of the ring.
    color: vec4<f32>,
    // Fill fraction along the arc, [0, 1].
    progress: f32,
    // Inner radius of the annulus in normalized units (edge = 1.0).
    inner: f32,
    // Edge softness (anti-alias width) in the same normalized units.
    softness: f32,
}

@group(1) @binding(0)
var<uniform> material: LockDwellRingMaterial;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    // Centre the UV so the node's middle is the origin; d = 0 at centre, 1 at
    // the mid-edges (corners reach ~1.41 and fall outside the outer radius).
    let p = in.uv - vec2<f32>(0.5, 0.5);
    let d = length(p) * 2.0;

    // Annulus band: inside `inner`..1.0 with soft edges.
    let sw = max(material.softness, 0.0001);
    let band = smoothstep(material.inner - sw, material.inner + sw, d)
        * (1.0 - smoothstep(1.0 - sw, 1.0 + sw, d));

    // Angle measured clockwise from the top (screen-space UV has +y downward):
    // top = 0, right = 0.25, bottom = 0.5, left = 0.75.
    let ang = atan2(p.x, -p.y);
    let frac = fract(ang / TAU);

    // Fill up to `progress`, with a soft leading edge.
    let arc = 1.0 - smoothstep(material.progress, material.progress + sw, frac);

    let a = band * arc * material.color.a;
    return vec4<f32>(material.color.rgb, a);
}
