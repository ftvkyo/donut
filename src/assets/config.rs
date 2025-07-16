use std::path::Path;

use anyhow::Result;
use log::debug;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum Shape {
    Disc { radius: f32 },
}

#[derive(Deserialize)]
pub struct LightAnimation {
    pub name: String,
    pub frames: usize,
    pub frames_per_second: usize,
    pub shape: Shape,
}

#[derive(Deserialize)]
pub struct Shader {
    pub name: String,
}

#[derive(Deserialize)]
pub struct Config {
    pub max_timestep: f32,
    pub lights: Vec<LightAnimation>,
    pub shaders: Vec<Shader>,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        debug!("Loading config '{}'...", path.as_ref().to_string_lossy());
        let config = toml::from_str(&std::fs::read_to_string(path)?)?;
        Ok(config)
    }
}
