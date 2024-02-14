use crate::input::Inputs;
use crate::simulation::{Simulation, SimulationBuilder};
use async_mutex::Mutex;
use core::future::Future;
use std::sync::Arc;
use winit::{
    dpi::PhysicalSize,
    event_loop::{EventLoop, EventLoopBuilder},
    platform::web::{WindowBuilderExtWebSys, WindowExtWebSys},
    window::WindowBuilder,
};

fn init_platform() {
    use console_error_panic_hook::hook;
    std::panic::set_hook(Box::new(hook));

    if cfg!(debug_assertions) {
        console_log::init_with_level(log::Level::Debug).expect("Failed to initialize console_log");
    } else {
        console_log::init_with_level(log::Level::Info).expect("Failed to initialize console_log");
    }
}

pub fn block_on<F: Future<Output = ()> + 'static>(fut: F) {
    wasm_bindgen_futures::spawn_local(fut);
}

pub fn sim_main<T: Simulation>(inputs: Inputs) {
    init_platform();

    log::debug!("aftgraphs::sim_main entered");

    let html_window = web_sys::window().expect("aftgraphs::sim_main: no global `window` exists");
    let document = html_window
        .document()
        .expect("aftgraphs::sim_main: should have a document on window");
    let _body = document
        .body()
        .expect("aftgraphs::sim_main: document should have a body");

    let event_loop: EventLoop<()> = EventLoopBuilder::default()
        .build()
        .expect("aftgraphs::sim_main: failed to build event loop");

    let window = WindowBuilder::new()
        .with_resizable(false)
        .with_append(true)
        .with_inner_size(PhysicalSize::new(1000, 1000))
        .build(&event_loop)
        .expect("aftgraphs::sim_main: failed to build winit window");

    let canvas = window.canvas().unwrap();
    let style = &canvas.style();
    style.set_property("margin", "50px").unwrap();

    document.set_title(inputs.simulation.name.as_str());

    block_on(async move {
        log::debug!("aftgraphs::sim_main: Building simulation context");
        let context = SimulationBuilder::<T, _>::new()
            .window(window)
            .event_loop(event_loop)
            .build()
            .await;

        let out_img = Arc::new(Mutex::new(vec![]));
        context.run(inputs, out_img).await;
    });
}
