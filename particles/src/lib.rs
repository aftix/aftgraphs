use aftgraphs::{block_on, prelude::*, spawn};
use aftgraphs_macros::sim_main;
use async_std::channel::{self, Receiver, Sender};

use physics::Physics;
use rand::prelude::*;
use std::{cmp::Ordering, collections::HashMap, num::NonZeroU64};

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
    physics_thread: PhysicsMessager,
}

#[derive(Debug)]
enum PhysicsMessage {
    Simulate(f32),
    GetState(f32),
    GetLength,
    Length(usize),
    AspectRatio(f32),
    State(Vec<Instance>),
    PushCircle((f32, f32), (f32, f32)),
    PushCircleResponse(bool),
    Pop(usize),
}

struct PhysicsMessager {
    send: Sender<PhysicsMessage>,
    recv: Receiver<PhysicsMessage>,
    _handle: Handle,
}

impl Drop for PhysicsMessager {
    fn drop(&mut self) {
        self.send.close();
        self.recv.close();
    }
}

fn physics_thread(
    radius: f32,
    aspect_ratio: f32,
    main_rx: Receiver<PhysicsMessage>,
    worker_tx: Sender<PhysicsMessage>,
) {
    block_on(async move {
        let mut physics = Physics::new(0.0, radius, aspect_ratio)
            .expect("aftgraphs::particles::PhysicsMessager: failed to create Physics");
        log::debug!("aftgraphs::particles::PhysicsMessager: Starting physics loop");

        while let Ok(msg) = main_rx.recv().await {
            match msg {
                PhysicsMessage::Simulate(t) => {
                    physics.simulate(t);
                }
                PhysicsMessage::GetState(t) => {
                    let state = physics.get_state(t);
                    if let Err(e) = worker_tx.send(PhysicsMessage::State(state)).await {
                        log::warn!("aftgraphs::particles::PhysicsMessager: failed to send: {e}");
                    }
                }
                PhysicsMessage::PushCircle(circle, velocity) => {
                    let resp = physics.push_circle(circle, velocity);
                    if let Err(e) = worker_tx
                        .send(PhysicsMessage::PushCircleResponse(resp))
                        .await
                    {
                        log::warn!("aftgraphs::particles::PhysicsMessager: failed to send: {e}");
                    }
                }
                PhysicsMessage::Pop(num) => {
                    physics.pop(num);
                }
                PhysicsMessage::AspectRatio(aspect_ratio) => {
                    physics.update_aspect_ratio(aspect_ratio);
                }
                PhysicsMessage::GetLength => {
                    if let Err(e) = worker_tx.send(PhysicsMessage::Length(physics.len())).await {
                        log::warn!("aftgraphs::particles::PhysicsMessager: failed to send: {e}");
                    }
                }
                msg => log::warn!(
                    "aftgraphs::particles::PhysicsMessager: invalid message recieved: {msg:?}"
                ),
            }
        }
        log::debug!("aftgraphs::particles::PhysicsMessager: Ending physics loop");
    });
}

impl PhysicsMessager {
    async fn new(_display: bool, radius: f32, aspect_ratio: f32) -> Self {
        let (main_tx, main_rx) = channel::bounded(10);
        let (worker_tx, worker_rx) = channel::bounded(10);

        let closure = move || physics_thread(radius, aspect_ratio, main_rx, worker_tx);
        let handle = spawn(closure)
            .await
            .expect("aftgraphs::particles::PhysicsMessager: Failed to spawn thread");

        Self {
            send: main_tx,
            recv: worker_rx,
            _handle: handle,
        }
    }

    async fn len(&self) -> usize {
        self.send
            .send(PhysicsMessage::GetLength)
            .await
            .expect("aftgraphs::particles::PhysicsMessager::len: failed to send message");

        match self.recv.recv().await {
            Ok(PhysicsMessage::Length(len)) => len,
            msg => {
                panic!("aftgraphs::particles::PhysicsMessager::len: invalid response {msg:?}")
            }
        }
    }

    async fn update_aspect_ratio(&self, aspect_ratio: f32) {
        self.send
            .send(PhysicsMessage::AspectRatio(aspect_ratio))
            .await.expect("aftgraphs::particles::PhysicsMessager::update_aspect_ratio: failed to send message");
    }

