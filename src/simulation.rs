use async_mutex::Mutex;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;
use winit::{
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    keyboard::{Key, NamedKey},
    window::Window,
};

use crate::block_on;
use crate::input::{InputState, InputValue, Inputs};
use crate::render::Renderer;

pub trait Simulation: 'static {
    #[allow(async_fn_in_trait)]
    async fn render(
        &mut self,
        renderer: &Renderer,
        render_pass: wgpu::RenderPass<'_>,
        inputs: &HashMap<String, InputValue>,
    );

    fn new(renderer: &Renderer) -> Self;
}

pub struct SimulationContext<T: Simulation> {
    event_loop: EventLoop<()>,
    renderer: Rc<Mutex<Renderer>>,
    window: Arc<Mutex<Option<Window>>>,
    simulation: T,
}

mod builder;
pub use builder::{BuilderState, SimulationBuilder};

impl<T: Simulation> SimulationContext<T> {
    pub async fn run(self, inputs: Inputs, out_img: Arc<Mutex<Vec<u8>>>) {
        let simulation = Arc::new(Mutex::new(self.simulation));
        let input_values = InputState::default();
        let mut last_frame = Instant::now();

        self.event_loop
            .run(move |event, win_target| match event {
                Event::WindowEvent {
                    event: WindowEvent::Resized(size),
                    ..
                } => {
                    let renderer = self.renderer.clone();
                    let window = self.window.clone();

                    block_on(async move {
                        let mut renderer = renderer.lock().await;

                        if size.width > 0 && size.height > 0 {
                            renderer.config.as_mut().unwrap().width = size.width;
                            renderer.config.as_mut().unwrap().height = size.height;
                            renderer
                                .surface
                                .as_ref()
                                .unwrap()
                                .configure(&renderer.device, renderer.config.as_ref().unwrap());
                        }

                        if let Some(win) = window.lock().await.as_ref() {
                            renderer.handle_event(win, &event);
                            win.request_redraw();
                        }
                    });
                }
                Event::WindowEvent {
                    event:
                        WindowEvent::KeyboardInput {
                            event:
                                KeyEvent {
                                    logical_key: Key::Named(NamedKey::Escape),
                                    state: ElementState::Pressed,
                                    ..
                                },
                            ..
                        },
                    ..
                }
                | Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    win_target.exit();
                }
                Event::NewEvents(_) => {
                    let now = Instant::now();
                    let delta_time = now - last_frame;
                    last_frame = now;

                    let renderer = self.renderer.clone();
                    block_on(async move {
                        let mut renderer = renderer.lock().await;
                        renderer.update_delta_time(delta_time)
                    });
                }
                Event::AboutToWait => {
                    let renderer = self.renderer.clone();
                    let window = self.window.clone();

                    block_on(async move {
                        let mut renderer = renderer.lock().await;
                        let window = window.lock().await;

                        if let Some(window) = window.as_ref() {
                            renderer.prepare_ui(window).await;
                            window.request_redraw();
                        }
                    });
                }
                Event::WindowEvent {
                    event: WindowEvent::RedrawRequested,
                    ..
                } => {
                    let renderer = self.renderer.clone();
                    let window = self.window.clone();
                    let simulation = simulation.clone();
                    let input_values = input_values.clone();
                    let inputs = inputs.clone();
                    let out_img = out_img.clone();

                    block_on(async move {
                        {
                            let input_values = input_values.lock().await;
                            renderer
                                .lock()
                                .await
                                .render(simulation.clone(), input_values.as_ref(), out_img)
                                .await;
                        }

                        let mut renderer = renderer.lock().await;
                        if let Some(window) = window.lock().await.as_ref() {
                            renderer.draw_ui(window, inputs, input_values).await;
                        }
                    });
                }
                event => {
                    let renderer = self.renderer.clone();
                    let window = self.window.clone();
                    block_on(async move {
                        let mut renderer = renderer.lock().await;
                        let window = window.lock().await;
                        if let Some(window) = window.as_ref() {
                            renderer.handle_event(window, &event);
                        }
                    });
                }
            })
            .unwrap();
    }
}
