#import bevy_pbr::{
    mesh_functions,
    forward_io::{Vertex, VertexOutput},
    view_transformations::position_world_to_clip,
}

//@group(2) @binding(100)
//var<uniform> heightmap_size: vec2f;
@group(1) @binding(100)
var heightmap: texture_2d;
@group(1) @binding(101)
var height_sampler: sampler;

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    var model = mesh_functions::get_model_matrix(vertex.instance_index);
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        vertex.instance_index
    );
    let height = textureSampleLevel(heightmap, height_sampler, vertex.position.xz, 0.0);
    //let height = 12.0;
    out.world_position = mesh_functions::mesh_position_local_to_world(model, vec4<f32>(vertex.position.x, height, vertex.position.z, 1.0));
    out.position = position_world_to_clip(out.world_position.xyz);
    out.uv = vertex.uv;
    return out;
}