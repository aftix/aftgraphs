use async_mutex::Mutex;
use core::future::Future;
use std::sync::Arc;
use winit::{
    event_loop::{EventLoop, EventLoopBuilder},
    window::Window,
};

use crate::input::Inputs;
use crate::simulation::{Simulation, SimulationBuilder};

fn init_platform() {
    env_logger::init();
}

pub fn block_on<F: Future<Output = ()> + 'static>(fut: F) {
    pollster::block_on(fut);
}

pub fn sim_main<T: Simulation>(
    shader: wgpu::ShaderModuleDescriptor<'static>,
    inputs: Inputs,
    simulation: T,
) {
    init_platform();

    let event_loop: EventLoop<()> = EventLoopBuilder::default()
        .build()
        .expect("failed to build event loop");

    let window = Window::new(&event_loop).unwrap();

    block_on(async move {
        let context = SimulationBuilder::new(simulation)
            .window(window)
            .event_loop(event_loop)
            .shader(shader)
            .build()
            .await;

        let out_img = Arc::new(Mutex::new(vec![]));
        context.run(inputs, out_img).await;
    });
}
