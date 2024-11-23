use crate::cli::{parse_cli, ARGUMENTS};
use crate::headless::HeadlessInput;
use crate::input::Inputs;
use crate::simulation::{Simulation, SimulationContext};
use crate::ui::UiWinitPlatform;
use async_std::{
    future::{pending, timeout},
    sync::Mutex,
};
use std::{fs::File, future::Future, io::read_to_string, sync::Arc, time::Duration};

fn init_platform() {
    env_logger::init();
}

pub fn block_on<F: Future<Output = ()> + 'static>(fut: F) {
    pollster::block_on(fut);
}

pub async fn wait(time: f64) {
    let duration = Duration::from_secs_f64(time);
    timeout(duration, pending::<()>()).await.err();
}

pub type Handle = std::thread::JoinHandle<()>;
pub type SpawnError = ();

pub async fn spawn(f: impl FnOnce() + Send + 'static) -> Result<Handle, SpawnError> {
    let handle = std::thread::spawn(f);
    Ok(handle)
}

pub fn sim_main<T: Simulation>(inputs: Inputs) {
    init_platform();

    parse_cli(
        inputs.simulation.name.as_str(),
        inputs.simulation.description.as_deref(),
        inputs.simulation.author.as_deref(),
    );

    block_on(async move {
        log::debug!("aftgraphs::sim_main: running simulation");
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

            let out_img = Arc::new(Mutex::new(vec![]));
            if let Err(e) = SimulationContext::<T, _>::new_headless(size)
                .run_headless(inputs, headless_input, out_img)
                .await
            {
                log::error!("aftgraphs::sim_main: headless rendering failed: {e}");
                panic!("aftgraphs::sim_main: headless rendering failed:  {e}");
            }
        } else if let Err(e) = SimulationContext::<T, UiWinitPlatform>::new()
            .run_display(inputs)
            .await
        {
            log::error!("aftgraphs::sim_main: simulation failed: {e}");
            panic!("aftgraphs::sim_main: simulation failed: {e}");
        };
    });
}
