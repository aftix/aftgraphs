struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec3<f32>,
    @location(2) quad_pos: vec2<f32>, // (-1, 1)
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) quad_pos: vec2<f32>, // (-1, 1)
}

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.color = model.color;
    out.clip_position = vec4<f32>(model.position, 1.0, 1.0);
    out.quad_pos = model.quad_pos;
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
