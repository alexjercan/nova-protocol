// Banded planet surfacing: colour a body by WHERE ON IT a fragment is, with no
// texture anywhere in the path.
//
// A planetoid today is an asteroid with a big radius, so it wears the rock
// material: one photo, projected triplanar-ly at a tiling tuned for a rock a
// few meters across. Across a body hundreds of meters wide that tile repeats
// hundreds of times, and hundreds of repeats of the same grey pixels is what
// the eye reads as a flat grey crust. The scale is what breaks, not the image -
// so no image can fix it.
//
// This shader reads two numbers off the fragment instead: how HIGH it is
// (recovered from its distance to the body's centre, against the range the
// mesh was displaced through) and what LATITUDE it is at. A palette of bands
// turns those into a colour, a roughness and a glow. The bands are hard
// thresholds and the last matching band wins - there is no blending in this
// round, on purpose.
//
// The variation inside a band, and the ragged edge between two of them, come
// from a value-noise field defined on the body's own DIRECTION. A field on the
// direction has no tile, so there is nothing to repeat however big the body
// gets. That is the whole answer to the repeat.
//
// LOCAL space, not world, for the same reason the rock material projects
// locally: a body that rotates would otherwise have its continents swim across
// its surface. The object's axes are the normalized columns of its model
// matrix, and projecting onto them is the inverse rotation - exact here because
// nothing in a planet's hierarchy is sheared or scaled non-uniformly.

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

// One elevation band. Mirrors `PlanetBandUniform`.
struct PlanetBand {
    // Linear rgb, with the emissive multiplier in w.
    color: vec4<f32>,
    // Roughness, height floor, latitude floor, one spare.
    surface: vec4<f32>,
}

