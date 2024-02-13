use aftgraphs::prelude::*;
use aftgraphs_macros::sim_main;
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(C, align(16))]
struct Float(f32);

unsafe impl bytemuck::Zeroable for Float {}
unsafe impl bytemuck::NoUninit for Float {}

struct TriangleSimulation {
    pipeline: RenderPipeline,
    rotation: Uniform<Float>,
    color: Uniform<Float>,
}

impl Simulation for TriangleSimulation {
    async fn render(
        &mut self,
        renderer: &Renderer,
        mut render_pass: RenderPass<'_>,
        inputs: &HashMap<String, InputValue>,
    ) {
        if let Some(val) = inputs.get("triangle inputs.rotation") {
            if let &InputValue::SLIDER(val) = val {
                let val = (val as f32).to_radians();
                self.rotation.update(renderer, Float(val));
            }
        }

        if let Some(val) = inputs.get("triangle inputs.color") {
            if let &InputValue::SLIDER(val) = val {
                self.color.update(renderer, Float(val as f32));
            }
        }

        render_pass.set_pipeline(&self.pipeline.pipeline);
        render_pass.set_bind_group(0, self.rotation.bind_group(), &[]);
        render_pass.set_bind_group(1, self.color.bind_group(), &[]);
        render_pass.draw(0..3, 0..1);
    }

    fn new(renderer: &Renderer) -> Self {
        let module = include_wgsl!(concat!(env!("CARGO_MANIFEST_DIR"), "/res/triangle.wgsl"));

        let rotation_layout = BindGroupLayoutBuilder::new()
            .with_label(Some("TriangleSimulation::rotation"))
            .with_entry(BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BINDING_UNIFORM_BUFFER,
                count: None,
            })
            .build(renderer);
        let color_layout = BindGroupLayoutBuilder::new()
            .with_label(Some("TriangleSimulation::color"))
            .with_entry(BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BINDING_UNIFORM_BUFFER,
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
            .with_layout_label(Some("TriangleSimulation"))
            .with_pipeline_label(Some("TriangleSimulation"))
            .with_vertex_shader(shader)
            .with_bind_group_layout(rotation.bind_group_layout())
            .with_bind_group_layout(color.bind_group_layout())
            .build(renderer);

        Self {
            pipeline,
            rotation,
            color,
        }
    }
}

sim_main! { "/res/triangle.toml", TriangleSimulation }
