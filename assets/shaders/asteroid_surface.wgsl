// Triplanar rock surfacing: texture a body by WHERE IT IS rather than by UVs,
// then shade it by WHAT IT IS.
//
// The mesh UVs an asteroid used to be textured through are planar PER TRIANGLE
// - each face projects the texture into its own plane from its own first vertex
// - so the texture restarted at every triangle edge and its scale followed the
// triangle's size. Two things fell out of that, and both are why this exists:
// a rock was visibly quilted, and a REMESHED rock (a carved one, whose
// triangles are grid-sized rather than subdivision-sized) wore a different
// texture scale from an uncarved one standing next to it.
//
// Sampling by position fixes both at once. There are no UVs, so there is
// nothing to seam and nothing that changes when the triangles do: a carved rock
// and a pristine one are the same material because the material never asked
// about the mesh.
//
// LOCAL space, not world. A rock tumbles and drifts, and a world-space
// projection would let the texture swim across the surface as it moved. The
// object's own axes are the normalized columns of its model matrix, and
// projecting onto them is the inverse rotation - exact here because nothing in
// the asteroid hierarchy is sheared or scaled non-uniformly.
//
// WHAT IS ADDED ON TOP, and why it is not a texture. A single tile wrapped by
// `fract` repeats about every three units, and a rock's surface stands three and
// a half to six units out, so the tile lands several times on one body - and
// identically on every body in the field. A bigger or better tile does not fix
// that; it moves the repeat further out and keeps it. What has no period at all
// is a noise field read straight off the same local position, so that is what
// the palette, the seams and the roughness are drawn from. The texture stays,
// demoted to what it is genuinely good at: photographic GRAIN, read for its
// brightness rather than its colour.
//
// Four layers, in the order they are computed:
//   1. a per-body rotation and offset of the whole sampling frame, from the
//      rock's silhouette seed, so two rocks of a kind are not one rock;
//   2. a domain-warped fBm read in local space, which picks the palette colour
//      and drives the roughness;
//   3. a Worley cell layer, which paints seams and crackle, skipped whole when
//      its kind does not want it;
//   4. the triplanar texture at two incommensurate scales, blended by the fBm,
//      which breaks what is left of the tile's period.

#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
    mesh_functions::get_world_from_local,
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}
#endif

