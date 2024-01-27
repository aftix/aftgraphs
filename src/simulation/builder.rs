use async_mutex::Mutex;
use std::{marker::PhantomData, sync::Arc};
use winit::{event_loop::EventLoop, window::Window};

use super::Simulation;
use crate::render::Renderer;

mod sealed {
    pub trait Sealed {
        type AddLoop: Sealed;
        type AddShader: Sealed;
    }
}

pub trait BuilderState: sealed::Sealed {
    type AddLoop: sealed::Sealed;
    type AddShader: sealed::Sealed;
}

pub struct SimulationBuilder<'a, T: Simulation, S: BuilderState> {
    event_loop: Option<EventLoop<()>>,
    window: Option<Window>,
    shader: Option<&'a str>,
    renderer: Option<Arc<Mutex<Renderer>>>,
    headless: bool,
    simulation: T,
    state: PhantomData<S>,
}

pub struct BuilderInit;

pub struct BuilderLoop;
pub struct BuilderShader;

pub struct BuilderComplete;

impl sealed::Sealed for BuilderInit {
    type AddLoop = BuilderLoop;
    type AddShader = BuilderShader;
}
impl sealed::Sealed for BuilderLoop {
    type AddLoop = BuilderLoop;
    type AddShader = BuilderComplete;
}
impl sealed::Sealed for BuilderShader {
    type AddLoop = BuilderComplete;
    type AddShader = BuilderShader;
}
impl sealed::Sealed for BuilderComplete {
    type AddLoop = BuilderComplete;
    type AddShader = BuilderComplete;
}

impl<T: sealed::Sealed> BuilderState for T {
    type AddLoop = T::AddLoop;
    type AddShader = T::AddShader;
}

impl<'a, T: Simulation> SimulationBuilder<'a, T, BuilderInit> {
    pub fn new(simulation: T) -> Self {
        Self {
            simulation,
            event_loop: None,
            window: None,
            shader: None,
            renderer: None,
            headless: false,
            state: PhantomData::default(),
        }
    }
}

impl<'a, T: Simulation> SimulationBuilder<'a, T, BuilderComplete> {
    pub async fn build(self) -> super::SimulationContext<T> {
        let shader = unsafe { self.shader.unwrap_unchecked() };
        let (window, renderer) = if self.headless {
            let renderer = crate::headless::init((1000, 1000), shader)
                .await
                .expect("SimulationBuilder::build: Failed to init headless mode");
            (None, renderer)
        } else {
            let window = self
                .window
                .expect("Display mode requires window set in SimulationBuilder");
            let renderer = crate::display::init(&window, shader)
                .await
                .expect("SimulationBuilder::build: Failed to init display mode");
            (Some(window), renderer)
        };

        unsafe {
            super::SimulationContext {
                simulation: self.simulation,
                event_loop: self.event_loop.unwrap_unchecked(),
                renderer: Arc::new(Mutex::new(renderer)),
                window,
            }
        }
    }
}

impl<'a, T: Simulation, S: BuilderState> SimulationBuilder<'a, T, S> {
    pub fn event_loop(
        self,
        event_loop: EventLoop<()>,
    ) -> SimulationBuilder<'a, T, <S as BuilderState>::AddLoop> {
        SimulationBuilder {
            event_loop: Some(event_loop),
            simulation: self.simulation,
            window: self.window,
            shader: self.shader,
            renderer: self.renderer,
            headless: self.headless,
            state: PhantomData::default(),
        }
    }

    pub fn window(self, window: Window) -> Self {
        Self {
            window: Some(window),
            simulation: self.simulation,
            event_loop: self.event_loop,
            shader: self.shader,
            renderer: self.renderer,
            headless: self.headless,
            state: PhantomData::default(),
        }
    }

    pub fn headless(self, headless: bool) -> Self {
        Self {
            headless,
            simulation: self.simulation,
            window: self.window,
            event_loop: self.event_loop,
            shader: self.shader,
            renderer: self.renderer,
            state: PhantomData::default(),
        }
    }

    pub fn shader(
        self,
        shader: &'a str,
    ) -> SimulationBuilder<'a, T, <S as BuilderState>::AddShader> {
        SimulationBuilder {
            shader: Some(shader),
            simulation: self.simulation,
            window: self.window,
            event_loop: self.event_loop,
            renderer: self.renderer,
            headless: self.headless,
            state: PhantomData::default(),
        }
    }
}
