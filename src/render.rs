use crate::prelude::Ui;
use crate::ui::UiPlatform;
use async_mutex::Mutex;
use std::ops::Range;
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
mod linux;
#[cfg(target_arch = "wasm32")]
mod wasm;

pub struct RendererPass {
    pub encoder: wgpu::CommandEncoder,
    pub frame: wgpu::SurfaceTexture,
    pub view: wgpu::TextureView,
}

pub struct Renderer {
    pub headless: bool,
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub render_pass: Mutex<Option<RendererPass>>,
    pub surface: Option<wgpu::Surface>,
    pub queue: wgpu::Queue,
    pub shader: wgpu::ShaderModule,
    pub render_pipeline: wgpu::RenderPipeline,
    pub pipeline_layout: wgpu::PipelineLayout,
    pub config: Option<wgpu::SurfaceConfiguration>,
    pub texture: Option<wgpu::Texture>,
    pub texture_view: Option<wgpu::TextureView>,
    pub buffer: Option<wgpu::Buffer>,
    pub platform: UiPlatform,
    pub ui: Ui,
}

impl Renderer {
    async fn display_render(&self, vertices: Range<u32>, indices: Range<u32>) {
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
            let mut render_pass = pass.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("display render pass"),
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
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.draw(vertices, indices)
        }
    }

    async fn headless_render(&self, vertices: Range<u32>, indices: Range<u32>, out_img: &mut [u8]) {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("display render enconder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("headless render pass"),
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

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.draw(vertices, indices);
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

    pub async fn render(
        &self,
        vertices: Range<u32>,
        indices: Range<u32>,
        out_img: Arc<Mutex<Vec<u8>>>,
    ) {
        if !self.headless {
            self.display_render(vertices, indices).await;
        } else {
            let out_img = out_img.clone();
            self.headless_render(vertices, indices, out_img.lock().await.as_mut_slice())
                .await;
        }
    }
}