// Field for field, in order, `AsteroidSurfaceUniform` in
// crates/nova_scenario/src/objects/asteroid_surface.rs. The leading vec4s give
// the struct a 16 byte alignment, which is what keeps it valid as a uniform on
// every backend without hand-written padding.
struct AsteroidSurfaceMaterialData {
    // The dark end of the kind's palette, linear.
    shade: vec4<f32>,
    // The light end of the kind's palette, linear.
    tint: vec4<f32>,
    // The colour the Worley cell walls are painted, linear.
    vein: vec4<f32>,
    // Texture repeats per unit of the body's own local space.
    tiling: f32,
    // How hard the three projections cut over to each other. Higher is a
    // crisper transition on a face pointing between two axes.
    sharpness: f32,
    // Macro-noise cycles per unit of local space.
    macro_scale: f32,
    // How far the macro noise's own domain is warped before it is read.
    warp: f32,
    // How hard the macro noise is pushed away from its midpoint.
    contrast: f32,
    // How much of the kind palette replaces the texture's own colour.
    kind_mix: f32,
    // How strongly the texture's brightness modulates the palette.
    grain: f32,
    // The texture's mean linear luminance, which `grain` is measured against.
    grain_mid: f32,
    // How much of the second texture scale is blended in to break the repeat.
    break_up: f32,
    // Worley cell cycles per unit of local space.
    vein_scale: f32,
    // How strongly the cell walls are painted in `vein`.
    vein_strength: f32,
    // Perceptual roughness where the surface is smoothest.
    roughness_low: f32,
    // Perceptual roughness where the surface is roughest.
    roughness_high: f32,
    // Metallic response.
    metallic: f32,
    // This body's own 0..1 draw from its silhouette seed.
    jitter: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> material: AsteroidSurfaceMaterialData;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var rock_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var rock_sampler: sampler;

const TAU: f32 = 6.2831853;

// How many octaves the macro fBm runs. Four, matching the octave count the
// SILHOUETTE is generated from (`ROCK_OCTAVES`), because the surface and the
// shape are describing the same rock and should agree about how detailed it is.
const MACRO_OCTAVES: i32 = 4;

// The second texture scale, as a fraction of the first. Deliberately not a
// simple ratio: two scales whose periods share a common multiple would repeat
// together, which is the thing being removed. 0.41 puts the shared period far
// past anything one body can show.
const SECOND_SCALE: f32 = 0.41;

// How far apart the two scales are pushed in the tile, so the blend never lines
// the same texel up with itself.
const SECOND_OFFSET: vec2<f32> = vec2<f32>(13.37, 7.11);

// How wide the Worley cell wall is, in cell units, and how hard the paint falls
// off across it. Narrow and squared: a seam is a hairline. A wide, linear one
// draws a honeycomb, and a honeycomb is a repeating pattern - which would be
// this shader introducing the fault it exists to remove.
const VEIN_WIDTH: f32 = 0.09;

// Where the crossover between the two texture scales starts and ends, in the
// macro field's own 0..1. Narrow, for the reason the blend itself documents.
const BLEND_EDGE_LOW: f32 = 0.42;
const BLEND_EDGE_HIGH: f32 = 0.58;

// One 32-bit avalanche over a lattice cell. Integer-only, so it is exact on
// every backend and has none of the banding a sin-based hash shows on some
// drivers.
fn hash_cell(cell: vec3<i32>) -> u32 {
    var hash = bitcast<u32>(cell.x) * 0x9e3779b1u
        ^ bitcast<u32>(cell.y) * 0x85ebca6bu
        ^ bitcast<u32>(cell.z) * 0xc2b2ae35u;
    hash ^= hash >> 15u;
    hash = hash * 0x2545f491u;
    hash ^= hash >> 13u;
    return hash;
}

// One lattice cell's 0..1 value.
fn hash_unit(cell: vec3<i32>) -> f32 {
    return f32(hash_cell(cell)) * (1.0 / 4294967296.0);
}

// Three 0..1 values from ONE hash, taken from disjoint bit fields. A cell's
// jitter needs three numbers and does not need three avalanches for them.
fn hash_unit3(cell: vec3<i32>) -> vec3<f32> {
    let hash = hash_cell(cell);
    return vec3<f32>(
        f32(hash & 0x3ffu) * (1.0 / 1024.0),
        f32((hash >> 10u) & 0x3ffu) * (1.0 / 1024.0),
        f32((hash >> 20u) & 0x3ffu) * (1.0 / 1024.0),
    );
}

// Value noise on the integer lattice, quintic-smoothed so the second derivative
// is continuous and the field has no lattice-aligned creases.
fn value_noise(point: vec3<f32>) -> f32 {
    let base = floor(point);
    let frac = point - base;
    let weight = frac * frac * frac * (frac * (frac * 6.0 - 15.0) + 10.0);
    let cell = vec3<i32>(base);

    let c000 = hash_unit(cell + vec3<i32>(0, 0, 0));
    let c100 = hash_unit(cell + vec3<i32>(1, 0, 0));
    let c010 = hash_unit(cell + vec3<i32>(0, 1, 0));
    let c110 = hash_unit(cell + vec3<i32>(1, 1, 0));
    let c001 = hash_unit(cell + vec3<i32>(0, 0, 1));
    let c101 = hash_unit(cell + vec3<i32>(1, 0, 1));
    let c011 = hash_unit(cell + vec3<i32>(0, 1, 1));
    let c111 = hash_unit(cell + vec3<i32>(1, 1, 1));

    let near = mix(mix(c000, c100, weight.x), mix(c010, c110, weight.x), weight.y);
    let far = mix(mix(c001, c101, weight.x), mix(c011, c111, weight.x), weight.y);
    return mix(near, far, weight.z);
}

// Fractal Brownian motion: octaves at 2.03x frequency and half amplitude,
// normalized to 0..1. The lacunarity is not exactly 2 so the octaves' lattices
// never realign into a visible grid.
fn fbm(point: vec3<f32>) -> f32 {
    var total = 0.0;
    var norm = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    for (var octave = 0; octave < MACRO_OCTAVES; octave = octave + 1) {
        total = total + amplitude * value_noise(point * frequency);
        norm = norm + amplitude;
        frequency = frequency * 2.03;
        amplitude = amplitude * 0.5;
    }
    return total / max(norm, 1e-4);
}

// Worley F2 - F1: near zero ON a cell wall and rising away from it, which is
// what makes the wall a line that can be painted rather than a blob that can be
// tinted. Twenty-seven cells, one hash each.
fn worley_wall(point: vec3<f32>) -> f32 {
    let base = floor(point);
    let cell = vec3<i32>(base);
    var nearest = 8.0;
    var second = 8.0;

    for (var x = -1; x <= 1; x = x + 1) {
        for (var y = -1; y <= 1; y = y + 1) {
            for (var z = -1; z <= 1; z = z + 1) {
                let neighbour = cell + vec3<i32>(x, y, z);
                let seed = vec3<f32>(neighbour) + hash_unit3(neighbour);
                let distance = length(seed - point);
                if distance < nearest {
                    second = nearest;
                    nearest = distance;
                } else if distance < second {
                    second = distance;
                }
            }
        }
    }
    return second - nearest;
}

// One projection's worth of the texture.
//
// Wrapped by hand rather than by the sampler's address mode: the texture
// arrives through the ordinary asset path (and through a mod's, for a modded
// rock), so it cannot be assumed to have been loaded as repeating. The
// derivatives come from the UNWRAPPED coordinate - taking them after `fract`
// would make the wrap seam look like a jump across the whole texture and
// collapse that pixel to the lowest mip.
fn sample_tile(uv: vec2<f32>) -> vec4<f32> {
    let ddx = dpdx(uv);
    let ddy = dpdy(uv);
    return textureSampleGrad(rock_texture, rock_sampler, fract(uv), ddx, ddy);
}

// The three projections blended by how much the face points along each axis.
fn triplanar(local: vec3<f32>, weight: vec3<f32>) -> vec3<f32> {
    return sample_tile(local.zy).rgb * weight.x
        + sample_tile(local.xz).rgb * weight.y
        + sample_tile(local.xy).rgb * weight.z;
}

// Rotate about Y then about X. Two angles from one jitter draw, so a body's
// projection frame is its own without carrying a second number for it.
fn jitter_frame(point: vec3<f32>, angle: f32) -> vec3<f32> {
    let sin_y = sin(angle);
    let cos_y = cos(angle);
    let turned = vec3<f32>(
        point.x * cos_y + point.z * sin_y,
        point.y,
        point.z * cos_y - point.x * sin_y,
    );
    let tilt = angle * 0.618;
    let sin_x = sin(tilt);
    let cos_x = cos(tilt);
    return vec3<f32>(
        turned.x,
        turned.y * cos_x - turned.z * sin_x,
        turned.y * sin_x + turned.z * cos_x,
    );
}

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    let world_from_local = get_world_from_local(in.instance_index);
    // The body's own axes. Rotation and uniform scale only, so normalizing the
    // columns gives an orthonormal basis and dotting against it is the inverse
    // rotation - no matrix inverse in the fragment shader.
    let axis_x = normalize(world_from_local[0].xyz);
    let axis_y = normalize(world_from_local[1].xyz);
    let axis_z = normalize(world_from_local[2].xyz);
    let origin = world_from_local[3].xyz;

