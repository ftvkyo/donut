use anyhow::{Context, Result, bail, ensure};
use glam::{vec2, vec3};
use winit::dpi::LogicalSize;

use crate::{
    geo::{Point, Segment, VisibilityPolygon},
    view::Quad,
};

pub struct Map {
    pub name: String,
    inner: tiled::Map,
    tileset_map: Vec<usize>,
    occlusion_segments: Vec<Segment>,
}

impl Map {
    pub(super) fn new(inner: tiled::Map, tileset_map: Vec<usize>) -> Result<Self> {
        let name = inner
            .source
            .file_stem()
            .context("Map path has no file stem?")?
            .to_str()
            .context("Map path file stem is not valid UTF-8?")?
            .to_string();

        ensure!(inner.orientation == tiled::Orientation::Orthogonal);

        let mut s = Self {
            name,
            inner,
            tileset_map,
            occlusion_segments: Vec::new(),
        };
        s.recalculate_occlusion_segments()?;

        Ok(s)
    }

    pub fn size_tiles(&self) -> LogicalSize<u32> {
        LogicalSize {
            width: self.inner.width,
            height: self.inner.height,
        }
    }

    pub fn quads(&self) -> Result<Vec<Quad>> {
        let mut quads = Vec::new();

        for layer in self.inner.layers() {
            let z = match layer.properties.get("Z") {
                Some(tiled::PropertyValue::FloatValue(z)) => *z,
                _ => 0.0,
            };

            let layer = layer
                .as_tile_layer()
                .context("Only tile layers are supported")?;

            let layer = match layer {
                tiled::TileLayer::Finite(layer) => layer,
                _ => bail!("Only finite tile layers are supported"),
            };

            quads.extend(self.quads_for_layer(z, &layer));
        }

        Ok(quads)
    }

    fn quads_for_layer(
        &self,
        z: f32,
        layer: &tiled::FiniteTileLayer<'_>,
    ) -> impl Iterator<Item = Quad> {
        let map_w = self.inner.width;
        let map_h = self.inner.height;
        // Shift all tiles to make (0.0, 0.0) be the map center
        // TODO: figure out what's going on with the magic numbers
        let map_offset = vec2(map_w as f32 - 1.0, map_h as f32 + 1.0) / -2.0;

        let layer_w = layer.width();
        let layer_h = layer.height();

        (0..layer_w).flat_map(move |layer_x| {
            (0..layer_h).filter_map(move |layer_y| {
                if let Some(layer_tile) = layer.get_tile(layer_x as i32, layer_y as i32) {
                    assert!(!layer_tile.flip_d);
                    assert!(!layer_tile.flip_h);
                    assert!(!layer_tile.flip_v);

                    let tileset = layer_tile.get_tileset();

                    let tile_id = layer_tile.id();
                    let tileset_x = (tile_id % tileset.columns) * tileset.tile_width;
                    let tileset_y = (tile_id / tileset.columns) * tileset.tile_height;

                    let pos_x = layer_x as f32;
                    let pos_y = (layer_h - layer_y) as f32;

                    let pos = vec3(pos_x, pos_y, z) + map_offset.extend(0.0);
                    let dim = vec2(1.0, 1.0);
                    let rot = 0.0;

                    let tex_num = self.tileset_map[layer_tile.tileset_index()] as u32;
                    let tex_pos = vec2(tileset_x as f32, tileset_y as f32);
                    let tex_dim = vec2(tileset.tile_width as f32, tileset.tile_height as f32);

                    Some(Quad {
                        pos,
                        dim,
                        rot,
                        tex_num,
                        tex_pos,
                        tex_dim,
                    })
                } else {
                    None
                }
            })
        })
    }

    fn recalculate_occlusion_segments(&mut self) -> Result<()> {
        let map_w2 = self.inner.width as f32 / 2.0;
        let map_h2 = self.inner.height as f32 / 2.0;

        for layer in self.inner.layers() {
            let occluding = match layer.properties.get("Occluding") {
                Some(tiled::PropertyValue::BoolValue(val)) => *val,
                _ => false,
            };

            if !occluding {
                continue;
            }

            let layer = layer
                .as_tile_layer()
                .context("Only tile layers are supported")?;

            let layer = match layer {
                tiled::TileLayer::Finite(layer) => layer,
                _ => bail!("Only finite tile layers are supported"),
            };

            let layer_w = layer.width() as i32;
            let layer_h = layer.height() as i32;

            let is_solid = |x: i32, y: i32| {
                let x = x.clamp(0, layer_w);
                let y = y.clamp(0, layer_h);
                layer.get_tile(x, y).is_some()
            };

            self.occlusion_segments.clear();

            for x in 0..layer_w {
                for y in 0..layer_h {
                    if !is_solid(x, y) {
                        continue;
                    }

                    let empty_up = !is_solid(x, y - 1);
                    let empty_right = !is_solid(x + 1, y);
                    let empty_down = !is_solid(x, y + 1);
                    let empty_left = !is_solid(x - 1, y);

                    let (x, y) = (x as f32 - map_w2, (layer_h - 1 - y) as f32 - map_h2);

                    if empty_up {
                        self.occlusion_segments
                            .push(Segment::new((x, y + 1.0), (x + 1.0, y + 1.0)).unwrap());
                    }

                    if empty_right {
                        self.occlusion_segments
                            .push(Segment::new((x + 1.0, y), (x + 1.0, y + 1.0)).unwrap());
                    }

                    if empty_down {
                        self.occlusion_segments
                            .push(Segment::new((x, y), (x + 1.0, y)).unwrap());
                    }

                    if empty_left {
                        self.occlusion_segments
                            .push(Segment::new((x, y), (x, y + 1.0)).unwrap());
                    }
                }
            }
        }

        // Also add map edges

        let (map_w2, map_h2) = (map_w2 + 1.0, map_h2 + 1.0);

        // Top edge
        self.occlusion_segments
            .push(Segment::new((-map_w2, map_h2), (map_w2, map_h2)).unwrap());

        // Right edge
        self.occlusion_segments
            .push(Segment::new((map_w2, map_h2), (map_w2, -map_h2)).unwrap());

        // Bottom edge
        self.occlusion_segments
            .push(Segment::new((map_w2, -map_h2), (-map_w2, -map_h2)).unwrap());

        // Left edge
        self.occlusion_segments
            .push(Segment::new((-map_w2, -map_h2), (-map_w2, map_h2)).unwrap());

        Ok(())
    }

    pub fn visibility_for(&self, point: Point) -> VisibilityPolygon {
        VisibilityPolygon::compute(point, &self.occlusion_segments)
    }
}
