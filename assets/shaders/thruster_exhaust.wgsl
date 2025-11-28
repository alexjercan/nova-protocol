#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
    mesh_functions::{get_world_from_local, mesh_position_local_to_world},
    view_transformations::position_world_to_clip,
    mesh_view_bindings::globals,
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

struct ThrusterExhaustMaterialData {
    thruster_input: f32,
    thruster_exhaust_radius: f32,
    thruster_exhaust_height: f32,
#ifdef SIXTEEN_BYTE_ALIGNMENT
    // WebGL2 support: structs must be 16 byte aligned.
    _webgl2_padding_16b: u32,
#endif
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> material: ThrusterExhaustMaterialData;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) blend_color: vec4<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    let r: f32 = length(vertex.position.xz);
    let max_r: f32 = material.thruster_exhaust_radius;
    let f: f32 = clamp(smoothstep(max_r, 0.0, r), 0.0, 1.0);
    let input: f32 = clamp(material.thruster_input, 0.0, 1.0);

    var offset_amount = f * material.thruster_input * material.thruster_exhaust_height;
    if vertex.position.y <= 0.0 {
        offset_amount = 0.0;
    }
    var pos = vertex.position + vec3<f32>(0.0, offset_amount, 0.0);

    let world_pos = get_world_from_local(vertex.instance_index) * vec4(pos, 1.0);
    let n = noise3(world_pos.xyz + vec3<f32>(0.0, globals.time * 5.0, 0.0));
    let wobble_amp = 0.1;
    if vertex.position.y > 0.0 {
        pos += vec3<f32>(0.0, n * wobble_amp, 0.0);
    }

    var world_from_local = get_world_from_local(vertex.instance_index);
    out.world_position = mesh_position_local_to_world(world_from_local, vec4(pos, 1.0));
    out.position = position_world_to_clip(out.world_position.xyz);

    return out;
}

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);

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

fn hash21(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453123);
}

fn noise2(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);

    let a = hash21(i + vec2<f32>(0.0, 0.0));
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));

    let x1 = mix(a, b, u.x);
    let x2 = mix(c, d, u.x);
    return mix(x1, x2, u.y);
}

fn noise3(p: vec3<f32>) -> f32 {
    let xy = noise2(p.xy);
    let yz = noise2(p.yz);
    let xz = noise2(vec2<f32>(p.x, p.z));

    return (xy + yz + xz) / 3.0;
}
