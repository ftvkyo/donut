use std::ops::Range;

use anyhow::{Context, Result, anyhow, bail, ensure};
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

            for quad in self.quads_for_layer(z, &layer) {
                quads.push(quad?);
            }
        }

        Ok(quads)
    }

    fn quads_for_layer(
        &self,
        z: f32,
        layer: &tiled::FiniteTileLayer<'_>,
    ) -> impl Iterator<Item = Result<Quad>> {
        let map_w = self.inner.width;
        let map_h = self.inner.height;
        // Shift all tiles to make (0.0, 0.0) be the map center
        // Quads are positioned by their centers
        let map_offset = vec2(map_w as f32, map_h as f32) / -2.0 + 0.5;

        let layer_w = layer.width();
        let layer_h = layer.height();

        (0..layer_w).flat_map(move |layer_x| {
            (0..layer_h).filter_map(move |layer_y| {
                if let Some(layer_tile) = layer.get_tile(layer_x as i32, layer_y as i32) {
                    if layer_tile.flip_d || layer_tile.flip_h || layer_tile.flip_v {
                        return Some(Err(anyhow!("Flipped tiles are not supported")));
                    }

                    let tileset = layer_tile.get_tileset();

                    let tile_id = layer_tile.id();
                    let tileset_x = (tile_id % tileset.columns) * tileset.tile_width;
                    let tileset_y = (tile_id / tileset.columns) * tileset.tile_height;

                    let pos_x = layer_x as f32;
                    let pos_y = (layer_h - 1 - layer_y) as f32;

                    let pos = vec3(pos_x, pos_y, z) + map_offset.extend(0.0);
                    let dim = vec2(1.0, 1.0);
                    let rot = 0.0;

                    let tex_num = self.tileset_map[layer_tile.tileset_index()] as u32;
                    let tex_pos = vec2(tileset_x as f32, tileset_y as f32);
                    let tex_dim = vec2(tileset.tile_width as f32, tileset.tile_height as f32);

                    Some(Ok(Quad {
                        pos,
                        dim,
                        rot,
                        tex_num,
                        tex_pos,
                        tex_dim,
                    }))
                } else {
                    None
                }
            })
        })
    }

    pub fn collision(&self) -> &[Segment] {
        &self.collision_segments
    }

    pub fn occlusion(&self) -> &[Segment] {
        &self.occlusion_segments
    }

    pub fn visibility_for(&self, point: Point) -> Vec<Segment> {
        compute_visibility(point, &self.occlusion_segments)
    }

    fn recalculate_collision_segments(&mut self) -> Result<()> {
        let colliding_layers = tile_layers(&self.inner, "Colliding")?;

        self.collision_segments =
            collision_segments(self.inner.width, self.inner.height, &colliding_layers);

        Ok(())
    }

    fn recalculate_occlusion_segments(&mut self) -> Result<()> {
        let occluding_layers = tile_layers(&self.inner, "Occluding")?;

        self.occlusion_segments =
            occlusion_segments(self.inner.width, self.inner.height, &occluding_layers);

        Ok(())
    }
}

/// Tile layers with the boolean `property` set, as functions telling whether a tile is solid.
fn tile_layers<'m>(
    map: &'m tiled::Map,
    property: &str,
) -> Result<Vec<impl Fn(i32, i32) -> bool + 'm>> {
    let mut layers = Vec::new();

    for layer in map.layers() {
        match layer.properties.get(property) {
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

        layers.push(move |x, y| layer.get_tile(x, y).is_some());
    }

    Ok(layers)
}

/// Segments between solid and empty tiles of the colliding layers.
/// A tile is solid if it is solid in any colliding layer, and tiles outside the map are empty.
/// The map is centered on (0, 0), its tiles are 1 unit wide, and `y = 0` is its top row.
pub fn collision_segments(
    width: u32,
    height: u32,
    colliding_layers: &[impl Fn(i32, i32) -> bool],
) -> Vec<Segment> {
    let (w, h) = (width as i32, height as i32);

    let is_solid = |x: i32, y: i32| {
        (0..w).contains(&x)
            && (0..h).contains(&y)
            && colliding_layers
                .iter()
                .any(|layer_is_solid| layer_is_solid(x, y))
    };

    tile_edges(width, height, 0..w, 0..h, is_solid)
}

