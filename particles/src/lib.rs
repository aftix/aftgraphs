use aftgraphs::prelude::*;
use aftgraphs_macros::sim_main;
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(C, align(16))]
struct Vertex {
    position: [f32; 2],
    color: [f32; 3],
    quad_pos: [f32; 2],
}

unsafe impl bytemuck::Zeroable for Vertex {}
unsafe impl bytemuck::NoUninit for Vertex {}

impl Vertex {
    pub fn new(center: (f32, f32), radius: f32, color: [f32; 3], aspect_ratio: f64) -> Vec<Self> {
        let y_radius = radius * aspect_ratio as f32;
        vec![
            Vertex {
                position: [center.0 - radius, center.1 + y_radius],
                color,
                quad_pos: [-1.0, 1.0],
            },
            Vertex {
                position: [center.0 + radius, center.1 + y_radius],
                color,
                quad_pos: [1.0, 1.0],
            },
            Vertex {
                position: [center.0 - radius, center.1 - y_radius],
                color,
                quad_pos: [-1.0, -1.0],
            },
            Vertex {
                position: [center.0 + radius, center.1 - y_radius],
                color,
                quad_pos: [1.0, -1.0],
            },
        ]
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(C, align(16))]
struct Float(f32);

unsafe impl bytemuck::Zeroable for Float {}
unsafe impl bytemuck::NoUninit for Float {}

#[allow(dead_code)]
struct Particles {
    pipeline: RenderPipeline,
    vertices: VertexBuffer<Vertex>,
    indices: IndexBuffer<u16>,
    radius: f32,
    particles: Vec<(f32, f32)>,
}

impl Simulation for Particles {
    fn new(renderer: &Renderer) -> Self {
        let module = include_wgsl!(concat!(env!("CARGO_MANIFEST_DIR"), "/res/particles.wgsl"));

        let particles = vec![(0.0, 0.0)];
        let radius = 0.125;

        let initial_vertices: Vec<_> = particles
            .iter()
            .flat_map(|&center| Vertex::new(center, radius, [1.0; 3], renderer.aspect_ratio).into_iter())
            .collect();

        let vertices = VertexBufferBuilder::new()
            .with_initial_vertices(initial_vertices.as_slice())
            .with_label(Some("aftgraphs::particles::Particles::vertices"))
            .with_attributes(&[
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32x2,
                },
                VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as BufferAddress,
                    shader_location: 1,
                    format: VertexFormat::Float32x3,
                },
                VertexAttribute {
                    offset: std::mem::size_of::<[f32; 5]>() as BufferAddress,
                    shader_location: 2,
                    format: VertexFormat::Float32x2,
                },
            ])
            .build(renderer);

        let indices = IndexBuffer::with_vec(
            renderer,
            vec![2, 1, 0, 2, 1, 3],
            IndexFormat::Uint16,
            Some("aftgraphs::particles::Particles::indices"),
        );

        let shader = ShaderBuilder::new()
            .with_module(module)
            .with_default_fs_entrypoint()
            .with_buffer(vertices.layout())
            .build(renderer);

        let pipeline = RenderPipelineBuilder::new()
            .with_vertex_shader(shader)
            .build(renderer);

        Self {
            pipeline,
            vertices,
            indices,
            radius,
            particles,
        }
    }

    async fn on_input(&mut self, _event: InputEvent) {}

    async fn render(
        &mut self,
        _renderer: &Renderer,
        mut render_pass: RenderPass<'_>,
        _inputs: &mut HashMap<String, InputValue>,
    ) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.vertices.as_vertex_buffer());
        render_pass.set_index_buffer(self.indices.as_index_buffer(), self.indices.format());
        render_pass.draw_indexed(self.indices.range(), 0, 0..1);
    }
}

sim_main! { "/res/particles.toml", Particles }
