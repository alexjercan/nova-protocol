// NOVA OS CRT: the SAMPLING shader (task 20260726-193233). Unlike the earlier
// overlay film, this material SAMPLES the offscreen image that holds the rendered
// terminal content, so it can do what an overlay never could: bloom the bright
// green glyphs into a soft halo and barrel-warp the CONTENT itself. It also
// carries the whole CRT treatment (soft resolution-aware scanlines + slot mask,
// edge vignette, analog grain, rounded-corner mask) plus the power-on/off raster
// collapse. Derivative-free and fixed-tap so it stays safe/cheap on WebGL2.

#import bevy_ui::ui_vertex_output::UiVertexOutput

struct NovaOsCrtMaterial {
    // Straight phosphor tint. `a` scales the film strength.
    tint: vec4<f32>,
    // Offscreen image pixel size, fed each frame (texel-sized bloom taps +
    // resolution-aware scanlines). Zero before layout.
    resolution: vec2<f32>,
    scanline_strength: f32,
    vignette_strength: f32,
    glow_strength: f32,
    grain_strength: f32,
    // Real-time seconds (grain shimmer).
    time: f32,
    // Rounded-corner radius in screen pixels (0 disables the mask).
    corner_radius: f32,
    // Barrel-distortion amount (0 = flat): warps the sample UV so the content bows.
    warp: f32,
    // Bloom strength (halo of the bright green glyphs).
    bloom: f32,
    // Power level 0..1 (from DrawerOpenness): 1 = full raster, 0 = collapsed to a
    // dying line/dot. Drives the CRT on/off collapse.
    power: f32,
    // Extra brightness multiply (>1 blooms hotter). Reserved for task 214617's
    // BRIGHT knob; 1.0 is neutral.
    brightness: f32,
    // Degauss envelope 0..1 (task 20260727-014148): pulsed to 1 on an app
    // launch/exit/switch and decayed back to 0 by `animate_nova_os_crt`. Drives a
    // brief horizontal wobble + white flash; every term it feeds is multiplied by
    // it, so at 0 the degauss is an exact no-op (readability preserved).
    degauss: f32,
}

@group(1) @binding(0) var<uniform> material: NovaOsCrtMaterial;
@group(1) @binding(1) var source_texture: texture_2d<f32>;
@group(1) @binding(2) var source_sampler: sampler;

const TWO_PI: f32 = 6.28318530718;
const SCANLINE_PITCH_PX: f32 = 3.0;
const SLOT_PITCH_PX: f32 = 3.0;
const SLOT_STRENGTH: f32 = 0.03;

// Degauss (task 20260727-014148): the coil settling on an app launch/exit is a
// decaying horizontal shear plus a brief white lift. Both scale with the
// `degauss` envelope (envelope^2) so they vanish to an exact no-op at rest.
const DEGAUSS_WOBBLE_PX: f32 = 6.0; // peak horizontal shear at full envelope
const DEGAUSS_FLASH: f32 = 0.16;    // peak white lift at full envelope

// Always-on analog micro-effects, each tiny enough that centre text stays crisp.
const HUM_BAR_SPEED: f32 = 0.11;      // tube-heights per second the hum bar drifts
const HUM_BAR_WIDTH: f32 = 0.11;      // gaussian half-width of the bar (uv)
const HUM_BAR_STRENGTH: f32 = 0.03;   // green lift under the drifting bar
const FLICKER_PERIOD: f32 = 4.5;      // seconds per mains-flicker cycle
const FLICKER_STRENGTH: f32 = 0.012;  // brightness wobble amplitude (multiply)
const RETRACE_PERIOD: f32 = 7.0;      // seconds between retrace beam sweeps
const RETRACE_SPEED: f32 = 3.0;       // tube-heights per second the beam falls
const RETRACE_WIDTH: f32 = 0.012;     // gaussian half-width of the beam (uv)
const RETRACE_STRENGTH: f32 = 0.09;   // green lift along the fast retrace beam

