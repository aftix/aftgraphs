pub mod display;
pub mod headless;
pub mod input;
pub mod primitives;
pub mod render;
pub mod simulation;
pub mod ui;

pub mod prelude {
    pub use crate::input::{InputState, InputValue};
    pub use crate::render::{RenderPipeline, RenderPipelineBuilder, Renderer, ShaderBuilder};
    pub use crate::simulation::{Simulation, SimulationContext};
    pub use crate::ui::{Ui, UiFrame};
    pub use async_mutex::Mutex;
    pub use std::sync::Arc;
    pub use wgpu;
    pub use wgpu::include_wgsl;
}

#[cfg(not(target_arch = "wasm32"))]
mod linux;
#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(target_arch = "wasm32")]
pub use wasm::*;

#[cfg(not(target_arch = "wasm32"))]
pub use linux::*;
