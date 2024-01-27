pub mod display;
pub mod headless;
pub mod primitives;
pub mod render;
pub mod simulation;

pub mod prelude {
    pub use crate::render::Renderer;
    pub use crate::simulation::SimulationContext;
}

#[cfg(not(target_arch = "wasm32"))]
mod linux;
#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(target_arch = "wasm32")]
pub use wasm::*;

#[cfg(not(target_arch = "wasm32"))]
pub use linux::*;
