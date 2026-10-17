#![cfg_attr(test, allow(dead_code))]

mod app;
mod assets;
mod game;
pub mod geo;
mod logging;
mod phys;
#[cfg(test)]
mod testing;
mod view;

pub use app::App;
pub use assets::{Assets, Config, collision_segments, occlusion_buckets};
pub use game::Game;
pub use logging::init_logging;

pub const CRATE_NAME: &str = module_path!();
