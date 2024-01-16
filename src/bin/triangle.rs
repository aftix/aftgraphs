use winit::{
    event::{Event, WindowEvent},
    event_loop::{EventLoop, EventLoopBuilder},
    window::Window,
};

use aftgraphs::display::init;

async fn run(event_loop: EventLoop<()>, window: Window) {
    let mut renderer = init(&window, include_str!("triangle.wgsl")).await.unwrap();

    event_loop
        .run(move |event, win_target| match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                renderer.config.as_mut().unwrap().width = size.width;
                renderer.config.as_mut().unwrap().height = size.height;
                renderer
                    .surface
                    .as_ref()
                    .unwrap()
                    .configure(&renderer.device, renderer.config.as_ref().unwrap());
                window.request_redraw();
            }
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                ..
            } => {
                pollster::block_on(renderer.render(0..3, 0..1, None));
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                win_target.exit();
            }
            _ => (),
        })
        .unwrap();
}

fn main() {
    let event_loop: EventLoop<()> = EventLoopBuilder::default()
        .build()
        .expect("failed to build event loop");

    let window = Window::new(&event_loop).unwrap();
    env_logger::init();
    pollster::block_on(run(event_loop, window));
}
