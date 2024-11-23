use crate::input::{InputState, InputValue, Inputs};
use crate::simulation::Simulation;
use crate::ui::{Ui, UiDrawError, UiPlatform};
use async_std::sync::Mutex;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use thiserror::Error;
use winit::window::Window;

#[cfg(not(target_arch = "wasm32"))]
mod linux;
#[cfg(target_arch = "wasm32")]
mod wasm;

pub mod builder;
pub use builder::{BindGroupLayoutBuilder, RenderPipelineBuilder, ShaderBuilder};
pub use wgpu::RenderPass;

pub static BINDING_UNIFORM_BUFFER: wgpu::BindingType = wgpu::BindingType::Buffer {
    ty: wgpu::BufferBindingType::Uniform,
    has_dynamic_offset: false,
    min_binding_size: None,
};

pub struct RendererPass {
    pub encoder: wgpu::CommandEncoder,
    pub frame: Option<wgpu::SurfaceTexture>,
    pub view: Option<wgpu::TextureView>,
}

pub struct Shader<'a> {
    shader: wgpu::ShaderModule,
    vs_entry: &'a str,
    fs_entry: Option<&'a str>,
    buffers: Vec<wgpu::VertexBufferLayout<'a>>,
    targets: Vec<Option<wgpu::ColorTargetState>>,
}

pub struct RenderPipeline {
    pub pipeline: wgpu::RenderPipeline,
    pub layout: wgpu::PipelineLayout,
}

impl AsRef<wgpu::RenderPipeline> for RenderPipeline {
    fn as_ref(&self) -> &wgpu::RenderPipeline {
        &self.pipeline
    }
}

impl AsRef<wgpu::PipelineLayout> for RenderPipeline {
    fn as_ref(&self) -> &wgpu::PipelineLayout {
        &self.layout
    }
}

impl AsMut<wgpu::RenderPipeline> for RenderPipeline {
    fn as_mut(&mut self) -> &mut wgpu::RenderPipeline {
        &mut self.pipeline
    }
}

impl AsMut<wgpu::PipelineLayout> for RenderPipeline {
    fn as_mut(&mut self) -> &mut wgpu::PipelineLayout {
        &mut self.layout
    }
}

impl Deref for RenderPipeline {
    type Target = wgpu::RenderPipeline;

    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}

impl DerefMut for RenderPipeline {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.pipeline
    }
}

pub struct Renderer<'a, P: UiPlatform> {
    pub headless: bool,
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub render_pass: Mutex<Option<RendererPass>>,
    pub surface: Option<wgpu::Surface<'a>>,
    pub queue: wgpu::Queue,
    pub config: Option<wgpu::SurfaceConfiguration>,
    pub texture: Option<wgpu::Texture>,
    pub texture_view: Option<wgpu::TextureView>,
    pub buffer: Option<wgpu::Buffer>,
    pub platform: P,
    pub ui: Ui,
    pub aspect_ratio: f64,
    pub time: f64,
    pub delta_time: f64,
}

