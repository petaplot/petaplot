struct GridUniform {
    transform_matrix: mat4x4<f32>,
    grid_color: vec4<f32>,
    grid_spacing: vec2<f32>,
    _padding: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> grid: GridUniform;

struct VertexInput {
    @location(0) position: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_grid(vertex: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let pos_4d = vec4<f32>(vertex.position, 0.0, 1.0);
    out.clip_position = grid.transform_matrix * pos_4d;
    return out;
}

@fragment
fn fs_grid(in: VertexOutput) -> @location(0) vec4<f32> {
    return grid.grid_color;
}
