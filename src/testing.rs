//! Shared helpers for tests: random tile maps and a brute-force ray caster.

use glam::DVec2;
use proptest::prelude::*;

use crate::{
    assets::occlusion_segments,
    geo::{Point, Segment},
};

#[derive(Clone, Debug)]
pub struct TileGrid {
    pub width: u32,
    pub height: u32,
    /// Row-major, `y = 0` is the top row.
    pub solid: Vec<bool>,
}

impl TileGrid {
    pub fn is_solid(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return false;
        }

        self.solid[y as usize * self.width as usize + x as usize]
    }

    /// Occlusion segments, built the same way as for a map loaded from a file.
    pub fn segments(&self) -> Vec<Segment> {
        occlusion_segments(self.width, self.height, &[|x, y| self.is_solid(x, y)])
    }

    /// Corner of the tile at `(x, y)` with the smallest coordinates, in map space.
    pub fn tile_origin(&self, x: u32, y: u32) -> Point {
        Point::new(
            x as f32 - self.width as f32 / 2.0,
            (self.height - 1 - y) as f32 - self.height as f32 / 2.0,
        )
    }
}

/// Random tile grids of up to `max_size` tiles per side, with roughly `density` of the tiles solid.
pub fn tile_grid(max_size: u32, density: f64) -> impl Strategy<Value = TileGrid> {
    (1..=max_size, 1..=max_size).prop_flat_map(move |(width, height)| {
        let len = (width * height) as usize;
        proptest::collection::vec(proptest::bool::weighted(density), len).prop_map(
            move |solid| TileGrid {
                width,
                height,
                solid,
            },
        )
    })
}

/// Distance along the ray from `origin` in direction `dir` to the nearest of `segments`,
/// and the index of that segment.
/// Computed in `f64` with no tolerances, to serve as a reference.
pub fn ray_cast(origin: Point, dir: DVec2, segments: &[Segment]) -> Option<(f64, usize)> {
    let q = DVec2::new(origin.x as f64, origin.y as f64);
    let dir = dir.normalize();

    let mut nearest: Option<(f64, usize)> = None;

    for (i, segment) in segments.iter().enumerate() {
        let (a, b) = segment.ab();
        let a = DVec2::new(a.x as f64, a.y as f64);
        let b = DVec2::new(b.x as f64, b.y as f64);

        let ab = b - a;
        let denominator = dir.perp_dot(ab);

        if denominator == 0.0 {
            continue;
        }

        let qa = a - q;
        let t = qa.perp_dot(ab) / denominator;
        let u = qa.perp_dot(dir) / denominator;

        if t < 0.0 || !(0.0..=1.0).contains(&u) {
            continue;
        }

        if nearest.is_none_or(|(t_nearest, _)| t < t_nearest) {
            nearest = Some((t, i));
        }
    }

    nearest
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ray_cast_square() {
        let segments = [
            Segment::new((-1.0, 1.0), (1.0, 1.0)).unwrap(),
            Segment::new((1.0, 1.0), (1.0, -1.0)).unwrap(),
            Segment::new((-1.0, 3.0), (1.0, 3.0)).unwrap(),
        ];

        let (t, i) = ray_cast(Point::ZERO, DVec2::Y, &segments).unwrap();
        assert_eq!((t, i), (1.0, 0));

        let (t, i) = ray_cast(Point::ZERO, DVec2::X, &segments).unwrap();
        assert_eq!((t, i), (1.0, 1));

        assert!(ray_cast(Point::ZERO, DVec2::NEG_Y, &segments).is_none());
    }

    #[test]
    fn tile_grid_segments() {
        // A single solid tile in the middle of a 3x3 map
        let grid = TileGrid {
            width: 3,
            height: 3,
            solid: vec![false, false, false, false, true, false, false, false, false],
        };

        // 4 tile edges plus 4 map edges
        assert_eq!(grid.segments().len(), 8);
        assert_eq!(grid.tile_origin(1, 1), Point::new(-0.5, -0.5));
    }
}