    async fn simulate(&self, t: f32) {
        self.send
            .send(PhysicsMessage::Simulate(t))
            .await
            .expect("aftgraphs::particles::PhysicsMessager::simulate: failed to send message");
    }

    async fn get_state(&self, t: f32) -> Vec<Instance> {
        self.send
            .send(PhysicsMessage::GetState(t))
            .await
            .expect("aftgraphs::particles::PhysicsMessager::get_state: failed to send message");

        match self.recv.recv().await {
            Ok(PhysicsMessage::State(state)) => state,
            msg => {
                panic!("aftgraphs::particles::PhysicsMessager::get_state: invalid response {msg:?}")
            }
        }
    }

    async fn push_circle(&mut self, circle: (f32, f32), velocity: (f32, f32)) -> bool {
        self.send
            .send(PhysicsMessage::PushCircle(circle, velocity))
            .await
            .expect("aftgraphs::particles::PhysicsMessager::push_circle: failed to send message");

        match self.recv.recv().await {
            Ok(PhysicsMessage::PushCircleResponse(resp)) => resp,
            msg => {
                panic!(
                    "aftgraphs::particles::PhysicsMessager::push_circle: invalid response {msg:?}"
                )
            }
        }
    }

    async fn pop(&self, num: usize) {
        self.send
            .send(PhysicsMessage::Pop(num))
            .await
            .expect("aftgraphs::particles::PhysicsMessager::push_circle: failed to send message");
    }
}

impl Simulation for Particles {
    async fn new<P: UiPlatform>(renderer: &Renderer<P>) -> Self {
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
        let rng = thread_rng();

        let physics_thread =
            PhysicsMessager::new(renderer.surface.is_some(), RADIUS, aspect_ratio.0).await;

        Self {
            pipeline,
            instances,
            indices,
            aspect_ratio,
            distribution,
            velocity_distribution,
            angle_distribution,
            rng,
            physics_thread,
        }
    }

    async fn on_input(&mut self, _event: InputEvent) {}

    async fn render<P: UiPlatform>(
        &mut self,
        renderer: &Renderer<P>,
        mut render_pass: RenderPass<'_>,
        inputs: &mut HashMap<String, InputValue>,
    ) {
        self.physics_thread
            .update_aspect_ratio(renderer.aspect_ratio as f32)
            .await;
        self.physics_thread.simulate(renderer.time as f32).await;

        self.aspect_ratio
            .update(renderer, Float(renderer.aspect_ratio as f32));

        if let Some(inp) = inputs.get_mut("controls.count") {
            let physics_len = self.physics_thread.len().await;

            let val = if let &mut InputValue::SLIDER(val) = inp {
                val as usize
            } else {
                physics_len
            };
            *inp = InputValue::SLIDER(val as f64);

            match val.cmp(&physics_len) {
                Ordering::Less => {
                    self.physics_thread.pop(physics_len - val).await;

                    let mut instances = self.instances.modify(renderer);
                    instances.instances_drain(val..);
                }
                Ordering::Greater => {
                    self.spawn(val - physics_len).await;

                    if self.physics_thread.len().await == physics_len {
                        *inp = InputValue::SLIDER(physics_len as f64);
                    }
                }
                Ordering::Equal => (),
            }
        }

        {
            let mut instances = self.instances.modify(renderer);
            *instances.instances_vec() = self.physics_thread.get_state(renderer.time as f32).await;
        }

        render_pass.set_pipeline(&self.pipeline);
        self.instances.bind(&mut render_pass, 0, 1);
        self.indices.bind(&mut render_pass);
        self.aspect_ratio.bind(&mut render_pass, 0);
        render_pass.draw_indexed(self.indices.range(), 0, self.instances.range_instance());
    }
}

impl Particles {
    async fn spawn(&mut self, num: usize) {
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

            if !self.physics_thread.push_circle((x, y), velocity).await {
                failed_circles += 1;
                continue;
            }

            failed_circles = 0;
            idx += 1;
        }
    }
}

sim_main! { "/res/particles.toml", Particles }
