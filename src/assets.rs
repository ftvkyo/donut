use std::{error::Error, path::Path};

use anyhow::{Context, bail};
use enumset::EnumSet;
use image::{ImageBuffer, ImageReader, Rgba};
use log::{debug, trace};
use rand::{
    distr::{Distribution, weighted::WeightedIndex},
    rng,
};

use crate::{
    config::{Config, TilePiece, TilePieceDesignation as TPD},
    game::Game,
    view::Vertex,
};

pub type TextureData = ImageBuffer<Rgba<u8>, Vec<u8>>;

pub struct Assets {
    pub config: Config,
    pub tile_set_colors: Vec<TextureData>,
    pub tile_set_normals: Vec<TextureData>,
    pub shader_sources: Vec<String>,
}

impl Assets {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        debug!("Loading {}...", path.as_ref().to_string_lossy());

        let config = std::fs::read(&path)?;
        let config = String::from_utf8(config)?;
        let config: Config = toml::from_str(&config)?;

        let parent = path.as_ref().parent().unwrap();

        let tile_sets_colors = config
            .tile_sets
            .iter()
            .map(|tile_set| {
                let texture = parent.join(format!("textures/{}.webp", tile_set.name));

                debug!("Loading {}...", texture.to_string_lossy());

                ImageReader::open(texture)
                    .unwrap()
                    .decode()
                    .unwrap()
                    .into_rgba8()
            })
            .collect();

        let tile_sets_normals = config
            .tile_sets
            .iter()
            .map(|tile_set| {
                let texture = parent.join(format!("textures/{}-normals.webp", tile_set.name));

                debug!("Loading {}...", texture.to_string_lossy());

                ImageReader::open(texture)
                    .unwrap()
                    .decode()
                    .unwrap()
                    .into_rgba8()
            })
            .collect();

        let shader_sources = config
            .shaders
            .iter()
            .map(|shader| {
                let shader_path = parent.join(format!("shaders/{}.wgsl", shader.name));

                debug!("Loading {}...", shader_path.to_string_lossy());

                std::fs::read_to_string(shader_path).unwrap()
            })
            .collect();

        Ok(Self {
            config,
            tile_set_colors: tile_sets_colors,
            tile_set_normals: tile_sets_normals,
            shader_sources,
        })
    }
}

impl TryFrom<Assets> for Game {
    type Error = Box<dyn Error>;

    fn try_from(assets: Assets) -> Result<Self, Self::Error> {
        let shader = assets
            .shader_sources
            .into_iter()
            .next()
            .ok_or("zero shaders loaded?")?;

        let texture_color = assets
            .tile_set_colors
            .into_iter()
            .next()
            .ok_or("zero tile set textures loaded?")?;

        let texture_normal = assets
            .tile_set_normals
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

        let stage_layer = StageLayer::new(layer.tile_map);

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
            shader,
            texture_color,
            texture_normal,
            sprites,
        })
    }
}

#[derive(Debug)]
pub struct Sprite {
    pub x: f32,
    pub y: f32,
    pub z: f32,

    pub w: f32,
    pub h: f32,

    pub tex_x: f32,
    pub tex_y: f32,

    pub tex_w: f32,
    pub tex_h: f32,
}

impl Sprite {
    pub fn vertex_data(&self) -> [Vertex; 4] {
        [
            Vertex {
                pos: [self.x, self.y, self.z, 1.0],
                tex_coord: [self.tex_x, self.tex_y + self.tex_h],
            },
            Vertex {
                pos: [self.x + self.w, self.y, self.z, 1.0],
                tex_coord: [self.tex_x + self.tex_w, self.tex_y + self.tex_h],
            },
            Vertex {
                pos: [self.x + self.w, self.y + self.h, self.z, 1.0],
                tex_coord: [self.tex_x + self.tex_w, self.tex_y],
            },
            Vertex {
                pos: [self.x, self.y + self.h, self.z, 1.0],
                tex_coord: [self.tex_x, self.tex_y],
            },
        ]
    }

    pub fn index_data(&self, offset: u16) -> [u16; 6] {
        [
            offset + 0,
            offset + 1,
            offset + 2,
            offset + 2,
            offset + 3,
            offset + 0,
        ]
    }
}

pub struct StageLayer {
    inner: Vec<Vec<bool>>,
}

impl StageLayer {
    pub fn new<S: AsRef<str>>(s: S) -> Self {
        let inner = s
            .as_ref()
            .trim()
            .split('\n')
            .rev()
            .map(|row| row.chars().map(|c| c == 'x').collect())
            .collect();

        Self { inner }
    }

    fn is_filled(&self, x: isize, y: isize) -> bool {
        if x < 0 || y < 0 {
            return false;
        }

        if let Some(row) = self.inner.get(y as usize) {
            if let Some(cell) = row.get(x as usize) {
                return *cell;
            }
        }

        return false;
    }
}

