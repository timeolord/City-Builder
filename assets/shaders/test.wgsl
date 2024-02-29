@group(0) @binding(0)
var<uniform> input: vec4<f32>;
@group(0) @binding(1)
var<storage, read_write> output: array<f32>;

@compute @workgroup_size(4, 1, 1)
fn main(@builtin(global_invocation_id) invocation_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    let index = (invocation_id.x + invocation_id.y * num_workgroups.x) * num_workgroups.z + invocation_id.z;
    output[index] = input[index];
}