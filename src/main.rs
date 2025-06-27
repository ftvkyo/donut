use anyhow::Result;
use log::{debug, error, info};

use crate::{
    app::App,
    assets::{Assets, Config},
    game::Game,
};

mod app;
mod assets;
mod game;
mod view;

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

fn run() -> Result<()> {
    use winit::event_loop::{ControlFlow, EventLoop};

    let config = Config::load("assets/config.toml")?;
    let assets = Assets::resolve(config, "assets")?;

    debug!("Creating event loop...");
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    debug!("Initialising game state...");
    let game = Game::new(&assets)?;

    debug!("Creating the app...");
    let mut app = App::new(&assets, game);

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
