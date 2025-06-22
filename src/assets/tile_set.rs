use std::path::Path;

use anyhow::{Result, bail};
use enumset::EnumSet;
use image::{ImageBuffer, ImageReader, Rgba};
use log::debug;
use rand::distr::Distribution;

use crate::assets::{Tile, TileDesignation};

pub type TextureData = ImageBuffer<Rgba<u8>, Vec<u8>>;

pub struct TileSet {
    tiles: Vec<super::Tile>,
    pub texture_color: TextureData,
    pub texture_normal: TextureData,
}

impl TileSet {
    pub fn load<P: AsRef<Path>>(value: super::config::TileSet, path: P) -> Result<Self> {
        let path = path.as_ref();
        debug!("Loading tile set {}...", value.name);

        let texture_color = path.join(&format!("{}.webp", value.name));
        debug!("Loading {}...", texture_color.to_string_lossy());
        let texture_color = ImageReader::open(texture_color)?.decode()?.into_rgba8();

        let texture_normal = path.join(&format!("{}-normals.webp", value.name));
        debug!("Loading {}...", texture_normal.to_string_lossy());
        let texture_normal = ImageReader::open(texture_normal)?.decode()?.into_rgba8();

        Ok(Self {
            tiles: value.tiles,
            texture_color,
            texture_normal,
        })
    }

    pub fn select_tile(&self, designation: EnumSet<TileDesignation>) -> Result<&Tile> {
        let candidates: Vec<_> = self
            .tiles
            .iter()
            .filter(|tp| tp.is == designation)
            .collect();

        if candidates.len() == 0 {
            bail!("No tile piece with designation {designation:?}");
        }

        if candidates.len() == 1 {
            return Ok(candidates[0]);
        }

        let weights: Vec<f32> = candidates.iter().map(|tp| *tp.weight).collect();
        let distribution = rand::distr::weighted::WeightedIndex::new(&weights)?;

        Ok(candidates[distribution.sample(&mut rand::rng())])
    }
}
