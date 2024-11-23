use crate::{
    input::{InputState, Inputs},
    prelude::InputEvent,
    render::Renderer,
    simulation::Simulation,
    ui::{UiPlatform, UiWinitPlatform},
};
use async_std::sync::Mutex;
use crossbeam::channel::bounded;
use std::{rc::Rc, sync::Arc};
use web_time::Instant;
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::{
        ElementState, Event, KeyEvent, MouseButton, RawKeyEvent, StartCause, Touch, TouchPhase,
        WindowEvent,
    },
    event_loop::ActiveEventLoop,
    keyboard::{Key, NamedKey},
    window::{Window, WindowAttributes, WindowId},
};

#[cfg(not(target_arch = "wasm32"))]
use crate::linux::block_on;
#[cfg(target_arch = "wasm32")]
use crate::wasm::block_on;

struct AppWindow<P: UiPlatform> {
    window: Arc<Window>,
    renderer: Renderer<'static, P>,
}

type AsyncWindow<P> = Rc<Mutex<AppWindow<P>>>;

struct AppData {
    cursor_position: PhysicalPosition<f64>,
    inputs: Inputs,
    input_values: InputState,
    last_frame: Instant,
    recieved_resize: bool,
    start_time: Instant,
    window_size: PhysicalSize<f64>,
}

impl AppData {
    fn new(inputs: Inputs) -> Self {
        let now = Instant::now();
        Self {
            cursor_position: PhysicalPosition::new(0.0, 0.0),
            last_frame: now,
            inputs,
            input_values: InputState::default(),
            recieved_resize: false,
            start_time: now,
            window_size: PhysicalSize::new(0.0, 0.0),
        }
    }
}

// Lock in alphabetical order, except simulation must be last
pub struct App<T: Simulation> {
    simulation: Option<Arc<Mutex<T>>>,
    data: Arc<Mutex<AppData>>,
    window: Option<AsyncWindow<UiWinitPlatform>>,
}

impl<T: Simulation> App<T> {
    pub fn new(inputs: Inputs) -> Self {
        Self {
            simulation: None,
            data: Arc::new(Mutex::new(AppData::new(inputs))),
            window: None,
        }
    }

    async fn on_resumed(
        window: Window,
        data: &mut AppData,
    ) -> (AsyncWindow<UiWinitPlatform>, Arc<Mutex<T>>) {
        let window = Arc::new(window);

        window.set_title(data.inputs.simulation.name.as_str());

        let PhysicalSize { width, height } = window.inner_size();
        data.window_size = PhysicalSize::new(width.into(), height.into());
        let renderer = crate::display::init(window.clone())
            .await
            .expect("failed to create renderer");

        let simulation = Arc::new(Mutex::new(T::new(&renderer).await));
        (
            Rc::new(Mutex::new(AppWindow { window, renderer })),
            simulation,
        )
    }

    async fn on_window_event(
        window_id: WindowId,
        event: WindowEvent,
        app_window: &mut AppWindow<UiWinitPlatform>,
        simulation: Arc<Mutex<T>>,
        data: &mut AppData,
    ) -> bool {
        match event.clone() {
            WindowEvent::RedrawRequested => {
                log::debug!("aftgraphs::app::App::on_window_event: window redraw requested");

                if cfg!(target_arch = "wasm32") && !data.recieved_resize {
                    return false;
                }

                {
                    log::debug!("aftgraphs::app::App::on_window_event: Rendering simulation");
                    let mut input_values = data.input_values.lock().await;
                    app_window
                        .renderer
                        .render(simulation, input_values.as_mut())
                        .await;
                }

                log::debug!("aftgraphs::app::App::on_window_event: Updating input values");
                if let Err(e) = app_window
                    .renderer
                    .draw_ui(
                        Some(&app_window.window),
                        &data.inputs,
                        data.input_values.clone(),
                    )
                    .await
                {
                    log::warn!("aftgraphs::app::App::on_window_event: {e}");
                }
            }
            WindowEvent::Resized(PhysicalSize { width, height }) => {
                log::info!("aftgraphs::app::App::on_window_event: Handling window resize event");

                data.recieved_resize = true;
                data.window_size = PhysicalSize::new(width.into(), height.into());

                if width > 0 && height > 0 {
                    if let Some(config) = app_window.renderer.config.as_mut() {
                        config.width = width;
                        config.height = height;
                    } else {
                        log::warn!("aftgraphs::app::App::on_window_event: Error handling window resize: No surface configuration");
                        return false;
                    }

                    if let (Some(surface), Some(config)) = (
                        app_window.renderer.surface.as_ref(),
                        app_window.renderer.config.as_ref(),
                    ) {
                        surface.configure(&app_window.renderer.device, config);
                    } else {
                        log::warn!("aftgraphs::app::App::on_window_event: Error handling window resize: No surface");
                        return false;
                    }

                    app_window.renderer.aspect_ratio = width as f64 / height as f64;
                }

                app_window.window.request_redraw();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: Key::Named(NamedKey::Escape),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            }
            | WindowEvent::CloseRequested => {
                log::info!("aftgraphs::app::App::on_window_event: Exit requested");
                return true;
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key,
                        state,
                        ..
                    },
                ..
            } => {
                log::debug!("aftgraphs::app::App::run: KeyboardEvent event found on window");

                simulation
                    .lock()
                    .await
                    .on_input(InputEvent::Keyboard(RawKeyEvent {
                        physical_key,
                        state,
                    }))
                    .await;
            }
            WindowEvent::CursorMoved { position, .. } => {
                log::debug!(
                    "aftgraphs::app::App::on_window_event: CursorMoved event found on window"
                );
                data.cursor_position = position;
            }
            WindowEvent::MouseInput { state, button, .. } => {
                log::debug!(
                    "aftgraphs::app::App::on_window_event: MouseInput event found on window"
                );

                // Convert mouse coordinates to screen space
                let position = (
                    data.cursor_position.x / data.window_size.width,
                    data.cursor_position.y / data.window_size.height,
                );
                let position = (position.0 * 2.0 - 1.0, 1.0 - position.1 * 2.0);

                simulation
                    .lock()
                    .await
                    .on_input(InputEvent::Mouse(state, button, position))
                    .await;
            }
            WindowEvent::Touch(Touch {
                phase, location, ..
            }) => {
                log::debug!("aftgraphs::app::App::on_window_event: Touch event found on window");

                let state = match phase {
                    TouchPhase::Started => ElementState::Pressed,
                    TouchPhase::Moved => return false,
                    TouchPhase::Ended | TouchPhase::Cancelled => ElementState::Released,
                };

                let position = (
                    location.x / data.window_size.width,
                    location.y / data.window_size.height,
                );

                let position = (position.0 * 2.0 - 1.0, 1.0 - position.1 * 2.0);

                simulation
                    .lock()
                    .await
                    .on_input(InputEvent::Mouse(state, MouseButton::Left, position))
                    .await;
            }
            _ => (),
        }

