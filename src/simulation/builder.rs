use async_mutex::Mutex;
use std::{marker::PhantomData, rc::Rc, sync::Arc};
use winit::{event_loop::EventLoop, window::Window};

use super::Simulation;
use crate::render::Renderer;

mod sealed {
    pub trait Sealed {}
}

pub trait BuilderState: sealed::Sealed {}

pub struct SimulationBuilder<T: Simulation, S: BuilderState> {
    event_loop: Option<EventLoop<()>>,
    window: Arc<Mutex<Option<Window>>>,
    renderer: Option<Rc<Mutex<Renderer>>>,
    headless: bool,
    simulation: PhantomData<T>,
    state: PhantomData<S>,
}

pub struct BuilderInit;
pub struct BuilderComplete;

impl sealed::Sealed for BuilderInit {}
impl sealed::Sealed for BuilderComplete {}

impl<T: sealed::Sealed> BuilderState for T {}

impl<T: Simulation> Default for SimulationBuilder<T, BuilderInit> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Simulation> SimulationBuilder<T, BuilderInit> {
    pub fn new() -> Self {
        Self {
            simulation: PhantomData,
            event_loop: None,
            window: Arc::new(Mutex::new(None)),
            renderer: None,
            headless: false,
            state: PhantomData,
        }
    }

    pub fn event_loop(self, event_loop: EventLoop<()>) -> SimulationBuilder<T, BuilderComplete> {
        SimulationBuilder {
            event_loop: Some(event_loop),
            simulation: self.simulation,
            window: self.window,
            renderer: self.renderer,
            headless: self.headless,
            state: PhantomData,
        }
    }
}

impl<T: Simulation> SimulationBuilder<T, BuilderComplete> {
    pub async fn build(self) -> super::SimulationContext<T> {
        let (window, renderer) = if self.headless {
            let renderer = crate::headless::init((1000, 1000))
                .await
                .expect("SimulationBuilder::build: Failed to init headless mode");
            (self.window, renderer)
        } else {
            log::info!("Building renderer");
            let renderer = {
                let window = self.window.lock().await;
                if let Some(window) = window.as_ref() {
                    crate::display::init(window)
                        .await
                        .expect("SimulationBuilder::build: Failed to init display mode")
                } else {
                    panic!("SimulationBuilder::build: Display mode requires window set in SimulationBuilder")
                }
            };
            (self.window, renderer)
        };

        log::info!("Built simulation");
        unsafe {
            super::SimulationContext {
                simulation: <T as Simulation>::new(&renderer),
                event_loop: self.event_loop.unwrap_unchecked(),
                renderer: Rc::new(Mutex::new(renderer)),
                window,
            }
        }
    }
}

impl<T: Simulation, S: BuilderState> SimulationBuilder<T, S> {
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

    pub fn headless(self, headless: bool) -> Self {
        Self {
            headless,
            simulation: self.simulation,
            window: self.window,
            event_loop: self.event_loop,
            renderer: self.renderer,
            state: PhantomData,
        }
    }
}
