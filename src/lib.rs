#![cfg_attr(test, allow(dead_code))]

mod app;
mod assets;
mod game;
pub mod geo;
mod logging;
mod phys;
mod view;

pub use app::App;
pub use assets::{Assets, Config, occlusion_segments};
pub use game::Game;
pub use logging::init_logging;

pub const CRATE_NAME: &str = module_path!();