/// Segments between solid and empty tiles of the occluding layers, with the map repeated
/// around itself so that light reaches across its edges, plus the edges of the repeated map.
/// A tile is solid if it is solid in any occluding layer.
/// The map is centered on (0, 0), its tiles are 1 unit wide, and `y = 0` is its top row.
pub fn occlusion_segments(
    width: u32,
    height: u32,
    occluding_layers: &[impl Fn(i32, i32) -> bool],
) -> Vec<Segment> {
    let (w, h) = (width as i32, height as i32);

    let is_solid = |x: i32, y: i32| {
        let x = x.rem_euclid(w);
        let y = y.rem_euclid(h);

        occluding_layers
            .iter()
            .any(|layer_is_solid| layer_is_solid(x, y))
    };

    let mut segments = tile_edges(width, height, -w..w * 2, -h..h * 2, is_solid);

    // Also add map edges

    let (map_w2, map_h2) = (width as f32 * 1.5 + 1.0, height as f32 * 1.5 + 1.0);

    // Top edge
    segments.push(Segment::new((-map_w2, map_h2), (map_w2, map_h2)).unwrap());

    // Right edge
    segments.push(Segment::new((map_w2, map_h2), (map_w2, -map_h2)).unwrap());

    // Bottom edge
    segments.push(Segment::new((map_w2, -map_h2), (-map_w2, -map_h2)).unwrap());

    // Left edge
    segments.push(Segment::new((-map_w2, -map_h2), (-map_w2, map_h2)).unwrap());

    segments
}

/// Edges between solid and empty tiles in `xs` by `ys` of a map of `width` by `height` tiles,
/// with aligned edges of neighbouring tiles joined into one segment.
fn tile_edges(
    width: u32,
    height: u32,
    xs: Range<i32>,
    ys: Range<i32>,
    is_solid: impl Fn(i32, i32) -> bool,
) -> Vec<Segment> {
    let mut out = Vec::new();

    let map_x = |x: i32| x as f32 - width as f32 / 2.0;
    let map_y = |y: i32| (height as i32 - y) as f32 - height as f32 / 2.0;

    // 1. Find all vertical segments

    for x in xs.clone() {
        let mut y_left = None;
        let mut y_right = None;

        for y in ys.start..=ys.end {
            let is_last = y == ys.end;
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

    for y in ys {
        let mut x_up = None;
        let mut x_down = None;

        for x in xs.start..=xs.end {
            let is_last = x == xs.end;
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

    out
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use proptest::{prelude::*, test_runner::RngSeed};

    use crate::testing::{TileGrid, tile_grid};

    use super::*;

    /// Ends of a unit edge, in half units so that they are integers.
    type UnitEdge = [(i32, i32); 2];

    fn unit_edge(a: (f32, f32), b: (f32, f32)) -> UnitEdge {
        let half_units = |(x, y): (f32, f32)| ((x * 2.0) as i32, (y * 2.0) as i32);
        let mut edge = [half_units(a), half_units(b)];
        edge.sort();
        edge
    }

    /// Unit edges between solid and empty tiles, with the same coordinates as `collision_segments`.
    fn unit_edges(grid: &TileGrid) -> BTreeSet<UnitEdge> {
        let (w2, h2) = (grid.width as f32 / 2.0, grid.height as f32 / 2.0);
        let mut edges = BTreeSet::new();

        for x in 0..grid.width as i32 {
            for y in 0..grid.height as i32 {
                if !grid.is_solid(x, y) {
                    continue;
                }

                let (left, bottom) = (x as f32 - w2, (grid.height as i32 - 1 - y) as f32 - h2);
                let (right, top) = (left + 1.0, bottom + 1.0);

                for (dx, dy, a, b) in [
                    (0, -1, (left, top), (right, top)),
                    (1, 0, (right, bottom), (right, top)),
                    (0, 1, (left, bottom), (right, bottom)),
                    (-1, 0, (left, bottom), (left, top)),
                ] {
                    if !grid.is_solid(x + dx, y + dy) {
                        edges.insert(unit_edge(a, b));
                    }
                }
            }
        }

        edges
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            failure_persistence: None,
            rng_seed: RngSeed::Fixed(0),
            ..ProptestConfig::default()
        })]

        #[test]
        fn collision_segments_join_unit_edges(grid in tile_grid(12, 0.4)) {
            let segments =
                collision_segments(grid.width, grid.height, &[|x, y| grid.is_solid(x, y)]);

            let mut joined = BTreeSet::new();
            for segment in &segments {
                let (a, b) = segment.ab();
                let units = a.dist(b).round() as i32;
                let step = a.dir(b) / units as f32;

                for i in 0..units {
                    let (a, b) = (a + step * i as f32, a + step * (i + 1) as f32);
                    prop_assert!(joined.insert(unit_edge(a.into(), b.into())), "Overlap at {segment}");
                }
            }

            prop_assert_eq!(joined, unit_edges(&grid));
        }
    }
}
