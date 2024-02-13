@group(0) @binding(0)
var<uniform> rotation: f32;

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> @builtin(position) vec4<f32> {
    let x = f32(i32(in_vertex_index) - 1);
    let y = f32(i32(in_vertex_index & 1u) * 2 - 1);

    let c = cos(rotation);
    let s = sin(rotation);

    let new_x = x * c - y * s;
    let new_y = x * s + y * c;

    return vec4<f32>(new_x, new_y, 0.0, 1.0);
}

@group(1) @binding(0)
var<uniform> color: f32;

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4(1.0, color, 0.0, 1.0);
}
