use crate::cli::{parse_cli, ARGUMENTS};
use crate::headless::HeadlessInput;
use crate::input::Inputs;
use crate::simulation::InputEvent;
use crate::simulation::{Simulation, SimulationBuilder, SimulationContext};
use crate::ui::UiWinitPlatform;
use async_mutex::Mutex;
use core::future::Future;
use std::fs::File;
use std::io::read_to_string;
use std::sync::Arc;
use winit::{event_loop::EventLoopBuilder, window::Window};

fn init_platform() {
    env_logger::init();
}

pub fn block_on<F: Future<Output = ()> + 'static>(fut: F) {
    pollster::block_on(fut);
}

pub fn sim_main<T: Simulation>(inputs: Inputs) {
    init_platform();

    parse_cli(
        inputs.simulation.name.as_str(),
        inputs.simulation.description.as_deref(),
        inputs.simulation.author.as_deref(),
    );

    block_on(async move {
        let is_headless = {
            let args = ARGUMENTS.read().await;
            args.headless.clone().map(|args| (args.in_file, args.size))
        };
        if let Some((in_file, arg_size)) = is_headless {
            let input_file = File::open(in_file).expect("Failed to open headless input file");
            let input_file =
                read_to_string(input_file).expect("Failed to read headless input file");
            let headless_input: HeadlessInput = toml::from_str(input_file.as_str())
                .expect("Failed to parse headless input file TOML");

            let mut size = (
                arg_size.0.unwrap_or_else(|| {
                    headless_input
                        .simulation
                        .size
                        .map(|size| size[0])
                        .unwrap_or(1000)
                }),
                arg_size.1.unwrap_or_else(|| {
                    headless_input
                        .simulation
                        .size
                        .map(|size| size[1])
                        .unwrap_or(1000)
                }),
            );

            size.0 = size.0.max(4);
            size.1 = size.1.max(4);

            let context: SimulationContext<T, ()> = SimulationBuilder::new()
                .headless(size)
                .build_headless()
                .await;

            let out_img = Arc::new(Mutex::new(vec![]));
            context.run_headless(inputs, headless_input, out_img).await;
        } else {
            let event_loop = EventLoopBuilder::<InputEvent>::with_user_event()
                .build()
                .expect("failed to build event loop");

            let window = Window::new(&event_loop).unwrap();
            window.set_title(inputs.simulation.name.as_str());

            let context: SimulationContext<T, UiWinitPlatform> = SimulationBuilder::new()
                .window(window)
                .event_loop(event_loop)
                .build()
                .await;
            context.run_display(inputs).await;
        };
    });
}
