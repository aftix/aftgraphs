use crate::input::{InputState, InputValue, Inputs};
use crate::simulation::Simulation;
use crate::ui::{Ui, UiPlatform};
use async_mutex::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
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
    pub frame: wgpu::SurfaceTexture,
    pub view: wgpu::TextureView,
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

pub struct Renderer {
    pub headless: bool,
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub render_pass: Mutex<Option<RendererPass>>,
    pub surface: Option<wgpu::Surface>,
    pub queue: wgpu::Queue,
    pub config: Option<wgpu::SurfaceConfiguration>,
    pub texture: Option<wgpu::Texture>,
    pub texture_view: Option<wgpu::TextureView>,
    pub buffer: Option<wgpu::Buffer>,
    pub platform: UiPlatform,
    pub ui: Ui,
}

impl Renderer {
    async fn display_render<T: Simulation>(
        &self,
        simulation: Arc<Mutex<T>>,
        input_values: &mut HashMap<String, InputValue>,
    ) {
        let surface = self.surface.as_ref().unwrap();

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
        let encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("aftgraphs::render::Renderer::display_render"),
            });

        *pass = Some(RendererPass {
            encoder,
            frame,
            view,
        });

        let pass = unsafe { pass.as_mut().unwrap_unchecked() };
        {
            let render_pass = pass.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("aftgraphs::render::Renderer::display_render"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &pass.view,
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
        }
    }

    async fn headless_render<T: Simulation>(
        &self,
        simulation: Arc<Mutex<T>>,
        input_values: &mut HashMap<String, InputValue>,
        out_img: &mut [u8],
    ) {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("aftgraphs::render::Renderer::headless_render"),
            });

        {
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("aftgraphs::render::Renderer::headless_render"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: self.texture_view.as_ref().unwrap(),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            b: 0.0,
                            g: 0.0,
                            a: 1.0,
                        }),
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
        }

        let u32_size = std::mem::size_of::<u32>() as u32;
        let texture_size = self.texture.as_ref().map(|tex| tex.size()).unwrap();
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: self.texture.as_ref().unwrap(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::ImageCopyBuffer {
                buffer: self.buffer.as_ref().unwrap(),
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(u32_size * texture_size.width),
                    rows_per_image: Some(texture_size.height),
                },
            },
            texture_size,
        );

        let buffer_slice = self.buffer.as_ref().map(|buf| buf.slice(..)).unwrap();
        let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        self.device.poll(wgpu::Maintain::Wait);
        rx.receive().await.unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();
        out_img.clone_from_slice(&data[..]);
    }

    // Render a frame using the RenderPipeline, calling the closure draw
    // to gain access to the rendering pass
    pub async fn render<T: Simulation>(
        &self,
        simulation: Arc<Mutex<T>>,
        input_values: &mut HashMap<String, InputValue>,
        out_img: Arc<Mutex<Vec<u8>>>,
    ) {
        if !self.headless {
            self.display_render(simulation, input_values).await;
        } else {
            let out_img = out_img.clone();
            self.headless_render(
                simulation,
                input_values,
                out_img.lock().await.as_mut_slice(),
            )
            .await;
        }
    }

    pub async fn draw_ui(&mut self, window: &Window, inputs: Inputs, state: InputState) {
        let ui = self.ui.context_mut();

        let frame = ui.frame();
        inputs.render(frame, state).await;

        let mut pass = self.render_pass.lock().await;
        if pass.is_none() {
            let surface = self.surface.as_ref().unwrap();
            let frame = match surface.get_current_texture() {
                Ok(frame) => frame,
                Err(e) => {
                    log::error!("aftgraphs::render::Renderer::draw_ui: dropped frame: {e:?}");
                    return;
                }
            };
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
                frame,
                view,
            });
        }

        {
            let pass = unsafe { pass.as_mut().unwrap_unchecked() };
            self.platform.prepare_render(frame, window);

            let view = pass
                .frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut render_pass = pass.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("aftgraphs::render::Renderer::draw_ui"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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

            self.ui
                .draw(&mut render_pass, &self.queue, &self.device)
                .expect("Renderer::draw_ui: rendering failed");
        }

        {
            let pass = unsafe { pass.take().unwrap_unchecked() };
            self.queue.submit(Some(pass.encoder.finish()));
            pass.frame.present();
        }
    }
}