        app_window.renderer.handle_event(
            &app_window.window,
            &Event::<InputEvent>::WindowEvent { window_id, event },
        );

        false
    }
}

#[cfg(target_arch = "wasm32")]
fn make_window_attributes() -> WindowAttributes {
    use winit::platform::web::WindowAttributesExtWebSys;
    Window::default_attributes()
        .with_resizable(false)
        .with_inner_size(PhysicalSize::new(1000, 1000))
        .with_append(true)
}

#[cfg(not(target_arch = "wasm32"))]
fn make_window_attributes() -> WindowAttributes {
    Window::default_attributes().with_resizable(false)
}

impl<T: Simulation> ApplicationHandler<InputEvent> for App<T> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attributes = make_window_attributes();
        let window = event_loop
            .create_window(attributes)
            .expect("Failed to create winit window");
        let data = self.data.clone();

        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowExtWebSys;
            let canvas = window.canvas().expect("Failed to get window canvas");
            canvas.set_id(crate::CANVAS_ID);
            let style = &canvas.style();
            style
                .set_property("margin", "50px")
                .expect("Failed to set canvas style");
        }

        let (send, recv) = bounded(1);
        block_on(async move {
            let mut data = data.lock().await;

            let app_window = Self::on_resumed(window, &mut data).await;
            send.send(app_window).expect("Failed to send AppWindow");
        });

        let (app_window, simulation) = recv.recv().expect("Failed to recieve AppWindow");
        self.window = Some(app_window);
        self.simulation = Some(simulation);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let Some(app_window) = self.window.as_ref().map(Clone::clone) else {
            return;
        };
        let data = self.data.clone();
        let simulation = self.simulation.as_ref().unwrap().clone();

        let (send, recv) = bounded(1);
        block_on(async move {
            let mut app_window = app_window.lock().await;
            let mut data = data.lock().await;

            let exit =
                Self::on_window_event(window_id, event, &mut app_window, simulation, &mut data)
                    .await;

            send.send(exit).expect("Failed to send window_event result");
        });

        if recv.recv().expect("Failed to recieve window_event result") {
            log::info!("aftgraphs::app::App::window_event: Exiting application");
            event_loop.exit();
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: InputEvent) {
        log::debug!("aftgraphs::app::App::user_event: UserEvent event found on window");
        let Some(app_window) = self.window.as_ref().map(Clone::clone) else {
            return;
        };
        let simulation = self.simulation.as_ref().unwrap().clone();

        block_on(async move {
            let mut app_window = app_window.lock().await;
            let AppWindow { window, renderer } = &mut *app_window;

            simulation.lock().await.on_input(event.clone()).await;
            renderer.handle_event(window, &Event::UserEvent(event));
        });
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        log::debug!("aftgraphs::app::App::device_event: DeviceEvent event found on window");
        let Some(app_window) = self.window.as_ref().map(Clone::clone) else {
            return;
        };

        block_on(async move {
            let mut app_window = app_window.lock().await;
            let AppWindow { window, renderer } = &mut *app_window;

            renderer.handle_event(
                window,
                &Event::<InputEvent>::DeviceEvent { device_id, event },
            );
        });
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        log::debug!("aftgraphs::app::App::about_to_wait: Window about to wait");
        let Some(app_window) = self.window.as_ref().map(Clone::clone) else {
            return;
        };

        block_on(async move {
            let mut app_window = app_window.lock().await;
            let AppWindow { window, renderer } = &mut *app_window;
            renderer.prepare_ui(window).await;
            renderer.handle_event(window, &Event::<InputEvent>::AboutToWait);
            app_window.window.request_redraw();
        });
    }

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: StartCause) {
        log::debug!("aftgraphs::app::App::new_events: New events found on window");
        let Some(app_window) = self.window.as_ref().map(Clone::clone) else {
            return;
        };
        let data = self.data.clone();

        block_on(async move {
            let mut app_window = app_window.lock().await;
            let mut data = data.lock().await;

            let now = Instant::now();
            let delta_time = now - data.last_frame;
            data.last_frame = now;

            app_window.renderer.update_delta_time(delta_time);
            app_window.renderer.time = now.duration_since(data.start_time).as_secs_f64();
        });
    }
}
