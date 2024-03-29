use crate::{
    block_on,
    input::{InputState, InputValue, Inputs},
    render::{RenderError, Renderer},
    ui::{UiPlatform, UiWinitPlatform},
};
use async_std::sync::Mutex;
use std::{collections::HashMap, rc::Rc, sync::Arc};
use thiserror::Error;
use web_time::Instant;
pub use winit::event::{ElementState, MouseButton, RawKeyEvent};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{Event, KeyEvent, Touch, TouchPhase, WindowEvent},
    event_loop::EventLoop,
    keyboard::{Key, NamedKey},
    window::Window,
};

pub enum InputEvent {
    Keyboard(RawKeyEvent),
    /// f64 pair is (x, y) coordinates in [-1, 1] space
    Mouse(ElementState, MouseButton, (f64, f64)),
}

pub trait Simulation: 'static {
    #[allow(async_fn_in_trait)]
    async fn render<P: UiPlatform>(
        &mut self,
        renderer: &Renderer<P>,
        render_pass: wgpu::RenderPass<'_>,
        inputs: &mut HashMap<String, InputValue>,
    );

    #[allow(async_fn_in_trait)]
    async fn on_input(&mut self, event: InputEvent);

    #[allow(async_fn_in_trait)]
    async fn new<P: UiPlatform>(renderer: &Renderer<P>) -> Self;
}

pub struct SimulationContext<T: Simulation, P: UiPlatform> {
    event_loop: Option<EventLoop<InputEvent>>,
    renderer: Rc<Mutex<Renderer<P>>>,
    window: Arc<Mutex<Option<Window>>>,
    simulation: T,
}

mod builder;
pub use builder::{BuilderState, SimulationBuilder};

#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "x264")]
mod encoder;

