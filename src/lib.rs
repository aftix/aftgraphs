pub mod display;
#[cfg(not(target_arch = "wasm32"))]
pub mod headless;
pub mod input;
pub mod primitives;
pub mod render;
pub mod simulation;
pub mod ui;
pub mod uniform;
pub mod vertex;

#[cfg(not(target_arch = "wasm32"))]
mod cli;

pub mod prelude {
    pub use crate::input::{InputState, InputValue};
    pub use crate::render::{
        BindGroupLayoutBuilder, RenderPass, RenderPipeline, RenderPipelineBuilder, Renderer,
        ShaderBuilder, BINDING_UNIFORM_BUFFER,
    };
    pub use crate::simulation::{
        ElementState, InputEvent, MouseButton, RawKeyEvent, Simulation, SimulationContext,
    };
    pub use crate::ui::{Ui, UiFrame, UiPlatform};
    pub use crate::uniform::{Uniform, UniformBuilder};
    pub use crate::vertex::{IndexBuffer, VertexBuffer, VertexBufferBuilder, PRIMITIVE_POINTS};

    pub use async_mutex::Mutex;
    pub use bytemuck;
    pub use std::sync::Arc;
    pub use wgpu::{
        self, include_wgsl, BindGroupLayoutEntry, BindingType, BufferAddress, IndexFormat,
        ShaderStages, VertexAttribute, VertexFormat,
    };
}

#[cfg(not(target_arch = "wasm32"))]
mod linux;
#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(target_arch = "wasm32")]
pub use wasm::*;

#[cfg(not(target_arch = "wasm32"))]
pub use linux::*;
