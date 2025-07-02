use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Context;
use anyhow::{Result, bail};
use log::debug;

mod animation;
mod config;
mod stage;
mod tile_set;

pub use animation::LightAnimation;

pub use config::Config;
pub use config::Tile;
pub use config::TileDesignation;
pub use config::TileWeight;

pub use stage::Stage;
pub use stage::StageLayer;

pub use tile_set::TextureData;
pub use tile_set::TexturePixel;
pub use tile_set::TileSet;

pub struct Assets {
    tile_sets: Vec<TileSet>,
    lights: Vec<LightAnimation>,
    stages: Vec<Stage>,
    shaders: BTreeMap<String, String>,
}

impl Assets {
    pub fn resolve<P: AsRef<Path>>(config: config::Config, path: P) -> Result<Self> {
        debug!(
            "Resolving config into assets at path '{}'",
            path.as_ref().to_string_lossy()
        );

        let mut s = Self {
            tile_sets: Vec::with_capacity(config.tile_sets.len()),
            lights: Vec::with_capacity(config.lights.len()),
            stages: Vec::with_capacity(config.stages.len()),
            shaders: BTreeMap::new(),
        };

        for tile_set in config.tile_sets {
            s.add_tile_set(tile_set, path.as_ref().join("textures"))?;
        }

        for light in config.lights {
            s.add_light(light, path.as_ref().join("textures"))?;
        }

        for stage in config.stages {
            s.add_stage(stage)?;
        }

        for shader in config.shaders {
            s.add_shader(shader, path.as_ref().join("shaders"))?;
        }

        Ok(s)
    }

    fn add_tile_set(&mut self, tile_set: config::TileSet, path: impl AsRef<Path>) -> Result<()> {
        if self.find_tile_set(&tile_set.name).is_ok() {
            bail!("Tile set with the name '{}' already exists", tile_set.name);
        }

        let tile_set = TileSet::load(tile_set, path)?;
        self.tile_sets.push(tile_set);

        Ok(())
    }

    pub fn find_tile_set(&self, name: &str) -> Result<(usize, &TileSet)> {
        self.tile_sets
            .iter()
            .enumerate()
            .find(|(_, e)| e.name == name)
            .with_context(|| format!("No tile set '{name}'"))
    }

    pub fn get_tile_set(&self, index: usize) -> Option<&TileSet> {
        self.tile_sets.get(index)
    }

    pub fn all_tile_sets(&self) -> impl Iterator<Item = (usize, &TileSet)> {
        self.tile_sets.iter().enumerate()
    }

    pub fn find_light(&self, name: &str) -> Result<(usize, &LightAnimation)> {
        self.lights
            .iter()
            .enumerate()
            .find(|(_, e)| e.name == name)
            .with_context(|| format!("No light animation '{name}'"))
    }

    fn add_light(&mut self, light: config::LightAnimation, path: impl AsRef<Path>) -> Result<()> {
        if self.find_light(&light.name).is_ok() {
            bail!(
                "Light animation with the name '{}' already exists",
                light.name
            );
        }

        let light = LightAnimation::load(light, path)?;
        self.lights.push(light);

        Ok(())
    }

    pub fn get_light(&self, index: usize) -> Option<&LightAnimation> {
        self.lights.get(index)
    }

    pub fn all_lights(&self) -> impl Iterator<Item = (usize, &LightAnimation)> {
        self.lights.iter().enumerate()
    }

    pub fn find_stage(&self, name: &str) -> Result<(usize, &Stage)> {
        self.stages
            .iter()
            .enumerate()
            .find(|(_, e)| e.name == name)
            .with_context(|| format!("No stage '{name}'"))
    }

    pub fn get_stage(&self, index: usize) -> Option<&Stage> {
        self.stages.get(index)
    }

    fn add_stage(&mut self, stage: config::Stage) -> Result<()> {
        if self.find_stage(&stage.name).is_ok() {
            bail!("Stage with the name '{}' already exists", stage.name);
        }

        let stage = Stage::new(&self, stage)?;
        self.stages.push(stage);

        Ok(())
    }

    pub fn find_shader(&self, name: &str) -> Result<&String> {
        self.shaders
            .get(name)
            .with_context(|| format!("No shader '{name}'"))
    }

    fn add_shader(&mut self, shader: config::Shader, path: impl AsRef<Path>) -> Result<()> {
        if self.find_shader(&shader.name).is_ok() {
            bail!("Shader with the name '{}' already exists", shader.name);
        }

        let shader_path = path.as_ref().join(format!("{}.wgsl", shader.name));
        debug!("Loading '{}'...", shader_path.to_string_lossy());
        self.shaders
            .insert(shader.name, std::fs::read_to_string(shader_path)?);

        Ok(())
    }
}
