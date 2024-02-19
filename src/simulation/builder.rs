use super::{InputEvent, Simulation};
use crate::ui::UiPlatform;
use crate::{render::Renderer, ui::UiWinitPlatform};
use async_mutex::Mutex;
use std::{marker::PhantomData, rc::Rc, sync::Arc};
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

impl<T: Simulation> SimulationBuilder<T, (), BuilderComplete> {
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn build_headless(self) -> anyhow::Result<super::SimulationContext<T, ()>> {
        log::info!("aftgraphs::simulation::SimulationBuilder: Building headless renderer");

        let size = self.headless.ok_or_else(|| {
            log::error!("aftgraphs::simulation::SimulationBuilder::build_headless: building headless renderer in display mode");
            anyhow::anyhow!("aftgraphs::simulation::SimulationBuilder::build_headless: building headless renderer in display mode")
        })?;

        let renderer = crate::headless::init(size).await.map_err(|e| {
            anyhow::anyhow!("aftgraphs::simulation::SimulationBuilder::build_headless: {e}")
        })?;

        if self.event_loop.is_some() {
            log::error!("aftgraphs::simulation::SimulationContext::build_headless: building headless renderer with an event loop");
            anyhow::bail!("aftgraphs::simulation::SimulationContext::build_headless: building headless renderer with an event loop");
        }

        Ok(super::SimulationContext {
            simulation: <T as Simulation>::new(&renderer),
            event_loop: self.event_loop,
            renderer: Rc::new(Mutex::new(renderer)),
            window: self.window,
        })
    }
}

impl<T: Simulation> SimulationBuilder<T, UiWinitPlatform, BuilderComplete> {
    pub async fn build(self) -> anyhow::Result<super::SimulationContext<T, UiWinitPlatform>> {
        log::info!("aftgraphs::simulation::SimulationBuilder: Building display renderer");

        let renderer = {
            let window = self.window.lock().await;
            let window = window.as_ref().ok_or_else(|| {
                log::error!("aftgraphs::simulation::SimulationBuilder::build: bulding display renderer without a window");
                anyhow::anyhow!("aftgraphs::simulation::SimulationBuilder::build: bulding display renderer without a window")
            })?;

            crate::display::init(window).await.map_err(|e| {
                anyhow::anyhow!("aftgraphs::simulation::SimulationBuilder::build: {e}")
            })?
        };

        if self.event_loop.is_none() {
            log::error!("aftgraphs::simulation::SimulationContext::build: building display renderer without an event loop");
            anyhow::bail!("aftgraphs::simulation::SimulationContext::build: building display renderer without an event loop");
        }

        Ok(super::SimulationContext {
            simulation: <T as Simulation>::new(&renderer),
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
