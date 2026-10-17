use std::path::Path;

use anyhow::Result;
use log::{debug, error, info};

use donut::{App, Assets, Config, Game, init_logging};

fn run() -> Result<()> {
    use winit::event_loop::EventLoop;

    let dir_cargo = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir_assets = dir_cargo.join("assets");

    let config = Config::load(dir_assets.join("config.toml"))?;
    let assets = Assets::resolve(config, dir_assets)?;

    debug!("Creating event loop...");
    let event_loop = EventLoop::new()?;

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