#[derive(Error, Clone, Debug)]
pub enum SimulationRunError {
    #[error("ran headless rendering on a binary not compiled with the 'x264' feature")]
    HeadlessWithoutx264,
    #[error("headless rendering used without an initialized draw target texture")]
    HeadlessWithoutTexture,
    #[error("headless rendering used without an output file")]
    HeadlessWithoutOutputFile,
    #[error("headless video encoding failed: {0}")]
    HeadlessEncodingError(String),
    #[error("display rendering used without a winit::event::EventLoop")]
    DisplayWithoutEventLoop,
    #[error("display rendering winit::event::EventLoop ended unexpectedly: {0}")]
    DisplayEventLoopFailure(String),
    #[error("rendering failed: {0}")]
    RenderFailure(#[from] RenderError),
}

impl<T: Simulation> SimulationContext<T, ()> {
    #[cfg(target_arch = "wasm32")]
    pub async fn run_headless(
        self,
        _inputs: Inputs,
        _out_img: Arc<Mutex<Vec<u8>>>,
    ) -> Result<(), SimulationRunError> {
        log::error!("aftgraphs::simulation::SimulationContext::run_headless not supported on WASM");
        unreachable!(
            "aftgraphs::simulation::SimulationContext::run_headless not supported on WASM"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[cfg(not(feature = "x264"))]
    pub async fn run_headless(
        self,
        inputs: Inputs,
        headless_inputs: crate::headless::HeadlessInput,
        out_img: Arc<Mutex<Vec<u8>>>,
    ) -> Result<(), SimulationRunError> {
        log::error!(
            "aftgraphs::simulation::SimulationContext::run_headless: {}",
            SimulationRunError::HeadlessWithoutx264
        );
        Err(SimulationRunError::HeadlessWithoutx264)
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[cfg(feature = "x264")]
    pub async fn run_headless(
        self,
        inputs: Inputs,
        headless_inputs: crate::headless::HeadlessInput,
        out_img: Arc<Mutex<Vec<u8>>>,
    ) -> Result<(), SimulationRunError> {
        use crate::cli::ARGUMENTS;
        use crate::headless::HeadlessMetadata;
        use web_time::Duration;
        use SimulationRunError as SRE;

        log::debug!("aftgraphs::simulation::SimulationContext::run_headless entered");

        let input_values = if let Some(ref initial) = headless_inputs.initial_inputs {
            let input_values = InputState::default();

            {
                let mut state = input_values.lock().await;
                let state = state.as_mut();
                for (name, val) in &initial.inputs {
                    let name = name.replace('_', " ").replace('-', ".");
                    state.insert(name, val.clone());
                }
            }
            input_values
        } else {
            InputState::default()
        };

        let HeadlessMetadata {
            duration,
            size: _,
            delta_t,
        } = headless_inputs.simulation;

        let mut events = headless_inputs.blocks;
        events.sort_by(|lhs, rhs| lhs.time.total_cmp(&rhs.time));
        let mut events = events.into_iter();
        let mut current_event = events.next();

        let mut renderer = self.renderer.lock().await;
        let simulation = Arc::new(Mutex::new(self.simulation));

        let size = renderer
            .texture
            .as_ref()
            .ok_or_else(|| {
                log::error!(
                    "aftgraphs::simulation::SimulationContext::run_headless: {}",
                    SRE::HeadlessWithoutTexture
                );
                SRE::HeadlessWithoutTexture
            })?
            .size();
        let size = (size.width, size.height);

        let mut out_img = out_img.lock().await;

        let (render_imgui, out_file) = {
            let args = ARGUMENTS.read().await;
            let headless = args.headless.clone().ok_or_else(|| {
                log::error!(
                    "aftgraphs::simulation::SimulationContext::run_headless: {}",
                    SRE::HeadlessWithoutOutputFile
                );
                SRE::HeadlessWithoutOutputFile
            })?;
            (args.render_imgui, headless.out_file)
        };

        let (send_frame, finished, handle) = encoder::encoder(size, delta_t, out_file);

        let mut time = 0.0;
        let delta_duration = Duration::from_secs_f64(delta_t);
        while time <= duration {
            renderer.update_delta_time(delta_duration);
            renderer.time = time;

            if let Some(ref event) = current_event {
                if time > event.time {
                    log::debug!("aftgraphs::simulation::SimulationContext::run_headless: Handling headless event at time {time}");

                    let mut state = input_values.lock().await;
                    let state = state.as_mut();

                    for (name, val) in &event.inputs {
                        let name = name.replace('_', " ").replace('-', ".");
                        state.insert(name, val.clone());
                    }

                    for event in &event.events {
                        let mut simulation = simulation.lock().await;
                        simulation.on_input(event.clone().into()).await;
                    }

                    current_event = events.next();
                }
            }

            {
                log::debug!(
                    "aftgraphs::simulation::SimulationContext::run_headless: Rendering simulation"
                );

                let mut input_values = input_values.lock().await;
                renderer
                    .render(simulation.clone(), input_values.as_mut())
                    .await;
            }

            if render_imgui {
                log::debug!("aftgraphs::simulation::SimulationContext::run_headless: Drawing ui");

                renderer
                    .draw_ui(None, &inputs, input_values.clone())
                    .await?;
            }

            renderer.render_headless_finish(out_img.as_mut()).await?;
            send_frame.send(out_img.to_owned()).map_err(|e| {
                log::error!("aftgraphs::simulation::SimulationContext::run_headless: Failed to send frame on channel: {e}");
                SRE::HeadlessEncodingError(format!("{e:?}"))
            })?;
            time += delta_t;
        }

        if let Err(e) = finished.send(()) {
            log::warn!("aftgraphs::simulation::SimulationContext::run_headless: error signaling end of frames to encoding thread: {e}");
        }

        if let Err(e) = handle.join() {
            log::error!("aftgraphs::simulation::SimulationContext::run_headless: encoding thread panicked: {e:?}");
            Err(SRE::HeadlessEncodingError(format!("{e:?}")))
        } else {
            Ok(())
        }
    }
}

impl<T: Simulation> SimulationContext<T, UiWinitPlatform> {
    pub async fn run_display(mut self, inputs: Inputs) -> Result<(), SimulationRunError> {
        use SimulationRunError as SRE;

        log::debug!("aftgraphs::simulation::SimulationContext::run_display entered");

        let start_time = Instant::now();
        let simulation = Arc::new(Mutex::new(self.simulation));
        let input_values = InputState::default();
        let mut last_frame = Instant::now();
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

        log::debug!(
            "aftgraphs::simulation::SimulationContext::run_display: Entering winit event_loop"
        );
        let event_loop = self.event_loop.take().ok_or_else(|| {
            log::error!(
                "aftgraphs::simulation::SimulationContext::run_display: {}",
                SRE::DisplayWithoutEventLoop
            );
            SRE::DisplayWithoutEventLoop
        })?;
        event_loop
            .run(move |event, win_target| {
                win_target.set_control_flow(winit::event_loop::ControlFlow::Poll);
                match event {
                    Event::UserEvent(input_event) => {
                        log::debug!("aftgraphs::simulation::SimulationContext::run_display: UserEvent event found on window");

                        let simulation = simulation.clone();
                        block_on(async move {
                            let mut simulation = simulation.lock().await;
                            simulation.on_input(input_event).await;
                        });
                    }
                    Event::WindowEvent {
                        event: WindowEvent::Resized(size),
                        ..
                    } => {
                        log::info!(
                            "aftgraphs::simulation::SimulationContext::run_display: Handling window resize event"
                        );

                        recieved_resize = true;
                        let PhysicalSize { width, height } = size;
                        window_size = PhysicalSize::new(width.into(), height.into());

                        let renderer = self.renderer.clone();
                        let window = self.window.clone();

                        block_on(async move {
                            let mut renderer = renderer.lock().await;

                            if size.width > 0 && size.height > 0 {
                                if let Some(config) = renderer.config.as_mut() {
                                    config.width = size.width;
                                    config.height = size.height;
                                } else {
                                    log::warn!("aftgraphs::simulation::SimulationContext::run_display: Error handling window resize: No surface configuration");
                                    return;
                                }

                                if let (Some(surface), Some(config)) = (renderer.surface.as_ref(), renderer.config.as_ref()){
                                    surface.configure(&renderer.device, config);
                                } else {
                                    log::warn!("aftgraphs::simulation::SimulationContext::run_display: Error handling window resize: No surface");
                                    return;
                                }

                                renderer.aspect_ratio = size.width as f64 / size.height as f64;
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
                        log::info!("aftgraphs::simulation::SimulationContext::run_display: Exit requested");
                        win_target.exit();
                    }
                    winit_event @ Event::WindowEvent {
                        event: WindowEvent::KeyboardInput { .. } , ..
                    } => {
                        log::debug!("aftgraphs::simulation::SimulationContext::run: KeyboardEvent event found on window");

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
                            simulation.on_input(InputEvent::Keyboard(RawKeyEvent { physical_key: event.physical_key, state: event.state })).await;

                            let window = window.lock().await;
                            if let Some(window) = window.as_ref() {
                                let mut renderer = renderer.lock().await;
                                renderer.handle_event(window, &winit_event);
                            }
                        });
                    }
                    winit_event @ Event::WindowEvent { event: WindowEvent::CursorMoved { .. }, .. } => {
                        log::debug!("aftgraphs::simulation::SimulationContext::run: CursorMoved event found on window");

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
                        log::debug!("aftgraphs::simulation::SimulationContext::run_display: MouseInput event found on window");

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
                    winit_event @ Event::WindowEvent {
                        event: WindowEvent::Touch(_), ..
                    } => {
                        log::debug!("aftgraphs::simulation::SimulationContext::run_display: Touch event found on window");

                        let (phase, location) = match &winit_event {
                            Event::WindowEvent { event: WindowEvent::Touch(Touch { phase, location, .. }), .. } => (*phase, *location),
                            _ => unreachable!(),
                        };

                        let state = match phase {
                            TouchPhase::Started => ElementState::Pressed,
                            TouchPhase::Moved => return,
                            TouchPhase::Ended | TouchPhase::Cancelled => ElementState::Released,
                        };
                        let position = (location.x / window_size.width, location.y / window_size.height);
                        let position = (position.0 * 2.0 - 1.0, 1.0 - position.1 * 2.0);

                        let simulation = simulation.clone();
                        let window = self.window.clone();
                        let renderer = self.renderer.clone();

                        block_on(async move {
                            let mut simulation = simulation.lock().await;
                            simulation.on_input(InputEvent::Mouse(state, MouseButton::Left, position)).await;

                            let window = window.lock().await;
                            if let Some(window) = window.as_ref() {
                                let mut renderer = renderer.lock().await;
                                renderer.handle_event(window, &winit_event)
                            }
                        });
                    }
                    Event::NewEvents(_) => {
                        log::debug!(
                            "aftgraphs::simulation::SimulationContext::run_display: New events found on window"
                        );
                        let now = Instant::now();
                        let delta_time = now - last_frame;
                        last_frame = now;

                        let renderer = self.renderer.clone();
                        block_on(async move {
                            let mut renderer = renderer.lock().await;
                            renderer.update_delta_time(delta_time);
                            renderer.time = now.duration_since(start_time).as_secs_f64();
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
                        log::debug!("aftgraphs::simulation::SimulationContext::run_display: window redraw requested");

                        if cfg!(target_arch = "wasm32") && !recieved_resize {
                            return;
                        }

                        let renderer = self.renderer.clone();
                        let window = self.window.clone();
                        let simulation = simulation.clone();
                        let input_values = input_values.clone();
                        let inputs = inputs.clone();

                        block_on(async move {
                            {
                                log::debug!(
                                    "aftgraphs::simulation::SimulationContext::run_display: Rendering simulation"
                                );
                                let mut input_values = input_values.lock().await;
                                renderer
                                    .lock()
                                    .await
                                    .render(simulation.clone(), input_values.as_mut())
                                    .await;
                            }

                            log::debug!("aftgraphs::simulation::SimulationContext::run_display: Updating input values");
                            let mut renderer = renderer.lock().await;
                            let window = window.lock().await;
                            if let Err(e) = renderer.draw_ui(window.as_ref(), &inputs, input_values).await {
                                log::warn!("aftgraphs::simulation::SimulationContext::run_display: {e}");
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
            }).map_err(|e| {
                let e = SRE::DisplayEventLoopFailure(format!("{e:?}"));
                log::error!("aftgraphs::simulation::SimulationContext::run_display: {e}");
                e
            })
    }
}
