use std::path::Path;

use anyhow::{Result, ensure};
use image::ImageReader;
use log::debug;

use crate::assets::TextureData;

pub struct LightAnimation {
    pub frame_size: [usize; 2],
    pub frame_count: usize,
    pub texture: TextureData,
}

impl LightAnimation {
    pub fn load<P: AsRef<Path>>(value: super::config::LightAnimation, path: P) -> Result<Self> {
        let path = path.as_ref();
        debug!("Loading light animation {}...", value.name);

        let texture = path.join(&format!("{}.webp", value.name));
        debug!("Loading {}...", texture.to_string_lossy());
        let texture = ImageReader::open(texture)?.decode()?.into_rgba8();

        ensure!(texture.width() as usize % value.frames == 0);

        Ok(Self {
            frame_size: [
                texture.width() as usize / value.frames,
                texture.height() as usize,
            ],
            frame_count: value.frames,
            texture,
        })
    }
}
