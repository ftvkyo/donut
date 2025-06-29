use std::collections::BTreeSet;

use anyhow::{Context, Result};
use enumset::EnumSet;
use glam::{vec2, vec3};

use crate::{
    assets::{TileDesignation as TD, TileSets},
    view::Quad,
};

pub struct StageLayer {
    pub tile_name: String,
    tile_map: BTreeSet<(usize, usize)>,
    z: f32,
}

impl From<super::config::StageLayer> for StageLayer {
    fn from(value: super::config::StageLayer) -> Self {
        let mut tile_map = BTreeSet::new();

        let s = value.tile_map.trim();
        for (y, row) in s.split('\n').rev().enumerate() {
            for (x, c) in row.chars().enumerate() {
                if c == 'x' {
                    tile_map.insert((x, y));
                }
            }
        }

        Self {
            tile_name: value.tile_name,
            tile_map,
            z: value.z,
        }
    }
}

impl StageLayer {
    fn is_filled(&self, x: isize, y: isize) -> bool {
        if x < 0 || y < 0 {
            return false;
        }

        return self.tile_map.get(&(x as usize, y as usize)).is_some();
    }

    pub fn quads(&self, tile_sets: &TileSets, stage_size: [usize; 2]) -> Result<Vec<Quad>> {
        let tile_set = tile_sets
            .get(&self.tile_name)
            .with_context(|| format!("No tileset found with name {}", self.tile_name))?;

        let mut sprites = Vec::with_capacity(self.tile_map.len());

        let offset_x = stage_size[0] as f32 / -2.0;
        let offset_y = stage_size[1] as f32 / -2.0;
        let (w, h) = (0.5, 0.5);
        let (tex_w, tex_h) = (tile_set.tile_size[0] as f32, tile_set.tile_size[1] as f32);

        let sprite = |x, y, tex_x, tex_y| Quad {
            pos: vec3(offset_x + x + w / 2.0, offset_y + y + h / 2.0, self.z),
            dim: vec2(w, h),
            rot: 0.0,
            tex_pos: vec2(tex_x as f32 * tex_w, tex_y as f32 * tex_h),
            tex_dim: vec2(tex_w, tex_w),
        };

        for (x, y) in self.tile_map.iter() {
            let (x, y) = (*x as isize, *y as isize);

            let filled_r = self.is_filled(x + 1, y);
            let filled_tr = self.is_filled(x + 1, y + 1);
            let filled_t = self.is_filled(x, y + 1);
            let filled_tl = self.is_filled(x - 1, y + 1);
            let filled_l = self.is_filled(x - 1, y);
            let filled_bl = self.is_filled(x - 1, y - 1);
            let filled_b = self.is_filled(x, y - 1);
            let filled_br = self.is_filled(x + 1, y - 1);

            let (x, y) = (x as f32, y as f32);

            // Top-left quarter of the tile.
            // Never bottom nor right.
            let mut tds_tl = EnumSet::new();

            // Top-right quarter of the tile.
            // Never bottom nor left.
            let mut tds_tr = EnumSet::new();

            // Bottom-right quarter of the tile.
            // Never top nor left.
            let mut tds_br = EnumSet::new();

            // Bottom-left quarter of the tile.
            // Never top nor right.
            let mut tds_bl = EnumSet::new();

            if !filled_t {
                tds_tl |= TD::Top;
                tds_tr |= TD::Top;
            }

            if !filled_r {
                tds_tr |= TD::Right;
                tds_br |= TD::Right;
            }

            if !filled_b {
                tds_br |= TD::Bottom;
                tds_bl |= TD::Bottom;
            }

            if !filled_l {
                tds_tl |= TD::Left;
                tds_bl |= TD::Left;
            }

            if filled_t && filled_l && !filled_tl {
                tds_tl |= TD::Inner | TD::Top | TD::Left;
            }

            if filled_t && filled_r && !filled_tr {
                tds_tr |= TD::Inner | TD::Top | TD::Right;
            }

            if filled_b && filled_r && !filled_br {
                tds_br |= TD::Inner | TD::Bottom | TD::Right;
            }

            if filled_b && filled_l && !filled_bl {
                tds_bl |= TD::Inner | TD::Bottom | TD::Left;
            }

            let tp_tl = tile_set
                .select_tile(tds_tl)
                .with_context(|| format!("top-left quarter of tile at x={x} y={y}"))?;

            sprites.push(sprite(x, y + h, tp_tl.x, tp_tl.y));

            let tp_tr = tile_set
                .select_tile(tds_tr)
                .with_context(|| format!("top-right quarter of tile at x={x} y={y}"))?;

            sprites.push(sprite(x + w, y + h, tp_tr.x, tp_tr.y));

            let tp_br = tile_set
                .select_tile(tds_br)
                .with_context(|| format!("bottom-right quarter of tile at x={x} y={y}"))?;

            sprites.push(sprite(x + w, y, tp_br.x, tp_br.y));

            let tp_bl = tile_set
                .select_tile(tds_bl)
                .with_context(|| format!("bottom-left quarter of tile at x={x} y={y}"))?;

            sprites.push(sprite(x, y, tp_bl.x, tp_bl.y));
        }

        Ok(sprites)
    }
}

pub struct Stage {
    pub size: [usize; 2],
    pub layers: Vec<StageLayer>,
}

impl From<super::config::Stage> for Stage {
    fn from(value: super::config::Stage) -> Self {
        let mut layers = Vec::with_capacity(value.layers.len());

        for layer in value.layers {
            layers.push(StageLayer::from(layer));
        }

        Self {
            size: value.size,
            layers,
        }
    }
}
