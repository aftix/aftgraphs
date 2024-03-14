use super::{InputEvent, Simulation};
use crate::{
    render::Renderer,
    ui::{UiPlatform, UiWinitPlatform},
    GraphicsInitError,
};
use async_std::sync::Mutex;
use std::{marker::PhantomData, rc::Rc, sync::Arc};
use thiserror::Error;
use winit::{event_loop::EventLoop, window::Window};

mod sealed {
    pub trait Sealed {}
}

pub trait BuilderState: sealed::Sealed {}

pub struct SimulationBuilder<T: Simulation, P: UiPlatform, S: BuilderState> {
    event_loop: Option<EventLoop<InputEvent>>,
    window: Arc<Mutex<Option<Window>>>,
    renderer: Option<Rc<Mutex<Renderer<P>>>>,
    headless: Option<(u32, u32)>,
    simulation: PhantomData<T>,
    state: PhantomData<S>,
}

pub struct BuilderInit;
pub struct BuilderComplete;

impl sealed::Sealed for BuilderInit {}
impl sealed::Sealed for BuilderComplete {}

impl<T: sealed::Sealed> BuilderState for T {}

impl<T: Simulation, P: UiPlatform> Default for SimulationBuilder<T, P, BuilderInit> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Simulation, P: UiPlatform> SimulationBuilder<T, P, BuilderInit> {
    pub fn new() -> Self {
        Self {
            simulation: PhantomData,
            event_loop: None,
            window: Arc::new(Mutex::new(None)),
            renderer: None,
            headless: None,
            state: PhantomData,
        }
    }

    pub fn event_loop(
        self,
        event_loop: EventLoop<InputEvent>,
    ) -> SimulationBuilder<T, P, BuilderComplete> {
        SimulationBuilder {
            event_loop: Some(event_loop),
            simulation: self.simulation,
            window: self.window,
            renderer: self.renderer,
            headless: self.headless,
            state: PhantomData,
        }
    }

    /// Build a simulation with headless mode
    pub fn headless(self, size: (u32, u32)) -> SimulationBuilder<T, P, BuilderComplete> {
        SimulationBuilder {
            headless: Some(size),
            simulation: self.simulation,
            window: self.window,
            event_loop: self.event_loop,
            renderer: self.renderer,
            state: PhantomData,
        }
    }
}

#[derive(Error, Debug, Clone)]
pub enum SimulationBuilderError {
    #[error("building simulation for headless rendering while using display rendering builder")]
    HeadlessInDisplayMode,
    #[error("building simulation for display rendering while using headless rendering builder")]
    DisplayInHeadlessMode,
    #[error("failed to initialize headless renderer: {0}")]
    HeadlessInitFailed(#[from] GraphicsInitError),
    #[error(
        "building simulation for headless rendering after calling SimulationBuilder::event_loop"
    )]
    HeadlessWithEventLoop,
    #[error("building simulation for display rendering without calling Simulation::event_loop")]
    DisplayWithoutEventLoop,
}

impl<T: Simulation> SimulationBuilder<T, (), BuilderComplete> {
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn build_headless(
        self,
    ) -> Result<super::SimulationContext<T, ()>, SimulationBuilderError> {
        use SimulationBuilderError as SBE;

        log::info!("aftgraphs::simulation::SimulationBuilder: Building headless renderer");

        let size = self.headless.ok_or_else(|| {
            log::error!(
                "aftgraphs::simulation::SimulationBuilder::build_headless: {:?}",
                SBE::HeadlessInDisplayMode
            );
            SBE::HeadlessInDisplayMode
        })?;

        let renderer = crate::headless::init(size).await?;

        if self.event_loop.is_some() {
            log::error!(
                "aftgraphs::simulation::SimulationContext::build_headless: {:?}",
                SBE::HeadlessWithEventLoop
            );
            return Err(SBE::HeadlessWithEventLoop);
        }

        Ok(super::SimulationContext {
            simulation: <T as Simulation>::new(&renderer).await,
            event_loop: self.event_loop,
            renderer: Rc::new(Mutex::new(renderer)),
            window: self.window,
        })
    }
}

impl<T: Simulation> SimulationBuilder<T, UiWinitPlatform, BuilderComplete> {
    pub async fn build(
        self,
    ) -> Result<super::SimulationContext<T, UiWinitPlatform>, SimulationBuilderError> {
        use SimulationBuilderError as SBE;

        log::info!("aftgraphs::simulation::SimulationBuilder: Building display renderer");

        let renderer = {
            let window = self.window.lock().await;
            let window = window.as_ref().ok_or_else(|| {
                log::error!(
                    "aftgraphs::simulation::SimulationBuilder::build: {}",
                    SBE::DisplayInHeadlessMode
                );
                SBE::DisplayInHeadlessMode
            })?;

            crate::display::init(window).await?
        };

        if self.event_loop.is_none() {
            log::error!(
                "aftgraphs::simulation::SimulationContext::build: {}",
                SBE::DisplayWithoutEventLoop
            );
            return Err(SBE::DisplayWithoutEventLoop);
        }

        Ok(super::SimulationContext {
            simulation: <T as Simulation>::new(&renderer).await,
            event_loop: self.event_loop,
            renderer: Rc::new(Mutex::new(renderer)),
            window: self.window,
        })
    }
}

impl<T: Simulation, P: UiPlatform, S: BuilderState> SimulationBuilder<T, P, S> {
    pub fn window(self, window: Window) -> Self {
        Self {
            window: Arc::new(Mutex::new(Some(window))),
            simulation: self.simulation,
            event_loop: self.event_loop,
            renderer: self.renderer,
            headless: self.headless,
            state: PhantomData,
        }
    }
}
