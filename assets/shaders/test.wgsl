@group(0) @binding(0)
var<storage, read_write> results: array<f32, 64>;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) invocation_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    results[invocation_id.x] = f32(invocation_id.x);
}