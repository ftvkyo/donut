use std::path::Path;

use anyhow::{Result, ensure};
use image::ImageReader;
use log::debug;

pub type TexturePixel = image::Rgba<u8>;
pub type TextureData = image::ImageBuffer<TexturePixel, Vec<u8>>;

pub struct LightAnimation {
    pub name: String,
    pub frame_size: [usize; 2],
    pub frames: usize,
    pub frames_per_second: usize,
    pub texture: TextureData,
}

impl LightAnimation {
    pub fn load<P: AsRef<Path>>(value: super::config::LightAnimation, path: P) -> Result<Self> {
        let path = path.as_ref();

        let super::config::LightAnimation {
            name,
            frames,
            frames_per_second,
        } = value;

        debug!("Loading light animation '{name}'...");

        let texture = path.join(&format!("{name}.webp"));
        debug!("Loading '{}'...", texture.to_string_lossy());
        let texture = ImageReader::open(texture)?.decode()?.into_rgba8();

        ensure!(texture.width() as usize % frames == 0);

        Ok(Self {
            name,
            frame_size: [texture.width() as usize / frames, texture.height() as usize],
            frames,
            frames_per_second,
            texture,
        })
    }
}
