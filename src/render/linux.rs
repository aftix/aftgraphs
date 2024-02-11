use super::{Renderer, RendererPass};
use crate::input::{InputState, Inputs};
use std::time::Duration;
use winit::{event::Event, window::Window};

impl Renderer {
    pub fn handle_event<T>(&mut self, window: &Window, event: &Event<T>) {
        self.platform
            .0
            .handle_event(self.ui.context_mut().io_mut(), window, event);
    }

    pub async fn prepare_ui(&mut self, window: &Window) {
        let platform = &self.platform.0;
        platform
            .prepare_frame(self.ui.context_mut().io_mut(), window)
            .expect("aftgraphs::render::Renderer::prepare_ui: Failed to prepare frame");
    }

    pub fn update_delta_time(&mut self, duration: Duration) {
        self.ui.context_mut().io_mut().update_delta_time(duration)
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
            self.platform.0.prepare_render(frame, window);

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
