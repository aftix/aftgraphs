use async_mutex::Mutex;
use std::rc::Rc;
use std::{collections::HashMap, sync::Arc};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    keyboard::{Key, NamedKey},
    window::Window,
};

use crate::block_on;
use crate::input::{InputState, InputValue, Inputs};
use crate::render::Renderer;

pub use winit::event::{ElementState, KeyEvent, MouseButton};

pub enum InputEvent {
    Keyboard(KeyEvent),
    /// f64 pair is (x, y) coordinates in [-1, 1] space
    Mouse(ElementState, MouseButton, (f64, f64)),
}

pub trait Simulation: 'static {
    #[allow(async_fn_in_trait)]
    async fn render(
        &mut self,
        renderer: &Renderer,
        render_pass: wgpu::RenderPass<'_>,
        inputs: &mut HashMap<String, InputValue>,
    );

    #[allow(async_fn_in_trait)]
    async fn on_input(&mut self, event: InputEvent);

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
        log::debug!("aftgraphs::SimulationContext::run entered");

        let simulation = Arc::new(Mutex::new(self.simulation));
        let input_values = InputState::default();
        let mut last_frame = web_time::Instant::now();
        let mut cursor_position = PhysicalPosition::new(0.0, 0.0);

        let mut window_size: PhysicalSize<f64> = {
            let window = self.window.lock().await;
            if let Some(window) = window.as_ref() {
                let PhysicalSize { width, height } = window.inner_size();
                PhysicalSize::new(width.into(), height.into())
            } else {
                PhysicalSize::new(4.0, 4.0)
            }
        };

        // On wasm you want to wait until the first resize event to render anything
        let mut recieved_resize = false;

        log::debug!("aftgraphs::SimulationContext::run: Entering winit event_loop");
        self.event_loop
            .run(move |event, win_target| {
                win_target.set_control_flow(winit::event_loop::ControlFlow::Poll);
                match event {
                    Event::WindowEvent {
                        event: WindowEvent::Resized(size),
                        ..
                    } => {
                        log::info!(
                            "aftgraphs::SimulationContext::run: Handling window resize event"
                        );

                        recieved_resize = true;
                        let PhysicalSize { width, height } = size;
                        window_size = PhysicalSize::new(width.into(), height.into());

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
                        log::info!("aftgraphs::SimulationContext::run: Exit requested");
                        win_target.exit();
                    }
                    winit_event @ Event::WindowEvent {
                        event: WindowEvent::KeyboardInput { .. } , ..
                    } => {
                        log::debug!("aftgraphs::SimulationContext::run: KeyboardEvent event found on window");

                        let event = match &winit_event {
                            Event::WindowEvent { event: WindowEvent::KeyboardInput { event, .. }, .. } => event.clone()
                            ,
                            _ => unreachable!(),
                        };

                        let simulation = simulation.clone();
                        let window = self.window.clone();
                        let renderer = self.renderer.clone();

                        block_on(async move {
                            let mut simulation = simulation.lock().await;
                            simulation.on_input(InputEvent::Keyboard(event)).await;

                            let window = window.lock().await;
                            if let Some(window) = window.as_ref() {
                                let mut renderer = renderer.lock().await;
                                renderer.handle_event(window, &winit_event);
                            }
                        });
                    }
                    winit_event @ Event::WindowEvent { event: WindowEvent::CursorMoved { .. }, .. } => {
                        log::debug!("aftgraphs::SimulationContext::run: CursorMoved event found on window");

                        cursor_position = match &winit_event {
                          Event::WindowEvent { event: WindowEvent::CursorMoved { position, .. }, .. }  => *position,
                          _ => unreachable!(),
                        };

                        let window = self.window.clone();
                        let renderer = self.renderer.clone();

                        block_on(async move {
                            let window = window.lock().await;
                            if let Some(window) = window.as_ref() {
                                let mut renderer = renderer.lock().await;
                                renderer.handle_event(window, &winit_event)
                            }
                        });
                    }
                    winit_event @ Event::WindowEvent {
                        event: WindowEvent::MouseInput { .. }, ..
                    } => {
                        log::debug!("aftgraphs::SimulationContext::run: MouseInput event found on window");

                        let (state, button) = match &winit_event {
                            Event::WindowEvent { event: WindowEvent::MouseInput { state, button, .. }, .. } => (*state, *button),
                            _ => unreachable!(),
                        };

                        // Convert mouse coordinates to screen space
                        let position = (cursor_position.x / window_size.width, cursor_position.y / window_size.height);
                        let position = (position.0 * 2.0 - 1.0, 1.0 - position.1 * 2.0);

                        let simulation = simulation.clone();
                        let window = self.window.clone();
                        let renderer = self.renderer.clone();

                        block_on(async move {
                            let mut simulation = simulation.lock().await;
                            simulation.on_input(InputEvent::Mouse(state, button, position)).await;

                            let window = window.lock().await;
                            if let Some(window) = window.as_ref() {
                                let mut renderer = renderer.lock().await;
                                renderer.handle_event(window, &winit_event)
                            }
                        });
                    }
                    Event::NewEvents(_) => {
                        log::debug!(
                            "aftgraphs::SimulationContext::run: New events found on window"
                        );
                        let now = web_time::Instant::now();
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
                        log::debug!("aftgraphs::SimulationContext::run: window redraw requested");

                        if cfg!(target_arch = "wasm32") && !recieved_resize {
                            return;
                        }

                        let renderer = self.renderer.clone();
                        let window = self.window.clone();
                        let simulation = simulation.clone();
                        let input_values = input_values.clone();
                        let inputs = inputs.clone();
                        let out_img = out_img.clone();

                        block_on(async move {
                            {
                                log::debug!(
                                    "aftgraphs::SimulationContext::run: Rendering simulation"
                                );
                                let mut input_values = input_values.lock().await;
                                renderer
                                    .lock()
                                    .await
                                    .render(simulation.clone(), input_values.as_mut(), out_img)
                                    .await;
                            }

                            log::debug!("aftgraphs::SimulationContext::run: Updating input values");
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
                }
            })
            .expect("aftgraphs::SimulationContext::run: winit::event_loop::EventLoop::run unexpectedly failed");
    }
}
