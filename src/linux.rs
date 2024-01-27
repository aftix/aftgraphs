use core::future::Future;

use winit::{
    event_loop::{EventLoop, EventLoopBuilder},
    window::Window,
};

use crate::simulation::SimulationBuilder;

fn init_platform() {
    env_logger::init();
}

pub fn block_on<F: Future<Output = ()> + 'static>(fut: F) {
    pollster::block_on(fut);
}

pub fn sim_main(shader: &'static str) {
    init_platform();

    let event_loop: EventLoop<()> = EventLoopBuilder::default()
        .build()
        .expect("failed to build event loop");

    let window = Window::new(&event_loop).unwrap();

    block_on(async move {
        let context = SimulationBuilder::new()
            .window(window)
            .event_loop(event_loop)
            .shader(shader)
            .build()
            .await;

        context.run().await;
    });
}
