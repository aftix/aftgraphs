use crate::{
    input::{InputValue, Inputs},
    render::{RenderError, Renderer},
    ui::{UiPlatform, UiWinitPlatform},
    GraphicsInitError,
};
use async_std::sync::Mutex;
use std::{collections::HashMap, marker::PhantomData, sync::Arc};
use thiserror::Error;
pub use winit::event::{ElementState, MouseButton, RawKeyEvent};
use winit::{
    error::EventLoopError,
    event_loop::{ControlFlow, EventLoop},
};

#[derive(Clone)]
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
    #[allow(dead_code)]
    size: Option<(u32, u32)>,
    _simulation: PhantomData<T>,
    _platform: PhantomData<P>,
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "x264")]
mod encoder;

#[derive(Error, Debug)]
pub enum SimulationRunError {
    #[error("ran headless rendering on a binary not compiled with the 'x264' feature")]
    HeadlessWithoutx264,
    #[error("headless rendering used without an initialized draw target texture")]
    HeadlessWithoutTexture,
    #[error("headless rendering used without an output file")]
    HeadlessWithoutOutputFile,
    #[error("headless rendering used without an output size")]
    HeadlessWithoutSize,
    #[error("headless video encoding failed: {0}")]
    HeadlessEncodingError(String),
    #[error("display rendering used without a winit::event::EventLoop")]
    DisplayWithoutEventLoop,
    #[error("display rendering used without a winit::window::Window")]
    DisplayWithoutWindow,
    #[error("display rendering winit::event::EventLoop ended unexpectedly: {0}")]
    DisplayEventLoopFailure(#[from] EventLoopError),
    #[error("rendering failed: {0}")]
    RenderFailure(#[from] RenderError),
    #[error("Initializing graphics failed: {0}")]
    GraphicsInitFailure(#[from] GraphicsInitError),
}

impl<T: Simulation> SimulationContext<T, ()> {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new_headless(size: (u32, u32)) -> Self {
        log::info!("aftgraphs::simulation: Building headless renderer");
        Self {
            size: Some(size),
            _simulation: PhantomData,
            _platform: PhantomData,
        }
    }

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
        use crate::{cli::ARGUMENTS, headless::HeadlessMetadata, input::InputState};
        use web_time::Duration;
        use SimulationRunError as SRE;

        log::debug!("aftgraphs::simulation::SimulationContext::run_headless entered");

        let size = self.size.ok_or(SRE::HeadlessWithoutSize)?;
        let mut renderer = crate::headless::init(size)
            .await
            .map_err(Into::<SRE>::into)?;

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

        let simulation = Arc::new(Mutex::new(T::new(&renderer).await));

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

impl<T: Simulation> Default for SimulationContext<T, UiWinitPlatform> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Simulation> SimulationContext<T, UiWinitPlatform> {
    pub fn new() -> Self {
        log::info!("aftgraphs::simulation: Building display renderer");
        Self {
            size: None,
            _simulation: PhantomData,
            _platform: PhantomData,
        }
    }

    pub async fn run_display(self, inputs: Inputs) -> Result<(), SimulationRunError> {
        log::debug!("aftgraphs::simulation::SimulationContext::run_display entered");
        log::debug!(
            "aftgraphs::simulation::SimulationContext::run_display: Entering winit event_loop"
        );

        let event_loop = EventLoop::with_user_event().build().map_err(|err| {
            log::error!(
                "aftgraphs::simulation::SimulationContext::run_display: {}",
                err,
            );
            Into::<SimulationRunError>::into(err)
        })?;

        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop
            .run_app(&mut crate::App::<T>::new(inputs))
            .map_err(|err| {
                log::error!(
                    "aftgraphs::simulation::SimulationContext::run_display: {}",
                    err,
                );
                err.into()
            })
    }
}
