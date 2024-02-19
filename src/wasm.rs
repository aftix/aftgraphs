use crate::input::Inputs;
use crate::simulation::{InputEvent, Simulation, SimulationBuilder, SimulationContext};
use crate::ui::UiWinitPlatform;
use std::future::Future;
use wasm_bindgen::prelude::*;
use web_sys::{Document, DomRect, Event, PointerEvent};
use winit::event::{ElementState, MouseButton};
use winit::event_loop::EventLoopProxy;
use winit::{
    dpi::PhysicalSize,
    event_loop::EventLoopBuilder,
    platform::web::{WindowBuilderExtWebSys, WindowExtWebSys},
    window::WindowBuilder,
};

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

struct ClickEvent {
    client_rect: DomRect,
    state: ElementState,
    screen_x: f64,
    screen_y: f64,
    button: MouseButton,
}

impl ClickEvent {
    fn event_listener(&self) -> anyhow::Result<InputEvent> {
        let (screen_x, screen_y) = (
            self.screen_x - self.client_rect.x(),
            self.screen_y - self.client_rect.y(),
        );

        if self.client_rect.width() <= 1.0 || self.client_rect.height() <= 1.0 {
            anyhow::bail!(
                "aftgraphs::ClickEvent::event_listener: Canvas element #{CANVAS_ID} too small"
            );
        }

        if screen_x >= self.client_rect.width() || screen_y >= self.client_rect.height() {
            anyhow::bail!("aftgraphs::ClickEvent::event_listener: Event happened outside of canvas #{CANVAS_ID}");
        }

        let fraction = (
            screen_x / (self.client_rect.width() - 1.0),
            screen_y / (self.client_rect.width() - 1.0),
        );
        let position = (fraction.0 * 2.0 - 1.0, 1.0 - fraction.1 * 2.0);

        Ok(InputEvent::Mouse(self.state, self.button, position))
    }

    fn from_pointer(
        pointer_event: PointerEvent,
        document: Document,
        is_pointerdown: bool,
    ) -> anyhow::Result<Self> {
        let canvas = document.get_element_by_id(CANVAS_ID).ok_or_else(|| anyhow::anyhow!("aftgraphs::ClientEvent::from_mouse: HTML document dose not have element with id {CANVAS_ID}"))?;
        let client_rect = canvas.get_bounding_client_rect();

        let state = if is_pointerdown {
            ElementState::Pressed
        } else {
            ElementState::Released
        };

        let button = match pointer_event.button() {
            0 => MouseButton::Left,
            1 => MouseButton::Middle,
            2 => MouseButton::Right,
            3 => MouseButton::Back,
            4 => MouseButton::Forward,
            _ => unreachable!(),
        };

        Ok(Self {
            client_rect,
            state,
            screen_x: pointer_event.screen_x() as f64,
            screen_y: pointer_event.screen_y() as f64,
            button,
        })
    }

    fn from_pointerdown(mouse_event: PointerEvent, document: Document) -> anyhow::Result<Self> {
        Self::from_pointer(mouse_event, document, true)
    }

    fn from_pointerup(mouse_event: PointerEvent, document: Document) -> anyhow::Result<Self> {
        Self::from_pointer(mouse_event, document, false)
    }
}

fn event_handler(
    event: Event,
    proxy: &EventLoopProxy<InputEvent>,
    document: Document,
) -> anyhow::Result<()> {
    match event.type_().as_str() {
        "pointerdown" => {
            let event: PointerEvent = event.dyn_into().map_err(|err| anyhow::anyhow!("aftgraphs::event_handler: pointerdown event called without Event being PointerEvent: {err:?}"))?;
            let event = ClickEvent::from_pointerdown(event, document)?;
            let event = event.event_listener()?;
            if proxy.send_event(event).is_err() {
                log::warn!("aftgraphs::event_handler: tried to send event to closed EventLoop");
            }
            Ok(())
        }
        "pointerup" => {
            let event: PointerEvent = event.dyn_into().map_err(|err| anyhow::anyhow!("aftgraphs::event_handler: pointerup event called without Event being PointerEvent: {err:?}"))?;
            let event = ClickEvent::from_pointerup(event, document)?;
            let event = event.event_listener()?;
            if proxy.send_event(event).is_err() {
                log::warn!("aftgraphs::event_handler: tried to send event to closed EventLoop");
            }
            Ok(())
        }
        other_event => {
            log::warn!("aftgraphs::event_handler called with unknown event: {other_event}");
            Ok(())
        }
    }
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

    let event_loop = EventLoopBuilder::<InputEvent>::with_user_event()
        .build()
        .expect("aftgraphs::sim_main: failed to build event loop");

    let proxy = event_loop.create_proxy();

    let window = WindowBuilder::new()
        .with_resizable(false)
        .with_append(true)
        .with_inner_size(PhysicalSize::new(1000, 1000))
        .build(&event_loop)
        .expect("aftgraphs::sim_main: failed to build winit window");

    document.set_title(inputs.simulation.name.as_str());
    let canvas = window.canvas().unwrap();
    canvas.set_id(CANVAS_ID);
    let style = &canvas.style();
    style.set_property("margin", "50px").unwrap();

    let cb = move |e: Event| {
        e.prevent_default();

        let document = document.clone();
        match event_handler(e, &proxy, document) {
            Ok(_) => (),
            Err(err) => log::error!("aftgraphs::event_handler returned error: {err:?}"),
        };
    };
    let cb = Closure::<dyn Fn(_)>::new(cb);

    canvas
        .add_event_listener_with_callback("pointerup pointerdown", cb.as_ref().unchecked_ref())
        .unwrap();

    block_on(async move {
        log::debug!("aftgraphs::sim_main: Building simulation context");
        let context: SimulationContext<T, UiWinitPlatform> = SimulationBuilder::new()
            .window(window)
            .event_loop(event_loop)
            .build()
            .await;

        context.run_display(inputs).await;
    });
}
