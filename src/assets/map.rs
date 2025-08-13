use anyhow::{Context, Result, bail, ensure};
use glam::{vec2, vec3};
use winit::dpi::LogicalSize;

use crate::{
    geo::{Point, Segment, compute_visibility},
    view::Quad,
};

pub struct Map {
    pub name: String,
    inner: tiled::Map,
    tileset_map: Vec<usize>,
    collision_segments: Vec<Segment>,
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
            collision_segments: Vec::new(),
            occlusion_segments: Vec::new(),
        };
        s.recalculate_collision_segments()?;
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

    pub fn collision(&self) -> &[Segment] {
        &self.collision_segments
    }

    pub fn visibility_for(&self, point: Point) -> Vec<Segment> {
        compute_visibility(point, &self.occlusion_segments)
    }

    fn recalculate_collision_segments(&mut self) -> Result<()> {
        let map_w2 = self.inner.width as f32 / 2.0;
        let map_h2 = self.inner.height as f32 / 2.0;

        self.collision_segments.clear();

        for layer in self.inner.layers() {
            match layer.properties.get("Colliding") {
                Some(tiled::PropertyValue::BoolValue(true)) => (),
                _ => continue,
            };

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

            let map_x = |x: i32| x as f32 - map_w2;
            let map_y = |y: i32| (layer_h - y) as f32 - map_h2;

            segments_in_range(
                0,
                layer_w,
                0,
                layer_h,
                map_x,
                map_y,
                is_solid,
                &mut self.collision_segments,
            );
        }

        Ok(())
    }

    fn recalculate_occlusion_segments(&mut self) -> Result<()> {
        let map_w2 = self.inner.width as f32 / 2.0;
        let map_h2 = self.inner.height as f32 / 2.0;

        self.occlusion_segments.clear();

        for layer in self.inner.layers() {
            match layer.properties.get("Occluding") {
                Some(tiled::PropertyValue::BoolValue(true)) => (),
                _ => continue,
            };

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
                let x = x.rem_euclid(layer_w);
                let y = y.rem_euclid(layer_h);

                return 0 <= x
                    && x <= layer_w
                    && 0 <= y
                    && y <= layer_h
                    && layer.get_tile(x, y).is_some();
            };

            let map_x = |x: i32| x as f32 - map_w2;
            let map_y = |y: i32| (layer_h - y) as f32 - map_h2;

            segments_in_range(
                -layer_w,
                layer_w * 2,
                -layer_h,
                layer_h * 2,
                map_x,
                map_y,
                is_solid,
                &mut self.occlusion_segments,
            );
        }

        // Also add map edges

        let (map_w2, map_h2) = (map_w2 * 3.0 + 1.0, map_h2 * 3.0 + 1.0);

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
}

fn segments_in_range(
    x_min: i32,
    x_max: i32,
    y_min: i32,
    y_max: i32,
    map_x: impl Fn(i32) -> f32,
    map_y: impl Fn(i32) -> f32,
    is_solid: impl Fn(i32, i32) -> bool,
    out: &mut Vec<Segment>,
) {
    // 1. Find all vertical segments

    for x in x_min..x_max {
        let mut y_left = None;
        let mut y_right = None;

        for y in y_min..=y_max {
            let is_last = y == y_max;
            let is_empty = !is_solid(x, y);
            let left_is_empty = !is_solid(x - 1, y);
            let right_is_empty = !is_solid(x + 1, y);

            if is_empty || is_last || !left_is_empty {
                // Commit left
                if let Some(y_left) = y_left.take() {
                    out.push(Segment::new((map_x(x), y_left), (map_x(x), map_y(y))).unwrap());
                }
            }

            if is_empty || is_last || !right_is_empty {
                // Commit right
                if let Some(y_right) = y_right.take() {
                    out.push(
                        Segment::new((map_x(x + 1), y_right), (map_x(x + 1), map_y(y))).unwrap(),
                    );
                }
            }

            if !is_empty && !is_last {
                if left_is_empty {
                    y_left.get_or_insert(map_y(y));
                }

                if right_is_empty {
                    y_right.get_or_insert(map_y(y));
                }
            }
        }
    }

    // 2. Find all horizontal segments

    for y in y_min..y_max {
        let mut x_up = None;
        let mut x_down = None;

        for x in x_min..=x_max {
            let is_last = x == x_max;
            let is_empty = !is_solid(x, y);
            let up_is_empty = !is_solid(x, y - 1);
            let down_is_empty = !is_solid(x, y + 1);

            if is_empty || is_last || !up_is_empty {
                // Commit up
                if let Some(x_up) = x_up.take() {
                    out.push(Segment::new((x_up, map_y(y)), (map_x(x), map_y(y))).unwrap());
                }
            }

            if is_empty || is_last || !down_is_empty {
                // Commit down
                if let Some(x_down) = x_down.take() {
                    out.push(
                        Segment::new((x_down, map_y(y + 1)), (map_x(x), map_y(y + 1))).unwrap(),
                    );
                }
            }

            if !is_empty && !is_last {
                if up_is_empty {
                    x_up.get_or_insert(map_x(x));
                }

                if down_is_empty {
                    x_down.get_or_insert(map_x(x));
                }
            }
        }
    }
}
