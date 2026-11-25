@group(0) @binding(0)
var<storage, read> ensemble_members: array<f32>;

@group(0) @binding(1)
var<storage, read_write> output: atomic<u32>;

@group(0) @binding(2)
var<uniform> params: Params;

struct Params {
    member_count: u32,
    bucket_min: f32,
    bucket_max: f32,
    padding: u32,
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    
    if (idx >= params.member_count) {
        return;
    }
    
    let temp = ensemble_members[idx];
    
    if (temp >= params.bucket_min && temp < params.bucket_max) {
        atomicAdd(&output, 1u);
    }
}
