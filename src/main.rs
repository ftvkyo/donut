use std::error::Error;

use log::{debug, error, info};

use crate::{assets::Assets, game::Game};

mod app;
mod assets;
mod game;
mod renderer;

fn init_logging() {
    use log::LevelFilter;

    let level = if cfg!(debug_assertions) {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    env_logger::builder()
        .filter_module(module_path!(), level)
        .parse_default_env()
        .init();
}

fn run() -> Result<(), Box<dyn Error>> {
    use winit::event_loop::{ControlFlow, EventLoop};

    let assets = Assets::load("assets/assets.toml")?;
    let game = Game::try_from(assets)?;

    debug!("Creating event loop...");
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    debug!("Creating the app...");
    let mut app = app::App::new(game);

    debug!("Running the app...");
    event_loop.run_app(&mut app)?;

    Ok(())
}

fn main() {
    init_logging();

    match run() {
        Ok(()) => info!("Done."),
        Err(err) => error!("Encountered an error:\n{err:?}"),
    }
}
