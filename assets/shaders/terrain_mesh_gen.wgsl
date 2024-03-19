@group(0) @binding(0)
var heightmap: texture_storage_2d<rgba8unorm, read_write>;
//@group(0) @binding(1)
//var vertices: texture_storage_2d<rgba32float, read_write>;
@group(0) @binding(1)
var<storage, read_write> buffer: array<f32>;
@group(0) @binding(2)
var<uniform> image_size: vec2<u32>;
@group(0) @binding(3)
var<uniform> world_size: vec2<u32>;

@compute @workgroup_size(8, 8, 1)
fn init(@builtin(global_invocation_id) invocation_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    var location = vec2<u32>(invocation_id.x * u32(4), invocation_id.y * u32(4));
    //let color = vec4<f32>(1.0, 0.0, 1.0, 1.0);
    //textureStore(texture, location, color);
    //location.y = location.y * u32(4);
    //location.x = location.x * u32(4);
    //let linear_position = location.y * map_size.x + location.x; 
    //vertices[linear_position] = textureLoad(texture, location);
    //vertices[0] = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    //for (var i = 0u; i < arrayLength(&buffer); i = i + 1u) {
    //    buffer[i] = f32(i);
    //}
    //vertices[0] = 1.0;
    let height = textureLoad(heightmap, location);
    let linear_position = (location.x * world_size.x) + location.y;
    buffer[linear_position] = f32(height.r);
    //buffer[linear_position] = f32(linear_position);
    //buffer[linear_position] = 12.0;
    //let vertex = vec4<f32>(f32(location.x), f32(height.r), f32(location.y), 0.0);
    //let color = vec4<f32>(0.3, 0.6, 0.9, 1.0);
    //textureStore(vertices, location, vertex);
    //textureStore(heightmap, location, color);
}

@compute @workgroup_size(8, 8, 1)
fn update(@builtin(global_invocation_id) invocation_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    //let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    //let randomNumber = randomFloat(invocation_id.y * num_workgroups.x + invocation_id.x);
    //let color = vec4<f32>(randomNumber, randomNumber, randomNumber, 1.0);
    //let color = vec4<f32>(1.0, 0.0, 1.0, 1.0);
    //textureStore(texture, location, color);
    //vertices[0] = 1.0;
    //for (var i = 0u; i < arrayLength(&vertices); i = i + 1u) {
    //    vertices[i] = 1.0;
    //}
    //for (var i = 0u; i < 10; i = i + 1u) {
    //    vertices[i] = 1.0;
    //}
    //var location = vec2<u32>(invocation_id.x, invocation_id.y);
    ////let location = vec2<u32>(u32(100), u32(100));
    //let color = vec4<f32>(1.0, 0.0, 1.0, 1.0);
    //textureStore(texture, location, color);
}

