use std::{ops::Deref, path::Path};

use anyhow::Result;
use enumset::{EnumSet, EnumSetType};
use log::debug;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct TileWeight(f32);

impl Default for TileWeight {
    fn default() -> Self {
        Self(1.0)
    }
}

impl Deref for TileWeight {
    type Target = f32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, EnumSetType, Serialize, Deserialize)]
#[enumset(serialize_repr = "list")]
#[serde(rename_all = "lowercase")]
pub enum TileDesignation {
    Top,
    Right,
    Bottom,
    Left,
    Inner,
}

#[derive(Deserialize)]
pub struct Tile {
    pub x: usize,
    pub y: usize,
    #[serde(default)]
    pub is: EnumSet<TileDesignation>,
    #[serde(default)]
    pub weight: TileWeight,
}

#[derive(Deserialize)]
pub struct TileSet {
    pub name: String,
    pub tiles: Vec<Tile>,
}

#[derive(Deserialize)]
pub struct LightAnimation {
    pub name: String,
    pub frames: usize,
}

#[derive(Deserialize)]
pub struct StageLayer {
    pub tile_name: String,
    pub tile_map: String,
    #[serde(default)]
    pub z: f32,
}

#[derive(Deserialize)]
pub struct Stage {
    pub name: String,
    pub layers: Vec<StageLayer>,
}

#[derive(Deserialize)]
pub struct Shader {
    pub name: String,
}

#[derive(Deserialize)]
pub struct Config {
    pub tile_size: usize,
    pub tile_sets: Vec<TileSet>,
    pub lights: Vec<LightAnimation>,
    pub stages: Vec<Stage>,
    pub shaders: Vec<Shader>,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        debug!("Loading config '{}'...", path.as_ref().to_string_lossy());
        let config = toml::from_str(&std::fs::read_to_string(path)?)?;
        Ok(config)
    }
}
