use std::path::Path;

use anyhow::Result;
use log::debug;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct LightAnimation {
    pub name: String,
    pub frames: usize,
    pub frames_per_second: usize,
}

#[derive(Deserialize)]
pub struct Shader {
    pub name: String,
}

#[derive(Deserialize)]
pub struct Config {
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
