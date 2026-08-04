struct CameraUniform {
    transform_matrix: mat4x4<f32>,
    color: vec4<f32>,
    line_width: f32,
    _padding: vec3<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct InstanceInput {
    @location(0) x_pos: f32,
    @location(1) y_min: f32,
    @location(2) y_max: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;

    // Asignar el valor y según si es el punto mínimo (vértice 0) o máximo (vértice 1)
    var y_val: f32 = instance.y_min;
    if (vertex_index % 2u == 1u) {
        y_val = instance.y_max;
    }

    let raw_position = vec4<f32>(instance.x_pos, y_val, 0.0, 1.0);
    out.clip_position = camera.transform_matrix * raw_position;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return camera.color;
}