#[derive(Error, Clone, Debug)]
pub enum RenderError {
    #[error("WGPU surface dropped frame: {0}")]
    DrawFrameDropped(#[from] wgpu::SurfaceError),
    #[error("draw_ui called without an active rendering pass active or an active WGPU surface")]
    DrawUiMissingRenderPass,
    #[error("drawing Ui failed: {0}")]
    DrawUiError(#[from] UiDrawError),
    #[error("attempted headless rendering without an active WGPU texture_view")]
    HeadlessWithoutTextureView,
    #[error("attempted to finish headless rendering pass without an active WGPU texture")]
    HeadlessWithoutTexture,
    #[error("render_headless_finished called without an active WGPU buffer")]
    HeadlessWithoutBuffer,
    #[error("render operation used without an active rendering pass")]
    MissingRenderPass,
    #[error("failed to map WGPU buffer to CPU slice")]
    FailedBufferMap,
}

impl<'a, P: UiPlatform> Renderer<'a, P> {
    async fn render_display<T: Simulation>(
        &self,
        surface: &wgpu::Surface<'_>,
        simulation: Arc<Mutex<T>>,
        input_values: &mut HashMap<String, InputValue>,
    ) {
        let mut pass = self.render_pass.lock().await;
        let frame = match surface.get_current_texture() {
            Ok(frame) => frame,
            Err(e) => {
                log::error!("aftgraphs::render::Renderer::display_render: dropped frame: {e:?}");
                return;
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("aftgraphs::render::Renderer::render_display"),
            });

        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("aftgraphs::render::Renderer::render_display"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        simulation
            .lock()
            .await
            .render(self, render_pass, input_values)
            .await;

        *pass = Some(RendererPass {
            encoder,
            frame: Some(frame),
            view: Some(view),
        });
    }

    #[cfg(target_arch = "wasm32")]
    async fn render_headless<T: Simulation>(
        &self,
        _simulation: Arc<Mutex<T>>,
        _input_values: &mut HashMap<String, InputValue>,
    ) {
        panic!("aftgraphs::render::Renderer::render_headless: headless rendering not supported on WASM")
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn render_headless<T: Simulation>(
        &self,
        simulation: Arc<Mutex<T>>,
        input_values: &mut HashMap<String, InputValue>,
    ) {
        let mut pass = self.render_pass.lock().await;

        let view = if let Some(ref texture_view) = self.texture_view {
            texture_view
        } else {
            panic!("aftgraphs::render::Renderer::render_headless: No target texture");
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("aftgraphs::render::Renderer::render_headless"),
            });

        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("aftgraphs::render::Renderer::render_headless"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        simulation
            .lock()
            .await
            .render(self, render_pass, input_values)
            .await;

        *pass = Some(RendererPass {
            encoder,
            frame: None,
            view: None,
        })
    }

    pub async fn render<T: Simulation>(
        &self,
        simulation: Arc<Mutex<T>>,
        input_values: &mut HashMap<String, InputValue>,
    ) {
        if let Some(surface) = self.surface.as_ref() {
            self.render_display(surface, simulation, input_values).await;
        } else {
            self.render_headless(simulation, input_values).await;
        }
    }

    pub async fn render_headless_finish(&self, out_img: &mut Vec<u8>) -> Result<(), RenderError> {
        use RenderError as RE;

        let u32_size = std::mem::size_of::<u32>() as u32;
        let texture = self.texture.as_ref().ok_or_else(|| {
            log::error!(
                "aftgraphs::render::Renderer::render_headless_finish: {}",
                RE::HeadlessWithoutTexture,
            );
            RE::HeadlessWithoutTexture
        })?;
        let texture_size = texture.size();

        let mut pass = self.render_pass.lock().await;
        let mut pass = pass.take().ok_or_else(|| {
            log::error!(
                "aftgraphs::render::Renderer::render_headless_finish: {}",
                RE::MissingRenderPass
            );
            RE::MissingRenderPass
        })?;

        let buffer = self.buffer.as_ref().ok_or_else(|| {
            log::error!(
                "aftgraphs::render::Renderer::render_headless_finish: {}",
                RE::HeadlessWithoutBuffer
            );
            RE::HeadlessWithoutBuffer
        })?;

        let bytes_per_row = u32_size * texture_size.width;
        let missing_bytes = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT
            - (bytes_per_row % wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let bytes_per_row = bytes_per_row + missing_bytes;

        pass.encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::ImageCopyBuffer {
                buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(texture_size.height),
                },
            },
            texture_size,
        );

        self.queue.submit(Some(pass.encoder.finish()));

        if out_img.len() != buffer.size() as usize {
            out_img.resize(buffer.size() as usize, 0);
        }

        {
            let buffer_slice = buffer.slice(..);
            let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();
            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                tx.send(result).expect("aftgraphs::render::Renderer::render_headless_finish: map_async closure failed to send");
            });
            self.device.poll(wgpu::Maintain::Wait);
            rx.receive()
                .await
                .ok_or_else(|| {
                    log::error!(
                        "aftgraphs::render::Renderer::render_headless_finish: {}",
                        RE::FailedBufferMap,
                    );
                    RE::FailedBufferMap
                })?
                .map_err(|e| {
                    log::error!(
                        "aftgraphs::render::Renderer::render_headless_finish: {}: {e:?}",
                        RE::FailedBufferMap
                    );
                    RE::FailedBufferMap
                })?;

            let data = buffer_slice.get_mapped_range();
            out_img.clone_from_slice(&data[..]);
        }

        buffer.unmap();
        Ok(())
    }

    pub async fn draw_ui(
        &mut self,
        window: Option<&Window>,
        inputs: &Inputs,
        state: InputState,
    ) -> Result<(), RenderError> {
        use RenderError as RE;

        let ui = self.ui.context_mut();

        let frame = ui.new_frame();
        inputs.render(frame, state).await;

        let mut pass = self.render_pass.lock().await;
        if pass.is_none() {
            let surface = self.surface.as_ref().ok_or_else(|| {
                log::error!(
                    "aftgraphs::render::Renderer::draw_ui: {}",
                    RE::DrawUiMissingRenderPass
                );
                RE::DrawUiMissingRenderPass
            })?;

            let frame = surface.get_current_texture()?;

            let view = frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("aftgraphs::render::Renderer::draw_ui"),
                });
            *pass = Some(RendererPass {
                encoder,
                frame: Some(frame),
                view: Some(view),
            });
        }

        {
            let pass = unsafe { pass.as_mut().unwrap_unchecked() };
            if let Some(window) = window {
                self.platform.prepare_render(frame, window);
            }

            let view = pass
                .view
                .as_ref()
                .map_or(self.texture_view.as_ref(), Option::Some)
                .ok_or_else(|| {
                    log::error!(
                        "aftgraphs::render::Renderer::draw_ui: {}",
                        RE::HeadlessWithoutTextureView
                    );
                    RE::HeadlessWithoutTextureView
                })?;

            let mut render_pass = pass.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("aftgraphs::render::Renderer::draw_ui"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.ui.draw(&mut render_pass, &self.queue, &self.device)?;
        }

        if !self.headless {
            let pass = unsafe { pass.take().unwrap_unchecked() };
            self.queue.submit(Some(pass.encoder.finish()));
            if let Some(frame) = pass.frame {
                frame.present();
            }
        }

        Ok(())
    }
}
