use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::Path;

use anyhow::{Context, Result, bail};
use log::debug;

mod config;
mod light;
mod map;
mod texture;
mod tileset;

pub use config::Config;
pub use light::LightSource;
pub use map::Map;
pub use texture::TextureData;
pub use texture::TexturePixel;
pub use tileset::Tileset;
pub use tileset::TilesetId;

pub struct Assets {
    lights: Vec<LightSource>,
    shaders: BTreeMap<String, String>,

    maps: Vec<Map>,
    tilesets: Vec<Tileset>,

    tiled_loader: tiled::Loader,
}

impl Assets {
    fn empty() -> Self {
        Self {
            lights: Vec::new(),
            shaders: BTreeMap::new(),

            maps: Vec::new(),
            tilesets: Vec::new(),

            tiled_loader: tiled::Loader::new(),
        }
    }

    pub fn resolve<P: AsRef<Path>>(config: config::Config, path: P) -> Result<Self> {
        let path = path.as_ref();

        debug!(
            "Resolving config into assets at path '{}'",
            path.to_string_lossy()
        );

        let mut s = Self::empty();

        let path_lights = path.join("textures");
        for light in config.lights {
            s.load_light(light, &path_lights)?;
        }

        let path_shaders = path.join("shaders");
        for shader in config.shaders {
            s.load_shader(shader, &path_shaders)?;
        }

        let path_maps = path.join("maps");
        let path_maps_extension = OsStr::new("tmx");
        for entry in std::fs::read_dir(path_maps)? {
            let entry = entry?;
            let entry_path = entry.path();
            if entry_path.is_file() && entry_path.extension() == Some(path_maps_extension) {
                s.load_map(entry_path)?;
            }
        }

        Ok(s)
    }

    fn load_light(&mut self, light: config::LightAnimation, path: impl AsRef<Path>) -> Result<()> {
        if self.find_light(&light.name).is_ok() {
            bail!(
                "Light animation with the name '{}' already exists",
                light.name
            );
        }

        let light = LightSource::load(light, path)?;
        self.lights.push(light);

        Ok(())
    }

    pub fn find_light(&self, name: &str) -> Result<(usize, &LightSource)> {
        self.lights
            .iter()
            .enumerate()
            .find(|(_, e)| e.name == name)
            .with_context(|| format!("No light animation '{name}'"))
    }

    pub fn get_light(&self, index: usize) -> Option<&LightSource> {
        self.lights.get(index)
    }

    pub fn all_lights(&self) -> impl Iterator<Item = (usize, &LightSource)> {
        self.lights.iter().enumerate()
    }

    fn load_shader(&mut self, shader: config::Shader, path: impl AsRef<Path>) -> Result<()> {
        if self.find_shader(&shader.name).is_ok() {
            bail!("Shader with the name '{}' already exists", shader.name);
        }

        let shader_path = path.as_ref().join(format!("{}.wgsl", shader.name));
        debug!("Loading '{}'...", shader_path.to_string_lossy());
        self.shaders
            .insert(shader.name, std::fs::read_to_string(shader_path)?);

        Ok(())
    }

    pub fn find_shader(&self, name: &str) -> Result<&String> {
        self.shaders
            .get(name)
            .with_context(|| format!("No shader '{name}'"))
    }

    pub fn load_map(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        debug!("Loading map '{}'", path.to_string_lossy());

        let map = self.tiled_loader.load_tmx_map(&path)?;
        let mut tileset_map = Vec::new();

        for tileset in map.tilesets() {
            let tileset_id = TilesetId::new(tileset);
            match self
                .tilesets
                .iter()
                .enumerate()
                .find(|(_, t)| tileset_id == t.id())
            {
                Some((i, _)) => tileset_map.push(i),
                None => {
                    let tileset = Tileset::load_for_map(tileset)?;
                    self.tilesets.push(tileset);
                    tileset_map.push(self.tilesets.len() - 1);
                }
            }
        }

        self.maps.push(Map::new(map, tileset_map)?);

        Ok(())
    }

    pub fn find_map(&self, name: &str) -> Result<(usize, &Map)> {
        self.maps
            .iter()
            .enumerate()
            .find(|(_, m)| m.name == name)
            .with_context(|| format!("No map '{name}'"))
    }

    pub fn get_map(&self, index: usize) -> Option<&Map> {
        self.maps.get(index)
    }

    pub fn all_tilesets(&self) -> impl Iterator<Item = (usize, &Tileset)> {
        self.tilesets.iter().enumerate()
    }
}
