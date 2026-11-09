import bevy_pbr::{
    render::pbr_fragment::pbr_input_from_standard_material,
    render::pbr_functions::alpha_discard,
    render::mesh_functions::get_world_from_local,
};

@if(PREPASS_PIPELINE)
import bevy_pbr::{
    prepass::io::{VertexOutput, FragmentOutput},
    deferred::functions::deferred_output,
};
@else
import bevy_pbr::{
    render::forward_io::{VertexOutput, FragmentOutput},
    render::pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
};

struct DirectionalSphereMaterialData {
    radius: f32,
    sharpness: f32,
    // WebGL2 support: structs must be 16 byte aligned.
    @if(SIXTEEN_BYTE_ALIGNMENT)
    _webgl2_padding_16b1: u32,
    // WebGL2 support: structs must be 16 byte aligned.
    @if(SIXTEEN_BYTE_ALIGNMENT)
    _webgl2_padding_16b2: u32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> material: DirectionalSphereMaterialData;

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
    pbr_input.material.base_color = alpha_discard(pbr_input.material.flags, pbr_input.material.alpha_cutoff, pbr_input.material.base_color);
    var out: FragmentOutput;
@if(PREPASS_PIPELINE) {
    // in deferred mode we can't modify anything after that, as lighting is run in a separate fullscreen shader.
    out = deferred_output(in, pbr_input);
} @else {
    // Directly output the color (ignoring lighting)
    out.color = pbr_input.material.base_color;

    // apply in-shader post processing (fog, alpha-premultiply, and also tonemapping, debanding if the camera is non-hdr)
    // note this does not include fullscreen postprocessing effects like bloom.
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
}

    return out;
}
