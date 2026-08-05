use wgpu::util::DeviceExt;
use wgpu::{
    BindGroup, BindGroupLayout, Buffer, BufferUsages, ColorTargetState, Device,
    PipelineLayoutDescriptor, Queue, RenderPipelineDescriptor, TextureFormat, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexStepMode,
};

use petaplot_core::compute::simd::MinMaxPair;
use crate::camera::ViewportCamera;

/// Pipeline de renderizado GPU para el dibujo acelerado de series temporales por instancias.
pub struct LineRenderPipeline {
    pub pipeline: wgpu::RenderPipeline,
    pub camera_buffer: Buffer,
    pub camera_bind_group: BindGroup,
    pub camera_bind_group_layout: BindGroupLayout,
    pub instance_buffer: Option<Buffer>,
    pub num_instances: u32,
}

impl LineRenderPipeline {
    /// Crea el pipeline de renderizado `wgpu` utilizando el shader WGSL instanciado.
    pub fn new(device: &Device, surface_format: TextureFormat, camera: &ViewportCamera) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Instanced Line Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/line.wgsl").into()),
        });

        let camera_uniform = camera.build_uniform();
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Uniform Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let instance_layout = VertexBufferLayout {
            array_stride: (std::mem::size_of::<f32>() * 3) as u64,
            step_mode: VertexStepMode::Instance,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32,
                },
                VertexAttribute {
                    offset: 4,
                    shader_location: 1,
                    format: VertexFormat::Float32,
                },
                VertexAttribute {
                    offset: 8,
                    shader_location: 2,
                    format: VertexFormat::Float32,
                },
            ],
        };

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Line Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Instanced Line Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[instance_layout],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            pipeline,
            camera_buffer,
            camera_bind_group,
            camera_bind_group_layout,
            instance_buffer: None,
            num_instances: 0,
        }
    }

    /// Actualiza la matriz de cámara en la GPU sin costo de re-asignación de geometría ($0\text{ ms}$ overhead).
    pub fn update_camera(&self, queue: &Queue, camera: &ViewportCamera) {
        let uniform = camera.build_uniform();
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[uniform]));
    }

    /// Carga los datos de las instancias de pares MinMax en el VBO de la GPU.
    pub fn upload_instances(&mut self, device: &Device, pairs: &[MinMaxPair], x_step: f32) {
        if pairs.is_empty() {
            self.num_instances = 0;
            return;
        }

        let mut raw_instance_data: Vec<f32> = Vec::with_capacity(pairs.len() * 3);
        for (i, pair) in pairs.iter().enumerate() {
            let x_pos = i as f32 * x_step;
            raw_instance_data.push(x_pos);
            raw_instance_data.push(pair.min);
            raw_instance_data.push(pair.max);
        }

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Line Instance Buffer"),
            contents: bytemuck::cast_slice(&raw_instance_data),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });

        self.instance_buffer = Some(buffer);
        self.num_instances = pairs.len() as u32;
    }

    /// Ejecuta el pase de renderizado en GPU.
    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        if let Some(ref instance_buffer) = self.instance_buffer {
            if self.num_instances > 0 {
                render_pass.set_pipeline(&self.pipeline);
                render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
                render_pass.set_vertex_buffer(0, instance_buffer.slice(..));
                render_pass.draw(0..2, 0..self.num_instances);
            }
        }
    }
}