fn hash21(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

// Barrel warp: push UVs outward from centre by r^2 so the middle stays put and
// the edges bow.
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

    // Power-on/off raster collapse: as power falls to 0 the picture squeezes
    // vertically toward the centre scan line, then to a dot. A brightness
    // overshoot near the transition sells the phosphor kick.
    let open_h = smoothstep(0.0, 0.65, material.power);
    let open_w = smoothstep(0.0, 0.28, material.power);
    var collapsed = 0.0;
    // Remap the sample Y through the collapsing band; X pinches last (the dot).
    let cy = (in.uv.y - 0.5) / max(open_h, 0.0008) + 0.5;
    let cx = (in.uv.x - 0.5) / max(open_w, 0.0008) + 0.5;
    if cy < 0.0 || cy > 1.0 || cx < 0.0 || cx > 1.0 {
        collapsed = 1.0;
    }
    let sample_uv = vec2<f32>(cx, cy);

    // Degauss coil settle: a fast decaying horizontal shear that wobbles the
    // raster sideways while the coil rings out. Envelope^2-scaled, so it is an
    // exact no-op when idle and never touches steady-state readability.
    let wobble = sin(in.uv.y * 18.0 + material.time * 90.0)
        * material.degauss * material.degauss
        * (DEGAUSS_WOBBLE_PX * texel.x);
    let shaken_uv = vec2<f32>(sample_uv.x + wobble, sample_uv.y);

    // Warp the (power-remapped) content. Anything outside the panel is tube-black.
    let warped = barrel(shaken_uv, material.warp);
    let in_bounds = f32(warped.x >= 0.0 && warped.x <= 1.0 && warped.y >= 0.0 && warped.y <= 1.0);

    var base = textureSample(source_texture, source_sampler, warped);

    // Fixed-tap derivative-free bloom: a 12-tap Gaussian-ish gather of the source
    // around the sample. The phosphor content is bright green on near-black, so
    // adding it back as a halo blooms the glyphs.
    let offs = array<vec2<f32>, 12>(
        vec2<f32>(1.0, 0.0), vec2<f32>(-1.0, 0.0), vec2<f32>(0.0, 1.0), vec2<f32>(0.0, -1.0),
        vec2<f32>(2.0, 0.0), vec2<f32>(-2.0, 0.0), vec2<f32>(0.0, 2.0), vec2<f32>(0.0, -2.0),
        vec2<f32>(1.5, 1.5), vec2<f32>(-1.5, 1.5), vec2<f32>(1.5, -1.5), vec2<f32>(-1.5, -1.5),
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

    // Soft resolution-aware scanlines + a whisper of aperture-grille slot mask.
    let scan_line = 0.5 - 0.5 * cos(in.uv.y * res_y / SCANLINE_PITCH_PX * TWO_PI);
    let slot = 0.5 - 0.5 * cos(in.uv.x * res_x / SLOT_PITCH_PX * TWO_PI);
    let scan = 1.0 - material.scanline_strength * scan_line - SLOT_STRENGTH * slot;

    // Edge vignette (transparent through the readable centre, darkening to corners)
    // plus the soft centre volume glow.
    let centered = (in.uv - vec2<f32>(0.5, 0.5)) * vec2<f32>(1.0, 0.82);
    let dist = length(centered);
    let vignette = 1.0 - smoothstep(0.46, 0.98, dist) * material.vignette_strength;
    let bulge = 1.0 - smoothstep(0.0, 0.95, dist);
    let glow = bulge * material.glow_strength;

    // Analog grain: fine layer interpolated between ~9 Hz reseeds, stronger in the
    // darker regions.
    let anim = material.time * 9.0;
    let blend = fract(anim);
    let cell = floor(in.uv * vec2<f32>(900.0, 520.0));
    let fine = mix(hash21(cell + floor(anim)), hash21(cell + floor(anim) + 1.0), blend);
    let grain_weight = mix(0.55, 1.3, smoothstep(0.1, 0.9, dist));
    let grain = (fine - 0.5) * material.grain_strength * grain_weight;

    // Phosphor kick: a brightness overshoot while the raster is mid-collapse.
    let kick = 1.0 + 2.2 * material.power * (1.0 - material.power);

    // Always-on analog micro-effects (task 20260727-014148). The mains-hum bar is
    // a soft bright band drifting slowly down the tube; the retrace beam is a
    // faster, rarer thin line falling once per period; the mains flicker is a
    // gentle brightness breathing. Hum + retrace lift the green phosphor; the
    // degauss flash lifts white. All are read against `in.uv` (the fixed screen
    // face), not the warped content, so they sit on the glass.
    let hum_pos = fract(material.time * HUM_BAR_SPEED);
    var dhum = abs(in.uv.y - hum_pos);
    dhum = min(dhum, 1.0 - dhum);
    let hum = exp(-(dhum * dhum) / (HUM_BAR_WIDTH * HUM_BAR_WIDTH)) * HUM_BAR_STRENGTH;

    let period_t = fract(material.time / RETRACE_PERIOD) * RETRACE_PERIOD;
    let beam_y = period_t * RETRACE_SPEED;
    let dbeam = abs(in.uv.y - beam_y);
    let retrace = exp(-(dbeam * dbeam) / (RETRACE_WIDTH * RETRACE_WIDTH))
        * step(beam_y, 1.0) * RETRACE_STRENGTH;

    let flicker = 1.0 + sin(material.time / FLICKER_PERIOD * TWO_PI) * FLICKER_STRENGTH;
    let flash = material.degauss * material.degauss * DEGAUSS_FLASH;
    let analog_add = (hum + retrace) * material.tint.rgb + vec3<f32>(flash, flash, flash);

    var rgb = (base.rgb * scan * vignette + glow + grain) * material.brightness * kick * flicker;
    rgb = (rgb + analog_add) * in_bounds * (1.0 - collapsed);

    // Rounded-corner mask in device pixels (a MaterialNode ignores BorderRadius).
    var corner_mask = 1.0;
    if material.resolution.x > 1.0 && material.resolution.y > 1.0 && material.corner_radius > 0.0 {
        let half = material.resolution * 0.5;
        let r = min(material.corner_radius, min(half.x, half.y));
        let p = abs(in.uv * material.resolution - half);
        let q = p - (half - vec2<f32>(r, r));
        let sd = length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - r;
        corner_mask = 1.0 - smoothstep(-1.0, 1.0, sd);
    }

    return vec4<f32>(rgb, corner_mask);
}
