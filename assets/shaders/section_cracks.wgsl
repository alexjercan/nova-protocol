// CRACKS: what damage does to a section's surface.
//
// Replaces the flat red tint every damaged section used to wear. A tint says
// "this is damaged" in one colour over a whole body, which is information and
// not a picture of anything - a hull at 60% looked like a hull painted red.
// Fracture lines say the same thing by showing the material failing, they say it
// WHERE it fails rather than everywhere at once, and they read at a glance
// against any authored paint because they are dark lines rather than a hue.
//
// LOCAL space, and for the reason the rock shader is: a section is bolted to a
// ship that flies and tumbles, and a world-space pattern would swim across it as
// it moved. The object's own axes are the normalized columns of its model
// matrix, so dotting against them is the inverse rotation with no matrix inverse
// in the fragment shader.

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

struct SectionCracksMaterialData {
    // How far gone the section is: 0 pristine, 1 dead.
    damage: f32,
    // Fracture lines per unit of the body's own local space.
    scale: f32,
#ifdef SIXTEEN_BYTE_ALIGNMENT
    // WebGL2 support: structs must be 16 byte aligned.
    _webgl2_padding_16b1: u32,
    // WebGL2 support: structs must be 16 byte aligned.
    _webgl2_padding_16b2: u32,
#endif
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> material: SectionCracksMaterialData;

// The widest a crack line gets, in the fracture field's own units.
//
// Small, and the first version was not: the field is three octaves of centred
// value noise, so its values cluster tightly about zero and a width of a third
// of its RANGE covered more than half the surface. That drew orange blotches,
// not fractures. At 0.09 a dead section is about a fifth cracked and a
// half-dead one about a twentieth, which reads as lines opening up.
const CRACK_WIDTH: f32 = 0.09;
// What a crack is: the dark of an opening in the material, not a colour.
const CRACK_COLOR: vec3<f32> = vec3<f32>(0.035, 0.028, 0.026);
// What is left of a section that has spent all of its health. Cold, so a dead
// section standing on a wreck reads as wreckage and not as something still hot.
const BURNT_COLOR: vec3<f32> = vec3<f32>(0.05, 0.04, 0.04);
// Where the burn starts taking over from the cracks.
const BURNT_FROM: f32 = 0.78;
// The heat that shows THROUGH a crack once a section is critical, before it
// goes cold. What used to be the whole-body red glow, now coming out of the
// openings it should have been coming out of.
const CRACK_GLOW: vec3<f32> = vec3<f32>(1.7, 0.22, 0.04);
// Where that heat starts.
const GLOW_FROM: f32 = 0.55;

fn hash(cell: vec3<f32>) -> f32 {
    // One sine-free hash: sines differ across drivers and a pattern that shifts
    // between machines is not a pattern.
    var h = dot(cell, vec3<f32>(127.1, 311.7, 74.7));
    h = fract(h * 0.1031);
    h *= h + 33.33;
    h *= h + h;
    return fract(h);
}

// Value noise: hash the eight corners of the cell and interpolate smoothly.
fn value_noise(at: vec3<f32>) -> f32 {
    let cell = floor(at);
    let f = fract(at);
    let w = f * f * (3.0 - 2.0 * f);

    let c000 = hash(cell + vec3<f32>(0.0, 0.0, 0.0));
    let c100 = hash(cell + vec3<f32>(1.0, 0.0, 0.0));
    let c010 = hash(cell + vec3<f32>(0.0, 1.0, 0.0));
    let c110 = hash(cell + vec3<f32>(1.0, 1.0, 0.0));
    let c001 = hash(cell + vec3<f32>(0.0, 0.0, 1.0));
    let c101 = hash(cell + vec3<f32>(1.0, 0.0, 1.0));
    let c011 = hash(cell + vec3<f32>(0.0, 1.0, 1.0));
    let c111 = hash(cell + vec3<f32>(1.0, 1.0, 1.0));

    let x00 = mix(c000, c100, w.x);
    let x10 = mix(c010, c110, w.x);
    let x01 = mix(c001, c101, w.x);
    let x11 = mix(c011, c111, w.x);
    return mix(mix(x00, x10, w.y), mix(x01, x11, w.y), w.z);
}

// Three octaves, centred on zero. The ZERO SET of this is the crack: a surface
// through the volume, so what it draws on a face is a continuous line rather
// than a scatter of spots, and it carries on across the seam onto the next face
// because it is a property of the volume and not of the surface.
fn fracture(at: vec3<f32>) -> f32 {
    var value = value_noise(at) - 0.5;
    value += (value_noise(at * 2.17) - 0.5) * 0.5;
    value += (value_noise(at * 4.41) - 0.5) * 0.25;
    return value;
}

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    let world_from_local = get_world_from_local(in.instance_index);
    let axis_x = normalize(world_from_local[0].xyz);
    let axis_y = normalize(world_from_local[1].xyz);
    let axis_z = normalize(world_from_local[2].xyz);
    let origin = world_from_local[3].xyz;

    let offset = in.world_position.xyz - origin;
    let local = vec3<f32>(
        dot(offset, axis_x),
        dot(offset, axis_y),
        dot(offset, axis_z),
    ) * material.scale;

    let damage = clamp(material.damage, 0.0, 1.0);
    // Width goes to zero at zero damage, so a pristine section is EXACTLY what
    // the artist painted - no line, no darkening, nothing.
    let width = damage * damage * CRACK_WIDTH;
    let crack = 1.0 - smoothstep(0.0, max(width, 1e-5), abs(fracture(local)));

    let burnt = smoothstep(BURNT_FROM, 1.0, damage);
    let glow = smoothstep(GLOW_FROM, 1.0, damage) * (1.0 - burnt);

    var colour = pbr_input.material.base_color.rgb;
    colour = mix(colour, CRACK_COLOR, crack);
    colour = mix(colour, BURNT_COLOR, burnt);
    pbr_input.material.base_color = vec4<f32>(colour, pbr_input.material.base_color.a);
    pbr_input.material.emissive = vec4<f32>(
        // Squared, so the heat is in the CORE of a crack rather than smeared
        // across its soft edge - a glow as wide as the line reads as paint.
        pbr_input.material.emissive.rgb + CRACK_GLOW * crack * crack * glow,
        pbr_input.material.emissive.a,
    );

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
