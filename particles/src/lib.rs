use aftgraphs::prelude::*;
use aftgraphs_macros::sim_main;
use physics::Physics;
use rand::prelude::*;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::num::NonZeroU64;
use web_time::Instant;

mod physics;

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

const RADIUS: f32 = 0.0625;

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

const MAX_VELOCITY: f32 = 0.5;

struct Particles {
    pipeline: RenderPipeline,
    instances: InstanceBuffer<Vertex, Instance>,
    indices: IndexBuffer<u16>,
    aspect_ratio: Uniform<Float>,
    distribution: rand::distributions::Uniform<f32>,
    velocity_distribution: rand::distributions::Uniform<f32>,
    angle_distribution: rand::distributions::Uniform<f32>,
    rng: ThreadRng,
    physics: Physics,
    first_frame: Instant,
}
impl Simulation for Particles {
    fn new<P: UiPlatform>(renderer: &Renderer<P>) -> Self {
        let module = include_wgsl!(concat!(env!("CARGO_MANIFEST_DIR"), "/res/particles.wgsl"));

        let initial_instances = vec![Instance {
            position: [0.0, 0.0],
            radius: RADIUS,
            color: [1.0; 3],
        }];

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

        let distribution = rand::distributions::Uniform::new_inclusive(-1.0, 1.0);
        let velocity_distribution = rand::distributions::Uniform::new_inclusive(0.0, MAX_VELOCITY);
        let angle_distribution = rand::distributions::Uniform::new(0.0, std::f32::consts::TAU);
        let mut rng = thread_rng();

        let center = loop {
            let x = rng.sample(distribution);
            let y = rng.sample(distribution);

            if x <= -1.0 + RADIUS || x >= 1.0 - RADIUS {
                continue;
            }
            if y <= -1.0 + RADIUS * aspect_ratio.0 || y >= 1.0 - RADIUS * aspect_ratio.0 {
                continue;
            }

            break (x, y);
        };

        let velocity = rng.sample(velocity_distribution);
        let angle = rng.sample(angle_distribution);
        let velocity = (velocity * angle.cos(), velocity * angle.sin());

        let mut physics = Physics::new(0.0, RADIUS, aspect_ratio.0).unwrap();
        physics.push_circle(center, velocity);

        Self {
            pipeline,
            instances,
            indices,
            aspect_ratio,
            distribution,
            velocity_distribution,
            angle_distribution,
            rng,
            physics,
            first_frame: Instant::now(),
        }
    }

    async fn on_input(&mut self, _event: InputEvent) {}

    async fn render<P: UiPlatform>(
        &mut self,
        renderer: &Renderer<P>,
        mut render_pass: RenderPass<'_>,
        inputs: &mut HashMap<String, InputValue>,
    ) {
        let now = Instant::now();

        self.physics
            .update_aspect_ratio(renderer.aspect_ratio as f32);
        self.physics
            .simulate(now.duration_since(self.first_frame).as_secs_f32());

        self.aspect_ratio
            .update(renderer, Float(renderer.aspect_ratio as f32));

        if let Some(inp) = inputs.get_mut("controls.count") {
            let val = if let &mut InputValue::SLIDER(val) = inp {
                val as usize
            } else {
                self.physics.len()
            };
            *inp = InputValue::SLIDER(val as f64);

            match val.cmp(&self.physics.len()) {
                Ordering::Less => {
                    self.physics.pop(self.physics.len() - val);

                    let mut instances = self.instances.modify(renderer);
                    instances.instances_drain(val..);
                }
                Ordering::Greater => {
                    let old_length = self.physics.len();
                    self.spawn(val - old_length);

                    if self.physics.len() == old_length {
                        *inp = InputValue::SLIDER(old_length as f64);
                    }
                }
                Ordering::Equal => (),
            }
        }

        {
            let mut instances = self.instances.modify(renderer);
            *instances.instances_vec() = self
                .physics
                .get_state(now.duration_since(self.first_frame).as_secs_f32());
        }

        render_pass.set_pipeline(&self.pipeline);
        self.instances.bind(&mut render_pass, 0, 1);
        self.indices.bind(&mut render_pass);
        self.aspect_ratio.bind(&mut render_pass, 0);
        render_pass.draw_indexed(self.indices.range(), 0, self.instances.range_instance());
    }
}

impl Particles {
    fn spawn(&mut self, num: usize) {
        let mut idx = 0;
        let mut failed_circles = 0;

        while idx < num && failed_circles < 50 {
            let x = self.rng.sample(self.distribution);
            let y = self.rng.sample(self.distribution);

            let velocity = self.rng.sample(self.velocity_distribution);
            let angle = self.rng.sample(self.angle_distribution);
            let velocity = (velocity * angle.cos(), velocity * angle.sin());

            if x <= -1.0 + RADIUS || x >= 1.0 - RADIUS {
                failed_circles += 1;
                continue;
            }
            if y <= -1.0 + RADIUS * self.aspect_ratio.0 || y >= 1.0 - RADIUS * self.aspect_ratio.0 {
                failed_circles += 1;
                continue;
            }

            if !self.physics.push_circle((x, y), velocity) {
                failed_circles += 1;
                continue;
            }

            failed_circles = 0;
            idx += 1;
        }
    }
}

sim_main! { "/res/particles.toml", Particles }
