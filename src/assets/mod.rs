use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Result, bail};
use log::debug;

mod config;
mod sprite;
mod stage;
mod tile_set;

pub use config::Config;
pub use config::Tile;
pub use config::TileDesignation;
pub use config::TileWeight;

pub use sprite::Sprite;

pub use stage::Stage;
pub use stage::StageLayer;

pub use tile_set::TextureData;
pub use tile_set::TexturePixel;
pub use tile_set::TileSet;

pub type TileSets = BTreeMap<String, TileSet>;
pub type Stages = BTreeMap<String, Stage>;
pub type Shaders = BTreeMap<String, String>;

pub struct Assets {
    pub tile_size: usize,
    pub tile_sets: TileSets,
    pub stages: Stages,
    pub shaders: Shaders,
}

impl Assets {
    pub fn resolve<P: AsRef<Path>>(config: config::Config, path: P) -> Result<Self> {
        debug!(
            "Resolving config into assets at path {}",
            path.as_ref().to_string_lossy()
        );

        let mut tile_sets = BTreeMap::new();
        for tile_set in config.tile_sets {
            let name = tile_set.name.clone();

            if tile_sets.contains_key(&name) {
                bail!("Tile set with the name {name} already exists");
            }

            tile_sets.insert(
                name,
                TileSet::load(tile_set, &path.as_ref().join("textures"))?,
            );
        }

        let mut stages = BTreeMap::new();
        for stage in config.stages {
            let name = stage.name.clone();

            if stages.contains_key(&name) {
                bail!("Stage with the name {name} already exists");
            }

            stages.insert(name, Stage::from(stage));
        }

        let mut shaders = BTreeMap::new();
        for shader in config.shaders {
            let name = shader.name.clone();

            if shaders.contains_key(&name) {
                bail!("Shader with the name {name} already exists");
            }

            let shader_path = path.as_ref().join(format!("shaders/{}.wgsl", name));

            debug!("Loading {}...", shader_path.to_string_lossy());
            shaders.insert(name, std::fs::read_to_string(shader_path)?);
        }

        Ok(Self {
            tile_size: config.tile_size,
            tile_sets,
            stages,
            shaders,
        })
    }
}