impl std::fmt::Debug for StageLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut st = f.debug_struct("StageLayer");

        for (irow, row) in self.inner.iter().enumerate().rev() {
            let row: String = row.iter().map(|c| if *c { 'x' } else { '_' }).collect();
            st.field(&format!("y={irow}"), &row);
        }

        st.finish()
    }
}

pub struct StageLayerResolver<'a, 'b> {
    pub tile_piece_size: usize,
    pub tile_pieces: &'a Vec<TilePiece>,
    pub stage_layer: &'b StageLayer,
}

impl<'a> StageLayerResolver<'a, '_> {
    fn select_tile_piece(&self, designation: EnumSet<TPD>) -> anyhow::Result<&'a TilePiece> {
        let candidates: Vec<_> = self
            .tile_pieces
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
        let distribution = WeightedIndex::new(&weights)?;

        Ok(candidates[distribution.sample(&mut rng())])
    }

    pub fn resolve(self, stage_layer_size: usize) -> anyhow::Result<Vec<Sprite>> {
        let mut sprites = vec![];

        let (w, h) = (0.5, 0.5);

        let tex_w = self.tile_piece_size as f32;
        let tex_h = self.tile_piece_size as f32;

        let sprite = |x, y, tex_x, tex_y| Sprite {
            x,
            y,
            w,
            h,
            z: 0.0,
            tex_x: tex_x as f32 * tex_w,
            tex_y: tex_y as f32 * tex_h,
            tex_w,
            tex_h,
        };

        for x in 0..stage_layer_size as isize {
            for y in 0..stage_layer_size as isize {
                if !self.stage_layer.is_filled(x, y) {
                    continue;
                }

                let filled_r = self.stage_layer.is_filled(x + 1, y);
                let filled_tr = self.stage_layer.is_filled(x + 1, y + 1);
                let filled_t = self.stage_layer.is_filled(x, y + 1);
                let filled_tl = self.stage_layer.is_filled(x - 1, y + 1);
                let filled_l = self.stage_layer.is_filled(x - 1, y);
                let filled_bl = self.stage_layer.is_filled(x - 1, y - 1);
                let filled_b = self.stage_layer.is_filled(x, y - 1);
                let filled_br = self.stage_layer.is_filled(x + 1, y - 1);

                let (x, y) = (x as f32, y as f32);

                // Top-left quarter of the tile.
                // Never bottom nor right.
                let mut tpds_tl = EnumSet::new();

                // Top-right quarter of the tile.
                // Never bottom nor left.
                let mut tpds_tr = EnumSet::new();

                // Bottom-right quarter of the tile.
                // Never top nor left.
                let mut tpds_br = EnumSet::new();

                // Bottom-left quarter of the tile.
                // Never top nor right.
                let mut tpds_bl = EnumSet::new();

                if !filled_t {
                    tpds_tl |= TPD::Top;
                    tpds_tr |= TPD::Top;
                }

                if !filled_r {
                    tpds_tr |= TPD::Right;
                    tpds_br |= TPD::Right;
                }

                if !filled_b {
                    tpds_br |= TPD::Bottom;
                    tpds_bl |= TPD::Bottom;
                }

                if !filled_l {
                    tpds_tl |= TPD::Left;
                    tpds_bl |= TPD::Left;
                }

                if filled_t && filled_l && !filled_tl {
                    tpds_tl |= TPD::Inner | TPD::Top | TPD::Left;
                }

                if filled_t && filled_r && !filled_tr {
                    tpds_tr |= TPD::Inner | TPD::Top | TPD::Right;
                }

                if filled_b && filled_r && !filled_br {
                    tpds_br |= TPD::Inner | TPD::Bottom | TPD::Right;
                }

                if filled_b && filled_l && !filled_bl {
                    tpds_bl |= TPD::Inner | TPD::Bottom | TPD::Left;
                }

                let tp_tl = self
                    .select_tile_piece(tpds_tl)
                    .with_context(|| format!("top-left quarter of tile at x={x} y={y}"))?;

                sprites.push(sprite(x, y + h, tp_tl.x, tp_tl.y));

                let tp_tr = self
                    .select_tile_piece(tpds_tr)
                    .with_context(|| format!("top-right quarter of tile at x={x} y={y}"))?;

                sprites.push(sprite(x + w, y + h, tp_tr.x, tp_tr.y));

                let tp_br = self
                    .select_tile_piece(tpds_br)
                    .with_context(|| format!("bottom-right quarter of tile at x={x} y={y}"))?;

                sprites.push(sprite(x + w, y, tp_br.x, tp_br.y));

                let tp_bl = self
                    .select_tile_piece(tpds_bl)
                    .with_context(|| format!("bottom-left quarter of tile at x={x} y={y}"))?;

                sprites.push(sprite(x, y, tp_bl.x, tp_bl.y));
            }
        }

        Ok(sprites)
    }
}
