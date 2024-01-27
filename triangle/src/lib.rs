use aftgraphs::prelude::*;
use aftgraphs_macros::sim_main;

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct TriangleSimulation;

impl Simulation for TriangleSimulation {
    async fn render(&mut self, renderer: Arc<Mutex<Renderer>>, out_img: Arc<Mutex<Vec<u8>>>) {
        let renderer = renderer.lock().await;
        renderer.render(0..3, 0..1, out_img).await
    }
}

sim_main! { "../res/triangle.wgsl", TriangleSimulation }
