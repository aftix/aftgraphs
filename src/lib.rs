pub mod display;
pub mod headless;
pub mod input;
pub mod primitives;
pub mod render;
pub mod simulation;

pub mod prelude {
    pub use crate::render::Renderer;
    pub use crate::simulation::{Simulation, SimulationContext};
    pub use async_mutex::Mutex;
    pub use std::sync::Arc;
}

#[cfg(not(target_arch = "wasm32"))]
mod linux;
#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(target_arch = "wasm32")]
pub use wasm::*;

#[cfg(not(target_arch = "wasm32"))]
pub use linux::*;
