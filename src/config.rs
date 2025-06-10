use std::ops::Deref;

use enumset::{EnumSet, EnumSetType};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct TilePieceWeight(f32);

impl Default for TilePieceWeight {
    fn default() -> Self {
        Self(1.0)
    }
}

impl Deref for TilePieceWeight {
    type Target = f32;

    fn deref(&self) -> &Self::Target {
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
    // pub tile_name: String,
    pub tile_map: String,
}

#[derive(Deserialize)]
pub struct Stage {
    // pub name: String,
    pub size: usize,
    pub layers: Vec<StageLayer>,
}

#[derive(Deserialize)]
pub struct Config {
    pub tile_piece_size: usize,
    pub tile_sets: Vec<TileSet>,
    pub stages: Vec<Stage>,
}