    let offset = in.world_position.xyz - origin;
    let body = vec3<f32>(
        dot(offset, axis_x),
        dot(offset, axis_y),
        dot(offset, axis_z),
    );

    let world_normal = normalize(in.world_normal.xyz);
    let body_normal = vec3<f32>(
        dot(world_normal, axis_x),
        dot(world_normal, axis_y),
        dot(world_normal, axis_z),
    );

    // Layer 1: this body's own frame. The rotation and the offset both ride the
    // silhouette seed, so a rock keeps the same surface on every load and no
    // two seeds sit in the same place in the tile or in the noise.
    let angle = material.jitter * TAU;
    let drift = vec3<f32>(
        material.jitter * 37.1,
        material.jitter * 17.7,
        material.jitter * 29.3,
    );
    let local = jitter_frame(body, angle) + drift;
    let normal = jitter_frame(body_normal, angle);

    var weight = pow(abs(normal), vec3<f32>(material.sharpness));
    weight = weight / max(weight.x + weight.y + weight.z, 1e-4);

    // Layer 2: the macro field. Warping the domain before reading it is what
    // turns fBm's soap bubbles into stretched, tangled strata - one extra read
    // for most of the character.
    let warp_at = local * 0.7;
    let warped = local + material.warp * vec3<f32>(
        value_noise(warp_at),
        value_noise(warp_at + vec3<f32>(19.3, 5.1, 31.7)),
        value_noise(warp_at + vec3<f32>(7.9, 23.5, 11.2)),
    );
    let raw = fbm(warped * material.macro_scale);
    let mottle = clamp((raw - 0.5) * material.contrast + 0.5, 0.0, 1.0);

