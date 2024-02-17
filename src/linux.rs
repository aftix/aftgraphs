use crate::simulation::InputEvent;
use async_mutex::Mutex;
use core::future::Future;
use std::sync::Arc;
use winit::{event_loop::EventLoopBuilder, window::Window};

use crate::input::Inputs;
use crate::simulation::{Simulation, SimulationBuilder};

fn init_platform() {
    env_logger::init();
}

pub fn block_on<F: Future<Output = ()> + 'static>(fut: F) {
    pollster::block_on(fut);
}

pub fn sim_main<T: Simulation>(inputs: Inputs) {
    init_platform();

    let event_loop = EventLoopBuilder::<InputEvent>::with_user_event()
        .build()
        .expect("failed to build event loop");

    let window = Window::new(&event_loop).unwrap();
    window.set_title(inputs.simulation.name.as_str());

    block_on(async move {
        let context = SimulationBuilder::<T, _>::new()
            .window(window)
            .event_loop(event_loop)
            .build()
            .await;

        let out_img = Arc::new(Mutex::new(vec![]));
        context.run(inputs, out_img).await;
    });
}
