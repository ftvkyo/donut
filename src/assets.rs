use std::{borrow::Borrow, error::Error, path::Path};

use anyhow::Context;
use enumset::{EnumSet, EnumSetType};
use image::{ImageBuffer, ImageReader, Rgba};
use log::{debug, trace};
use serde::{Deserialize, Serialize};

use crate::{
    game::{Game, Movement},
    sprite::StageLayerResolver,
};

pub type Texture = ImageBuffer<Rgba<u8>, Vec<u8>>;

#[derive(Deserialize)]
pub struct TilePieceWeight(f32);

impl Default for TilePieceWeight {
    fn default() -> Self {
        Self(1.0)
    }
}

impl Borrow<f32> for TilePieceWeight {
    fn borrow(&self) -> &f32 {
        &self.0
    }
}

#[derive(Debug, EnumSetType, Serialize, Deserialize)]
#[enumset(serialize_repr = "list")]
#[serde(rename_all = "lowercase")]
pub enum TilePieceDesignation {
    Top,
    Right,
    Bottom,
    Left,
    Inner,
}

#[derive(Deserialize)]
pub struct TilePiece {
    pub x: usize,
    pub y: usize,
    #[serde(default)]
    pub is: EnumSet<TilePieceDesignation>,
    #[serde(default)]
    pub weight: TilePieceWeight,
}

#[derive(Deserialize)]
pub struct TileSet {
    pub name: String,
    pub pieces: Vec<TilePiece>,
}

#[derive(Deserialize)]
pub struct StageLayer {
    pub tile_name: String,
    pub tile_map: String,
}

#[derive(Deserialize)]
pub struct StageConfig {
    pub name: String,
    pub size: usize,
    pub layers: Vec<StageLayer>,
}

#[derive(Deserialize)]
pub struct AssetsConfig {
    pub tile_piece_size: usize,
    pub tile_sets: Vec<TileSet>,
    pub stages: Vec<StageConfig>,
}

pub struct Assets {
    pub config: AssetsConfig,
    pub tile_sets_textures: Vec<Texture>,
}

impl Assets {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        debug!("Loading {}...", path.as_ref().to_string_lossy());

        let config = std::fs::read(&path)?;
        let config = String::from_utf8(config)?;
        let config: AssetsConfig = toml::from_str(&config)?;

        let tile_sets_textures = config
            .tile_sets
            .iter()
            .map(|tset| {
                let tset_texture = path
                    .as_ref()
                    .parent()
                    .unwrap()
                    .join(format!("textures/{}.webp", tset.name));

                debug!("Loading {}...", tset_texture.to_string_lossy());

                ImageReader::open(tset_texture)
                    .unwrap()
                    .decode()
                    .unwrap()
                    .into_rgba8()
            })
            .collect();

        Ok(Self {
            config,
            tile_sets_textures,
        })
    }
}

impl TryFrom<Assets> for Game {
    type Error = Box<dyn Error>;

    fn try_from(assets: Assets) -> Result<Self, Self::Error> {
        let texture = assets
            .tile_sets_textures
            .into_iter()
            .next()
            .ok_or("zero tile set textures loaded?")?;

        let stage = assets
            .config
            .stages
            .into_iter()
            .next()
            .ok_or("zero stages declared?")?;

        let layer = stage
            .layers
            .into_iter()
            .next()
            .ok_or("zero stage layers declared?")?;

        let stage_layer = crate::sprite::StageLayer::new(layer.tile_map);

        let slr = StageLayerResolver {
            tile_piece_size: assets.config.tile_piece_size,
            tile_pieces: &assets.config.tile_sets[0].pieces,
            stage_layer: &stage_layer,
        };

        let sprites = slr
            .resolve(stage.size)
            .with_context(|| format!("{stage_layer:#?}"))?;

        trace!("sprites: {sprites:#?}");

        Ok(Self {
            texture,
            sprites,
            movement: Movement::new_at(glam::vec2(4.0, 4.0)),
        })
    }
}
