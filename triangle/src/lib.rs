use aftgraphs::prelude::*;

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct TriangleSimulation;

impl Simulation for TriangleSimulation {
    async fn render(&mut self, renderer: Arc<Mutex<Renderer>>, out_img: Arc<Mutex<Vec<u8>>>) {
        let renderer = renderer.lock().await;
        renderer.render(0..3, 0..1, out_img).await
    }
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = "simMain")]
pub fn sim_main() {
    aftgraphs::sim_main(
        include_str!("../res/triangle.wgsl"),
        TriangleSimulation::default(),
    );
}

#[cfg(not(target_arch = "wasm32"))]
pub fn sim_main() {
    aftgraphs::sim_main(
        include_str!("../res/triangle.wgsl"),
        TriangleSimulation::default(),
    );
}