    // Layer 4 (computed before 3, which needs nothing from it): the texture at
    // two incommensurate scales, blended by the macro field. Neither scale's
    // period survives the blend, so the grain stops arriving on a grid.
    let tiled = local * material.tiling;
    let near = triplanar(tiled, weight);
    let far = triplanar(tiled * SECOND_SCALE + vec3<f32>(SECOND_OFFSET, SECOND_OFFSET.x), weight);
    // A HARD choice with a narrow crossfade, not an average. Blending two
    // decorrelated samples at even weight halves their variance, and the
    // variance is the crevice detail the texture is kept for - a first pass
    // that mixed them linearly came back visibly softer than the control it was
    // meant to beat. Choosing one scale over most of the surface and crossing
    // over in a narrow band keeps the grain and still leaves neither scale's
    // period intact.
    let choose = smoothstep(BLEND_EDGE_LOW, BLEND_EDGE_HIGH, mottle) * material.break_up;
    let texture_rgb = mix(near, far, choose);

    // The texture's brightness as a RATIO against its own mean, so it lands as
    // relief on the palette instead of dragging the palette down to its level.
    let value = dot(texture_rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let relief = 1.0 + (value / max(material.grain_mid, 1e-4) - 1.0) * material.grain;

    let palette = mix(material.shade.rgb, material.tint.rgb, mottle) * max(relief, 0.0);
    var albedo = mix(texture_rgb, palette, material.kind_mix);

    // Layer 3: the cell walls. A uniform branch, so it is coherent across the
    // whole draw and a kind that wants no seams pays nothing for them.
    if material.vein_strength > 0.0 {
        // The WARPED coordinate, not the plain one. Worley on an unwarped
        // lattice draws cells of one size in rows, which is a grid however
        // jittered the seeds are; warping it first is what makes the network
        // read as fracture rather than as chicken wire.
        let wall = worley_wall(warped * material.vein_scale);
        let edge = 1.0 - smoothstep(0.0, VEIN_WIDTH, wall);
        albedo = mix(albedo, material.vein.rgb, edge * edge * material.vein_strength);
    }

    // MULTIPLIED into the authored base colour rather than replacing it, so a
    // tint on the standard material still tints and a damage grade still
    // grades.
    pbr_input.material.base_color = pbr_input.material.base_color * vec4<f32>(albedo, 1.0);

    // Colour and specular read the SAME field, so where a rock looks worn it
    // also scatters like it. Two thirds macro, one third grain: the big regions
    // decide the character and the crevices texture it.
    let grain_wear = clamp(value / max(material.grain_mid, 1e-4) * 0.5, 0.0, 1.0);
    let wear = clamp(mottle * 0.65 + grain_wear * 0.35, 0.0, 1.0);
    pbr_input.material.perceptual_roughness =
        mix(material.roughness_low, material.roughness_high, wear);
    pbr_input.material.metallic = material.metallic;

    // alpha discard
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);
#ifdef PREPASS_PIPELINE
    // in deferred mode we can't modify anything after that, as lighting is run in a separate fullscreen shader.
    let out = deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;
    // apply lighting
    out.color = apply_pbr_lighting(pbr_input);

    // apply in-shader post processing (fog, alpha-premultiply, and also tonemapping, debanding if the camera is non-hdr)
    // note this does not include fullscreen postprocessing effects like bloom.
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
#endif

    return out;
}
