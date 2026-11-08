import bevy_pbr::{
    render::pbr_fragment::pbr_input_from_standard_material,
    render::pbr_functions::alpha_discard,
    render::mesh_functions::{get_world_from_local, mesh_position_local_to_world},
    render::view_transformations::position_world_to_clip,
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

struct DirectionalMagnitudeMaterialData {
    magnitude_input: f32,
    radius: f32,
    max_height: f32,
    // WebGL2 support: structs must be 16 byte aligned.
    @if(SIXTEEN_BYTE_ALIGNMENT)
    _webgl2_padding_16b: u32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> material: DirectionalMagnitudeMaterialData;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) blend_color: vec4<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    let r: f32 = length(vertex.position.xz);
    let max_r: f32 = material.radius;
    let f: f32 = clamp(smoothstep(max_r, 0.0, r), 0.0, 1.0);

    let offset_amount = clamp(f * material.magnitude_input, 0.0, material.max_height);
    var pos = vertex.position + vec3<f32>(0.0, offset_amount, 0.0);

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
    pbr_input.material.base_color = alpha_discard(pbr_input.material.flags, pbr_input.material.alpha_cutoff, pbr_input.material.base_color);
    var out: FragmentOutput;
@if(PREPASS_PIPELINE) {
    // in deferred mode we can't modify anything after that, as lighting is run in a separate fullscreen shader.
    out = deferred_output(in, pbr_input);
} @else {
    // apply lighting
    out.color = apply_pbr_lighting(pbr_input);

    // apply in-shader post processing (fog, alpha-premultiply, and also tonemapping, debanding if the camera is non-hdr)
    // note this does not include fullscreen postprocessing effects like bloom.
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
}

    return out;
}
