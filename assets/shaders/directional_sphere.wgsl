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

struct DirectionalSphereMaterialData {
    radius: f32,
    sharpness: f32,
#ifdef SIXTEEN_BYTE_ALIGNMENT
    // WebGL2 support: structs must be 16 byte aligned.
    _webgl2_padding_16b1: u32,
    // WebGL2 support: structs must be 16 byte aligned.
    _webgl2_padding_16b2: u32,
#endif
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> material: DirectionalSphereMaterialData;

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    var world_from_local = get_world_from_local(in.instance_index);
    let local_forward = vec3(0.0, 0.0, -1.0);
    let world_forward = normalize((world_from_local * vec4(local_forward, 0.0)).xyz);

    let normal = normalize(in.world_normal.xyz);
    let d = max(dot(normal, world_forward), 0.0);
    let falloff = pow(d, material.sharpness);

    let col = pbr_input.material.base_color * falloff;
    pbr_input.material.base_color = col;

    // alpha discard
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);
#ifdef PREPASS_PIPELINE
    // in deferred mode we can't modify anything after that, as lighting is run in a separate fullscreen shader.
    let out = deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;
    // apply lighting
    // out.color = apply_pbr_lighting(pbr_input);
    // Directly output the color (ignoring lighting)
    out.color = pbr_input.material.base_color;

    // apply in-shader post processing (fog, alpha-premultiply, and also tonemapping, debanding if the camera is non-hdr)
    // note this does not include fullscreen postprocessing effects like bloom.
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
#endif

    return out;
}
