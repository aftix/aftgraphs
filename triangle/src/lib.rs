#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = "simMain")]
pub fn sim_main() {
    aftgraphs::sim_main(include_str!("../res/triangle.wgsl"));
}

#[cfg(not(target_arch = "wasm32"))]
pub fn sim_main() {
    aftgraphs::sim_main(include_str!("../res/triangle.wgsl"));
}