// Mirrors `PlanetSurfaceUniform`. Every member is a vec4 or an array of them,
// so the layout is 16-byte aligned throughout and needs no padding members -
// there is no scalar in it for an alignment rule to strand.
struct PlanetSurfaceData {
    bands: array<PlanetBand, 6>,
    // Deepest radius, the range to the highest, live band count, one spare.
    shape: vec4<f32>,
    // Warp amount, warp frequency, grain amount, grain frequency.
    detail: vec4<f32>,
    // Bump strength, noise seed, two spares.
    extra: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> planet: PlanetSurfaceData;

// How many bands the uniform holds. Must match `PLANET_BAND_LIMIT`.
const PLANET_BAND_LIMIT: u32 = 6u;

// How many octaves the surface grain and the band warp each carry. Three: one
// for the mass, one for the break-up, one for the grain a close pass sees.
const NOISE_OCTAVES: u32 = 3u;

// How much each octave shrinks against the one before it.
const NOISE_GAIN: f32 = 0.5;

// How much each octave's frequency grows against the one before it. Not two:
// an exact doubling lines the octaves' cell walls up and the field grows a
// faint grid.
const NOISE_LACUNARITY: f32 = 2.17;

// How far the normal tilts per unit of measured grain slope. Tuned by eye
// against the close pass; a planet is not a hammered metal sheet.
const BUMP_GAIN: f32 = 6.0;

// How far the roughness may swing inside one band, as a fraction. Enough that
// a dust plain is not one flat sheen, small enough that it stays one material.
const ROUGHNESS_GRAIN: f32 = 0.35;

// Where the bump starts and where it reaches full strength, read against the
// band's own roughness.
//
// A glossy band is a sea or a sheet of ice, and both are flat by definition.
// Bending a flat, glossy surface along the grain does not read as texture: it
// reads as per-pixel sparkle, because every bent normal catches a different
// slice of the specular lobe. A rough band has no such lobe to break up, so it
// takes the whole bump.
const BUMP_ROUGHNESS_FLOOR: f32 = 0.05;
// Where the bump reaches full strength (see BUMP_ROUGHNESS_FLOOR).
const BUMP_ROUGHNESS_CEILING: f32 = 0.55;

// How far the grain may move a band threshold, as a fraction of the height
// range.
//
// Small, and it is not about the coastline - the coarse warp draws that. This
// breaks the FACET: the elevation a threshold reads is recovered from the
// interpolated fragment position, so its contours across one triangle are
// straight lines and a band edge follows the mesh. The grain is the only field
// here fine enough to hide a facet, and reusing it costs nothing.
const EDGE_GRAIN: f32 = 0.035;

// How far the coarse warp may move a polar cap's latitude. A cap cut on the
// raw latitude is a ruled line across the body; this makes it a coastline.
const CAP_WARP: f32 = 0.06;

// Where the grain fades out, in noise cells per pixel.
//
// A cell smaller than a pixel cannot be resolved and can only alias, and the
// grain drives a NORMAL as well as a colour - an aliasing normal sparkles.
// Fading it is the whole level-of-detail scheme: at backdrop range the bands
// and the mesh carry the body, and at close range the grain comes back.
const GRAIN_FADE_START: f32 = 0.5;
// Where the grain is gone entirely (see GRAIN_FADE_START).
const GRAIN_FADE_END: f32 = 1.5;

// One lattice corner's value, in [0, 1).
//
// FNV-1a's multiply, the hash this project already derives asteroid seeds and
// axis stretches with, but with an AVALANCHE STEP between components and a
// multiplicative finalizer. An integer hash rather than the usual
// fract-of-a-sine: a sine hash's quality depends on the driver's transcendental
// precision, and this one is exact everywhere.
//
// The avalanche is not optional and it is not tidiness. FNV mixes one BYTE per
// round; folding a whole 32-bit coordinate in per round leaves neighbouring
// cells differing by a near-constant amount, and the field grows a visible
// comb along an axis. Measured over an 80x80x6 lattice, neighbour-to-neighbour
// correlation without the shift is 0.54 along y and 0.81 along z - the streaks
// that put this line here. With it, all three axes measure below 0.002.
fn hash_cell(cell: vec3<i32>, seed: u32) -> f32 {
    var hash: u32 = 2166136261u ^ seed;
    hash = (hash ^ u32(cell.x + 65536)) * 16777619u;
    hash = hash ^ (hash >> 13u);
    hash = (hash ^ u32(cell.y + 65536)) * 16777619u;
    hash = hash ^ (hash >> 13u);
    hash = (hash ^ u32(cell.z + 65536)) * 16777619u;
    hash = hash ^ (hash >> 13u);
    hash = hash * 2654435761u;
    hash = hash ^ (hash >> 16u);
    return f32(hash >> 8u) * (1.0 / 16777216.0);
}

// Trilinearly interpolated value noise, in [0, 1].
fn value_noise(point: vec3<f32>, seed: u32) -> f32 {
    let base = floor(point);
    let cell = vec3<i32>(base);
    let offset = point - base;
    // Smoothstep the interpolation weights, or the lattice shows as creases.
    let weight = offset * offset * (3.0 - 2.0 * offset);

    let c000 = hash_cell(cell + vec3<i32>(0, 0, 0), seed);
    let c100 = hash_cell(cell + vec3<i32>(1, 0, 0), seed);
    let c010 = hash_cell(cell + vec3<i32>(0, 1, 0), seed);
    let c110 = hash_cell(cell + vec3<i32>(1, 1, 0), seed);
    let c001 = hash_cell(cell + vec3<i32>(0, 0, 1), seed);
    let c101 = hash_cell(cell + vec3<i32>(1, 0, 1), seed);
    let c011 = hash_cell(cell + vec3<i32>(0, 1, 1), seed);
    let c111 = hash_cell(cell + vec3<i32>(1, 1, 1), seed);

    let x00 = mix(c000, c100, weight.x);
    let x10 = mix(c010, c110, weight.x);
    let x01 = mix(c001, c101, weight.x);
    let x11 = mix(c011, c111, weight.x);
    return mix(mix(x00, x10, weight.y), mix(x01, x11, weight.y), weight.z);
}

// Fractal value noise, in [0, 1].
fn fbm(point: vec3<f32>, seed: u32) -> f32 {
    var total = 0.0;
    var amplitude = 1.0;
    var range = 0.0;
    var at = point;
    for (var octave = 0u; octave < NOISE_OCTAVES; octave = octave + 1u) {
        total = total + value_noise(at, seed + octave) * amplitude;
        range = range + amplitude;
        amplitude = amplitude * NOISE_GAIN;
        at = at * NOISE_LACUNARITY;
    }
    return total / range;
}

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    let world_from_local = get_world_from_local(in.instance_index);
    // The body's own axes and its uniform scale. Dividing the offset by the
    // scale puts the fragment back in the MESH's unit space, which is the space
    // the displacement range in `shape` is measured in.
    let scale = length(world_from_local[0].xyz);
    let axis_x = world_from_local[0].xyz / scale;
    let axis_y = normalize(world_from_local[1].xyz);
    let axis_z = normalize(world_from_local[2].xyz);
    let origin = world_from_local[3].xyz;

