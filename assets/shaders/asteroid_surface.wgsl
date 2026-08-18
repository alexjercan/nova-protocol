// Triplanar rock surfacing: texture a body by WHERE IT IS rather than by UVs.
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

struct AsteroidSurfaceMaterialData {
    // Texture repeats per unit of the body's own local space.
    tiling: f32,
    // How hard the three projections cut over to each other. Higher is a
    // crisper transition on a face pointing between two axes.
    sharpness: f32,
#ifdef SIXTEEN_BYTE_ALIGNMENT
    // WebGL2 support: structs must be 16 byte aligned.
    _webgl2_padding_16b1: u32,
    // WebGL2 support: structs must be 16 byte aligned.
    _webgl2_padding_16b2: u32,
#endif
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> material: AsteroidSurfaceMaterialData;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var rock_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var rock_sampler: sampler;

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
    let local = vec3<f32>(
        dot(offset, axis_x),
        dot(offset, axis_y),
        dot(offset, axis_z),
    ) * material.tiling;

    let world_normal = normalize(in.world_normal.xyz);
    let normal = vec3<f32>(
        dot(world_normal, axis_x),
        dot(world_normal, axis_y),
        dot(world_normal, axis_z),
    );

    // Blend the three projections by how much the face points along each axis.
    var weight = pow(abs(normal), vec3<f32>(material.sharpness));
    weight = weight / max(weight.x + weight.y + weight.z, 1e-4);

    // Each projection drops the axis it looks along.
    let rock = sample_tile(local.zy) * weight.x
        + sample_tile(local.xz) * weight.y
        + sample_tile(local.xy) * weight.z;

    // MULTIPLIED into the authored base colour rather than replacing it, so a
    // tint on the standard material still tints and a damage grade still
    // grades.
    pbr_input.material.base_color = pbr_input.material.base_color * rock;

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
