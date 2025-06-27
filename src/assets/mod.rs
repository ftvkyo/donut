use std::collections::BTreeMap;
use std::path::Path;

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

pub type Animations = BTreeMap<String, LightAnimation>;
pub type TileSets = BTreeMap<String, TileSet>;
pub type Stages = BTreeMap<String, Stage>;
pub type Shaders = BTreeMap<String, String>;

pub struct Assets {
    pub tile_sets: TileSets,
    pub lights: Animations,
    pub stages: Stages,
    pub shaders: Shaders,
}

impl Assets {
    pub fn resolve<P: AsRef<Path>>(config: config::Config, path: P) -> Result<Self> {
        debug!(
            "Resolving config into assets at path '{}'",
            path.as_ref().to_string_lossy()
        );

        let mut tile_sets = BTreeMap::new();
        for tile_set in config.tile_sets {
            let name = tile_set.name.clone();

            if tile_sets.contains_key(&name) {
                bail!("Tile set with the name '{name}' already exists");
            }

            tile_sets.insert(
                name,
                TileSet::load(tile_set, &path.as_ref().join("textures"))?,
            );
        }

        let mut lights = BTreeMap::new();
        for light in config.lights {
            let name = light.name.clone();

            if lights.contains_key(&name) {
                bail!("Animation with the name '{name}' already exists");
            }

            lights.insert(
                name,
                LightAnimation::load(light, &path.as_ref().join("textures"))?,
            );
        }

        let mut stages = BTreeMap::new();
        for stage in config.stages {
            let name = stage.name.clone();

            if stages.contains_key(&name) {
                bail!("Stage with the name '{name}' already exists");
            }

            stages.insert(name, Stage::from(stage));
        }

        let mut shaders = BTreeMap::new();
        for shader in config.shaders {
            let name = shader.name.clone();

            if shaders.contains_key(&name) {
                bail!("Shader with the name '{name}' already exists");
            }

            let shader_path = path.as_ref().join(format!("shaders/{}.wgsl", name));

            debug!("Loading '{}'...", shader_path.to_string_lossy());
            shaders.insert(name, std::fs::read_to_string(shader_path)?);
        }

        Ok(Self {
            tile_sets,
            lights,
            stages,
            shaders,
        })
    }
}
