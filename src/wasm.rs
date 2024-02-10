use async_mutex::Mutex;
use core::future::Future;
use std::sync::Arc;
use winit::{
    dpi::PhysicalSize,
    event_loop::{EventLoop, EventLoopBuilder},
    platform::web::WindowBuilderExtWebSys,
    window::WindowBuilder,
};

use crate::input::Inputs;
use crate::simulation::{Simulation, SimulationBuilder};

fn init_platform() {
    console_log::init().expect("Failed to initialize console_log");
}

pub fn block_on<F: Future<Output = ()> + 'static>(fut: F) {
    wasm_bindgen_futures::spawn_local(fut);
}

pub fn sim_main<T: Simulation>(shader: &'static str, _inputs: Inputs, simulation: T) {
    init_platform();

    let html_window = web_sys::window().expect("no global `window` exists");
    let document = html_window
        .document()
        .expect("should have a document on window");
    let _body = document.body().expect("document should have a body");

    let event_loop: EventLoop<()> = EventLoopBuilder::default()
        .build()
        .expect("failed to build event loop");

    let window = WindowBuilder::new()
        .with_resizable(false)
        .with_append(true)
        .with_inner_size(PhysicalSize::new(1000, 1000))
        .build(&event_loop)
        .expect("failed to build winit window");

    block_on(async move {
        let context = SimulationBuilder::new(simulation)
            .window(window)
            .event_loop(event_loop)
            .shader(shader)
            .build()
            .await;

        let out_img = Arc::new(Mutex::new(vec![]));
        context.run(out_img).await;
    });
}
