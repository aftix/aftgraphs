#[cfg(not(target_arch = "wasm32"))]
mod linux;
#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(not(target_arch = "wasm32"))]
pub use linux::*;
#[cfg(target_arch = "wasm32")]
pub use wasm::*;
