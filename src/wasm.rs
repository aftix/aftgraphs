use crate::{
    input::Inputs,
    simulation::{InputEvent, Simulation, SimulationContext},
    ui::UiWinitPlatform,
};
use std::future::Future;
use wasm_bindgen::prelude::*;
use winit::event_loop::{ControlFlow, EventLoop};

pub static CANVAS_ID: &str = "renderTarget";

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

#[wasm_bindgen(module = "/res/common.js")]
extern "C" {
    #[wasm_bindgen(catch, js_name = wait)]
    async fn promise_wait(millis: i32) -> Result<(), JsValue>;

    #[wasm_bindgen(catch, js_name = createWorker)]
    async fn create_worker(memory: &JsValue, ptr: &JsValue) -> Result<JsValue, JsValue>;
}

pub async fn wait(time: f64) {
    let duration = web_time::Duration::from_secs_f64(time);
    promise_wait(duration.as_millis() as i32).await.unwrap();
}

pub type Handle = web_sys::Worker;
pub type SpawnError = JsValue;

pub async fn spawn(f: impl FnOnce() + core::marker::Send + 'static) -> Result<Handle, SpawnError> {
    // Double-box to turn fat pointer into thin pointer
    let ptr = Box::into_raw(Box::new(Box::new(f) as Box<dyn FnOnce()>));

    let worker = create_worker(&wasm_bindgen::memory(), &JsValue::from(ptr as u32)).await?;

    worker.dyn_into()
}

#[wasm_bindgen]
pub fn worker_entry_point(addr: u32) {
    let closure = unsafe { Box::from_raw(addr as *mut Box<dyn FnOnce()>) };
    (*closure)();
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

    let event_loop = EventLoop::<InputEvent>::with_user_event()
        .build()
        .expect("aftgraphs::sim_main: failed to build event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    document.set_title(inputs.simulation.name.as_str());

    block_on(async move {
        log::debug!("aftgraphs::sim_main: running simulation context");
        if let Err(e) = SimulationContext::<T, UiWinitPlatform>::new()
            .run_display(inputs)
            .await
        {
            log::error!("aftgraphs::sim_main: simulation failed: {e}");
            panic!("aftgraphs::sim_main: simulation failed: {e}");
        }
    });
}
