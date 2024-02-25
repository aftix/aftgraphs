struct VertexInput {
    @align(16) @location(0) quad_pos: vec2<f32>, // (-1, 1)
}

struct InstanceInput {
    @location(1) position: vec2<f32>,
    @location(2) radius: f32,
    @location(3) color: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) quad_pos: vec2<f32>, // (-1, 1)
}

@group(0) @binding(0) var<uniform> aspect_ratio: f32;

@vertex
fn vs_main(vertex: VertexInput, instance: InstanceInput) -> VertexOutput {
    var out: VertexOutput;
    out.color = instance.color;
    let quad_pos = vec2<f32>(vertex.quad_pos.x * instance.radius, vertex.quad_pos.y * instance.radius * aspect_ratio);
    out.clip_position = vec4<f32>(quad_pos + instance.position, 1.0, 1.0);
    out.quad_pos = vertex.quad_pos;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let distance = length(in.quad_pos);
    if (distance <= 1.0) {
        return vec4<f32>(in.color, 1.0);
    } else {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    };
}
