use aftgraphs::prelude::*;
use aftgraphs_macros::sim_main;
use std::{collections::HashMap, num::NonZeroU64};

#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(C, align(16))]
struct Float(f32);

unsafe impl bytemuck::Zeroable for Float {}
unsafe impl bytemuck::NoUninit for Float {}

struct TriangleSimulation {
    pipeline: RenderPipeline,
    rotation: Uniform<Float>,
    color: Uniform<Float>,
    mouse_enabled: bool,
    snap_rotation: Option<f32>,
}

impl TriangleSimulation {
    fn update_inputs(&mut self, renderer: &Renderer, inputs: &HashMap<String, InputValue>) {
        if let Some(&InputValue::SLIDER(val)) = inputs.get("triangle inputs.rotation") {
            let val = (val as f32).to_radians();
            self.rotation.update(renderer, Float(val));
        }

        if let Some(&InputValue::SLIDER(val)) = inputs.get("triangle inputs.color") {
            self.color.update(renderer, Float(val as f32));
        }

        if let Some(&InputValue::CHECKBOX(val)) = inputs.get("triangle inputs.mouseInput") {
            self.mouse_enabled = val;
        }
    }
}

impl Simulation for TriangleSimulation {
    async fn render(
        &mut self,
        renderer: &Renderer,
        mut render_pass: RenderPass<'_>,
        inputs: &mut HashMap<String, InputValue>,
    ) {
        self.update_inputs(renderer, inputs);

        if let Some(snap) = self.snap_rotation.take() {
            self.rotation.update(renderer, Float(snap.to_radians()));
            inputs.insert(
                "triangle inputs.rotation".to_owned(),
                InputValue::SLIDER(snap as f64),
            );
        }

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, self.rotation.bind_group(), &[]);
        render_pass.set_bind_group(1, self.color.bind_group(), &[]);
        render_pass.draw(0..3, 0..1);
    }

    async fn on_input(&mut self, input: InputEvent) {
        if let InputEvent::Mouse(state, button, position) = input {
            if self.mouse_enabled && state.is_pressed() && matches!(button, MouseButton::Left) {
                let click_angle = position.1.atan2(position.0).to_degrees() as f32;
                let click_angle = if click_angle <= 0.0 {
                    360.0 + click_angle
                } else {
                    click_angle
                };

                // Click angle is now [0, 360) position of the click
                // We want the second (initially upwards) vertex to point there
                // tri_rotation + 90 deg = rotation of 2nd vertex
                // => snap_angle + 90 = click_angle

                let snap_angle = if click_angle - 90.0 < 0.0 {
                    click_angle + 270.0
                } else {
                    click_angle - 90.0
                };
                self.snap_rotation = Some(snap_angle);
            }
        }
    }

    fn new(renderer: &Renderer) -> Self {
        let module = include_wgsl!(concat!(env!("CARGO_MANIFEST_DIR"), "/res/triangle.wgsl"));

        let rotation_layout = BindGroupLayoutBuilder::new()
            .with_label(Some("TriangleSimulation::rotation"))
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
        let color_layout = BindGroupLayoutBuilder::new()
            .with_label(Some("TriangleSimulation::color"))
            .with_entry(BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(16u64),
                },
                count: None,
            })
            .build(renderer);

        let rotation: Uniform<Float> = UniformBuilder::new()
            .with_label(Some("TriangleSimulation::rotation"))
            .with_bind_group_layout(rotation_layout)
            .with_zero_data()
            .build(renderer);
        let color: Uniform<Float> = UniformBuilder::new()
            .with_label(Some("TriangleSimulation::color"))
            .with_bind_group_layout(color_layout)
            .with_zero_data()
            .build(renderer);

        let shader = ShaderBuilder::new()
            .with_module(module)
            .with_default_fs_entrypoint()
            .build(renderer);

        let pipeline = RenderPipelineBuilder::new()
            .with_layout_label(Some("TriangleSimulation::pipeline_layout"))
            .with_pipeline_label(Some("TriangleSimulation::pipeline"))
            .with_vertex_shader(shader)
            .with_bind_group_layout(rotation.bind_group_layout())
            .with_bind_group_layout(color.bind_group_layout())
            .build(renderer);

        Self {
            pipeline,
            rotation,
            color,
            mouse_enabled: false,
            snap_rotation: None,
        }
    }
}

sim_main! { "/res/triangle.toml", TriangleSimulation }