    let offset = in.world_position.xyz - origin;
    let local = vec3<f32>(
        dot(offset, axis_x),
        dot(offset, axis_y),
        dot(offset, axis_z),
    ) / scale;

    let radial = max(length(local), 1e-4);
    let direction = local / radial;
    let height = clamp((radial - planet.shape.x) / max(planet.shape.y, 1e-4), 0.0, 1.0);

    let seed = u32(planet.extra.y);

    // The grain: one field, read three times, doing four jobs - the colour
    // variation inside a band, the roughness swing, the normal bump, and the
    // fine break-up of a band edge.
    let grain_frequency = planet.detail.w;
    let grain = fbm(direction * grain_frequency, seed);
    // How many grain cells one pixel spans. Past one there is nothing to
    // resolve, so the grain fades out rather than aliasing.
    let footprint = length(fwidth(direction)) * grain_frequency;
    let fade = clamp(
        (GRAIN_FADE_END - footprint) / (GRAIN_FADE_END - GRAIN_FADE_START),
        0.0,
        1.0,
    );

    // The warp: a second, much coarser field added to the elevation the band
    // threshold reads. This is what turns a contour ring into a coastline, and
    // it costs one add rather than a second palette lookup.
    let warp = fbm(direction * planet.detail.y, seed + 97u);
    let banded = clamp(
        height
            + planet.detail.x * (warp * 2.0 - 1.0)
            + EDGE_GRAIN * fade * (grain * 2.0 - 1.0),
        0.0,
        1.0,
    );

    // Warped as well, or a cap is a ruled line drawn across the body.
    let latitude = clamp(abs(direction.y) + CAP_WARP * (warp * 2.0 - 1.0), 0.0, 1.0);

    // Hard thresholds, last match wins. A cap band sets a latitude floor and
    // no height floor, so it claims its latitude at any elevation - which is
    // what puts sea ice on a temperate world's poles.
    var color = planet.bands[0].color;
    var surface = planet.bands[0].surface;
    for (var index = 1u; index < PLANET_BAND_LIMIT; index = index + 1u) {
        if (f32(index) >= planet.shape.z) {
            break;
        }
        let band = planet.bands[index];
        if (banded >= band.surface.y && latitude >= band.surface.z) {
            color = band.color;
            surface = band.surface;
        }
    }

    let shade = 1.0 + planet.detail.z * fade * (grain * 2.0 - 1.0);
    let tinted = color.rgb * shade;

    // MULTIPLIED into the authored base colour rather than replacing it, so a
    // tint on the standard material still tints.
    pbr_input.material.base_color = pbr_input.material.base_color * vec4<f32>(tinted, 1.0);
    pbr_input.material.perceptual_roughness = clamp(
        surface.x * (1.0 + ROUGHNESS_GRAIN * fade * (grain * 2.0 - 1.0)),
        0.04,
        1.0,
    );
    // Emissive bypasses camera exposure, so a band that glows carries a
    // multiplier in the tens rather than a fraction (see the round record).
    pbr_input.material.emissive = vec4<f32>(tinted * color.a, 1.0);

    // Bend the shading normal along the same grain field. Without this a close
    // pass reads as painted plastic: the mesh carries the mountains and nothing
    // carries anything smaller than a facet.
    let bump = planet.extra.x
        * fade
        * smoothstep(BUMP_ROUGHNESS_FLOOR, BUMP_ROUGHNESS_CEILING, surface.x);
    if (bump > 0.0) {
        let reference = select(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 0.0, 1.0), abs(direction.z) < 0.9);
        let tangent = normalize(cross(reference, direction));
        let bitangent = cross(direction, tangent);
        // A fraction of a noise cell: close enough to measure the local slope,
        // far enough not to be measuring float noise.
        let step = 0.35 / max(grain_frequency, 1.0);
        let along = fbm(normalize(direction + tangent * step) * grain_frequency, seed);
        let across = fbm(normalize(direction + bitangent * step) * grain_frequency, seed);

        let slope = vec2<f32>(along - grain, across - grain) * bump * BUMP_GAIN;
        let local_bend = -(tangent * slope.x + bitangent * slope.y);
        let bend = axis_x * local_bend.x + axis_y * local_bend.y + axis_z * local_bend.z;
        pbr_input.N = normalize(pbr_input.N + bend);
    }

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
