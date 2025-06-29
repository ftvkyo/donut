use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Result, bail};
use image::ImageBuffer;
use image::Rgba;
use log::debug;

mod animation;
mod config;

pub use animation::LightAnimation;

pub use config::Config;

pub type TexturePixel = Rgba<u8>;
pub type TextureData = ImageBuffer<TexturePixel, Vec<u8>>;

pub type Map = tiled::Map;

pub type Shaders = BTreeMap<String, String>;
pub type LightAnimations = BTreeMap<String, LightAnimation>;
pub type Maps = BTreeMap<String, Map>;

pub struct Assets {
    pub lights: LightAnimations,
    pub shaders: Shaders,
    pub maps: Maps,
}

impl Assets {
    pub fn resolve<P: AsRef<Path>>(config: config::Config, path: P) -> Result<Self> {
        let path = path.as_ref();
        debug!(
            "Resolving config into assets at path '{}'",
            path.to_string_lossy()
        );

        let mut shaders = BTreeMap::new();
        for shader in config.shaders {
            let name = shader.name.clone();

            if shaders.contains_key(&name) {
                bail!("Shader with the name '{name}' already exists");
            }

            let shader_path = path.join(format!("shaders/{}.wgsl", name));

            debug!("Loading '{}'...", shader_path.to_string_lossy());
            shaders.insert(name, std::fs::read_to_string(shader_path)?);
        }

        let mut lights = BTreeMap::new();
        for light in config.lights {
            let name = light.name.clone();

            if lights.contains_key(&name) {
                bail!("Animation with the name '{name}' already exists");
            }

            lights.insert(name, LightAnimation::load(light, &path.join("textures"))?);
        }

        let mut map_loader = tiled::Loader::new();
        let mut maps = BTreeMap::new();
        for map in config.maps {
            let name = map.name.clone();

            if maps.contains_key(&name) {
                bail!("Map with the name '{name}' already exists");
            }

            let path = path.join("maps").join(format!("{name}.tmx"));

            maps.insert(name, map_loader.load_tmx_map(path)?);
        }

        Ok(Self {
            shaders,
            lights,
            maps,
        })
    }
}
