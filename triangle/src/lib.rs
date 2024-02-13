use aftgraphs::prelude::*;
use aftgraphs_macros::sim_main;
use std::collections::HashMap;

struct TriangleSimulation {
    pipeline: RenderPipeline,
}

impl Simulation for TriangleSimulation {
    async fn render(
        &mut self,
        renderer: Arc<Mutex<Renderer>>,
        _inputs: &HashMap<String, InputValue>,
        out_img: Arc<Mutex<Vec<u8>>>,
    ) {
        let renderer = renderer.lock().await;
        renderer.render(&self.pipeline, 0..3, 0..1, out_img).await
    }

    fn new(renderer: &Renderer) -> Self {
        let module = include_wgsl!(concat!(env!("CARGO_MANIFEST_DIR"), "/res/triangle.wgsl"));

        let shader = ShaderBuilder::new()
            .with_module(module)
            .with_default_fs_entrypoint()
            .build(renderer);

        let pipeline = RenderPipelineBuilder::new()
            .with_layout_label(Some("TriangleSimulation"))
            .with_pipeline_label(Some("TriangleSimulation"))
            .with_vertex_shader(shader)
            .build(renderer);

        Self { pipeline }
    }
}

sim_main! { "/res/triangle.toml", TriangleSimulation }
