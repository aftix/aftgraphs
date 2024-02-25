use aftgraphs::prelude::*;
use aftgraphs_macros::sim_main;
use std::collections::HashMap;
use std::num::NonZeroU64;

#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(C, align(16))]
struct Vertex {
    quad_pos: [f32; 2],
}

#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(C, align(16))]
struct Instance {
    position: [f32; 2],
    radius: f32,
    color: [f32; 3],
}

#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(C, align(16))]
struct Float(f32);

unsafe impl bytemuck::Zeroable for Vertex {}
unsafe impl bytemuck::NoUninit for Vertex {}

unsafe impl bytemuck::Zeroable for Instance {}
unsafe impl bytemuck::NoUninit for Instance {}

unsafe impl bytemuck::Zeroable for Float {}
unsafe impl bytemuck::NoUninit for Float {}

const QUAD: [Vertex; 4] = [
    Vertex {
        quad_pos: [-1.0, 1.0],
    },
    Vertex {
        quad_pos: [1.0, 1.0],
    },
    Vertex {
        quad_pos: [-1.0, -1.0],
    },
    Vertex {
        quad_pos: [1.0, -1.0],
    },
];

const INDICES: [u16; 6] = [2, 1, 0, 2, 1, 3];

#[allow(dead_code)]
struct Particles {
    pipeline: RenderPipeline,
    instances: InstanceBuffer<Vertex, Instance>,
    indices: IndexBuffer<u16>,
    aspect_ratio: Uniform<Float>,
    radius: f32,
    particles: Vec<(f32, f32)>,
}

impl Simulation for Particles {
    fn new<P: UiPlatform>(renderer: &Renderer<P>) -> Self {
        let module = include_wgsl!(concat!(env!("CARGO_MANIFEST_DIR"), "/res/particles.wgsl"));

        let particles = vec![(0.0, 0.0), (-0.5, 0.0), (0.5, 0.0), (0.0, -0.5), (0.0, 0.5)];
        let radius = 0.0625;

        let initial_instances: Vec<_> = particles
            .iter()
            .map(|&center| Instance {
                position: [center.0, center.1],
                radius,
                color: [1.0; 3],
            })
            .collect();

        let instances = InstanceBufferBuilder::new()
            .with_initial_vertices(QUAD.as_slice())
            .with_initial_instances_owned(initial_instances)
            .with_vertex_label(Some("aftgraphs::particles::Particles::vertices"))
            .with_instance_label(Some("aftgraphs::particles::Particles::instances"))
            .with_vertex_attributes_owned(vec![VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: VertexFormat::Float32x2,
            }])
            .with_instance_attributes_owned(vec![
                VertexAttribute {
                    offset: 0,
                    shader_location: 1,
                    format: VertexFormat::Float32x2,
                },
                VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as BufferAddress,
                    shader_location: 2,
                    format: VertexFormat::Float32,
                },
                VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as BufferAddress,
                    shader_location: 3,
                    format: VertexFormat::Float32x3,
                },
            ])
            .build(renderer);

        let indices = IndexBuffer::with_vec(
            renderer,
            INDICES.into(),
            IndexFormat::Uint16,
            Some("aftgraphs::particles::Particles::indices"),
        );

        let aspect_ratio_layout = BindGroupLayoutBuilder::new()
            .with_label(Some("aftgraphs::particles::Particles::aspect_ratio"))
            .with_entry(BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(16u64),
                },
                count: None,
            })
            .build(renderer);
        let aspect_ratio = UniformBuilder::new()
            .with_label(Some("aftgraphs::particles::Particles::aspect_ratio"))
            .with_bind_group_layout(aspect_ratio_layout)
            .with_data(Float(renderer.aspect_ratio as f32))
            .build(renderer);

        let shader = ShaderBuilder::new()
            .with_module(module)
            .with_default_fs_entrypoint()
            .with_buffer(instances.vertex_layout())
            .with_buffer(instances.instance_layout())
            .build(renderer);

        let pipeline = RenderPipelineBuilder::new()
            .with_vertex_shader(shader)
            .with_bind_group_layout(aspect_ratio.bind_group_layout())
            .build(renderer);

        Self {
            pipeline,
            instances,
            indices,
            aspect_ratio,
            radius,
            particles,
        }
    }

    async fn on_input(&mut self, _event: InputEvent) {}

    async fn render<P: UiPlatform>(
        &mut self,
        renderer: &Renderer<P>,
        mut render_pass: RenderPass<'_>,
        _inputs: &mut HashMap<String, InputValue>,
    ) {
        self.aspect_ratio
            .update(renderer, Float(renderer.aspect_ratio as f32));

        render_pass.set_pipeline(&self.pipeline);
        self.instances.bind(&mut render_pass, 0, 1);
        self.indices.bind(&mut render_pass);
        self.aspect_ratio.bind(&mut render_pass, 0);
        render_pass.draw_indexed(self.indices.range(), 0, self.instances.range_instance());
    }
}

sim_main! { "/res/particles.toml", Particles }
