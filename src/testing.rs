//! Shared helpers for tests: random tile maps, a brute-force ray caster,
//! and a check of visibility polygons against it.

use std::{any::Any, f64::consts::TAU, fmt::Display};

use glam::DVec2;
use proptest::prelude::*;

use crate::{
    assets::collision_segments,
    geo::{Point, Segment, compute_visibility},
};

#[derive(Clone, Debug)]
pub struct TileGrid {
    pub width: u32,
    pub height: u32,
    /// Row-major, `y = 0` is the top row.
    pub solid: Vec<bool>,
}

impl TileGrid {
    /// A grid from rows of `#` for solid and `.` for empty tiles, top row first.
    pub fn from_rows(rows: &[&str]) -> Self {
        Self {
            width: rows[0].len() as u32,
            height: rows.len() as u32,
            solid: rows
                .iter()
                .flat_map(|row| row.chars().map(|c| c == '#'))
                .collect(),
        }
    }

    pub fn is_solid(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return false;
        }

        self.solid[y as usize * self.width as usize + x as usize]
    }

    /// Whether `p` in map space is inside a solid tile.
    pub fn is_solid_at(&self, p: Point) -> bool {
        let x = (p.x + self.width as f32 / 2.0).floor() as i32;
        let y = self.height as i32 - 1 - (p.y + self.height as f32 / 2.0).floor() as i32;
        self.is_solid(x, y)
    }

    /// Tile edges, built the same way as the collision segments of a map loaded from a file,
    /// plus edges one unit around the map.
    pub fn segments(&self) -> Vec<Segment> {
        let mut segments =
            collision_segments(self.width, self.height, &[|x, y| self.is_solid(x, y)]);

        let x = self.width as f32 / 2.0 + 1.0;
        let y = self.height as f32 / 2.0 + 1.0;
        let corners = [(-x, y), (x, y), (x, -y), (-x, -y)];

        for i in 0..corners.len() {
            let next = (i + 1) % corners.len();
            segments.push(Segment::new(corners[i], corners[next]).unwrap());
        }

        segments
    }

    /// Grid corner `(x, y)` in map space, the top-left corner of the tile at `(x, y)`.
    pub fn corner(&self, x: u32, y: u32) -> Point {
        Point::new(
            x as f32 - self.width as f32 / 2.0,
            self.height as f32 / 2.0 - y as f32,
        )
    }

    /// Corner of the tile at `(x, y)` with the smallest coordinates, in map space.
    pub fn tile_origin(&self, x: u32, y: u32) -> Point {
        Point::new(
            x as f32 - self.width as f32 / 2.0,
            (self.height - 1 - y) as f32 - self.height as f32 / 2.0,
        )
    }
}

impl Display for TileGrid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for row in self.solid.chunks(self.width as usize) {
            let row: String = row
                .iter()
                .map(|&solid| if solid { '#' } else { '.' })
                .collect();
            writeln!(f, "{row}")?;
        }

        Ok(())
    }
}

/// Random tile grids of up to `max_size` tiles per side, with roughly `density` of the tiles solid.
pub fn tile_grid(max_size: u32, density: f64) -> impl Strategy<Value = TileGrid> {
    (1..=max_size, 1..=max_size).prop_flat_map(move |(width, height)| {
        let len = (width * height) as usize;
        proptest::collection::vec(proptest::bool::weighted(density), len).prop_map(move |solid| {
            TileGrid {
                width,
                height,
                solid,
            }
        })
    })
}

/// The message of a panic caught by `std::panic::catch_unwind`.
pub fn panic_message(payload: Box<dyn Any + Send>) -> String {
    match payload.downcast::<String>() {
        Ok(message) => *message,
        Err(payload) => payload
            .downcast::<&str>()
            .map_or_else(|_| String::new(), |message| message.to_string()),
    }
}

/// Distance along the ray from `origin` in direction `dir` to the nearest of `segments`,
/// and the index of that segment.
/// Computed in `f64` with no tolerances, to serve as a reference.
pub fn ray_cast(origin: Point, dir: DVec2, segments: &[Segment]) -> Option<(f64, usize)> {
    let q = dvec(origin);
    let dir = dir.normalize();

    let mut nearest: Option<(f64, usize)> = None;

    for (i, segment) in segments.iter().enumerate() {
        let (a, b) = segment.ab();
        let (a, b) = (dvec(a), dvec(b));

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

/// Checks the visibility polygon from `q` over `input` against `ray_cast` over `input`:
/// every output segment must lie along input segments, and every sampled ray must hit
/// exactly one output segment, passing by the point where it hits `input` first.
/// Points are compared within `tolerance`, and rays are sampled where moving any segment end
/// by up to `tolerance` cannot change which segments they hit:
/// between the segment ends as seen from `q`, and in those of the `extra` directions (radians).
pub fn check_visibility(
    q: Point,
    input: &[Segment],
    extra: &[f64],
    tolerance: f64,
) -> Result<(), TestCaseError> {
    let output = compute_visibility(q, input);

    for s in &output {
        prop_assert!(
            is_along(s, input, tolerance),
            "Stray output segment: {s} does not lie along input segments"
        );
    }

    let directions = sample_directions(q, input.iter().chain(&output), extra, tolerance);

    for angle in directions {
        let dir = DVec2::from_angle(angle);

        let expected = ray_cast(q, dir, input);
        let hits: Vec<_> = output
            .iter()
            .filter_map(|s| ray_cast(q, dir, std::slice::from_ref(s)).map(|(t, _)| (t, s)))
            .collect();

        match (expected, hits.as_slice()) {
            (None, []) => (),
            (Some((t, i)), [(t_out, s)]) => prop_assert!(
                distance_to_segment(dvec(q) + dir * t, s) <= tolerance,
                "Wrong output segment: at angle {angle}, expected {} at distance {t}, got {s} at distance {t_out}",
                input[i],
            ),
            (Some((t, i)), []) => prop_assert!(
                false,
                "Gap: at angle {angle}, expected {} at distance {t}, got nothing",
                input[i],
            ),
            (None, [(t_out, s)]) => prop_assert!(
                false,
                "Extra output segment: at angle {angle}, expected nothing, got {s} at distance {t_out}",
            ),
            (_, hits) => {
                let hits: Vec<_> = hits
                    .iter()
                    .map(|(t, s)| format!("{s} at distance {t}"))
                    .collect();
                prop_assert!(
                    false,
                    "Overlapping output segments: at angle {angle}, got {hits:?}"
                );
            }
        }
    }

    Ok(())
}

/// Whether every point of `s` is within `tolerance` of `segments`,
/// walking from one end of `s` to the other through the segments that lie along it.
fn is_along(s: &Segment, segments: &[Segment], tolerance: f64) -> bool {
    let (a, b) = s.ab();
    let (a, b) = (dvec(a), dvec(b));
    let length = a.distance(b);
    let dir = (b - a) / length;

    let mut reached = 0.0;

    while reached < length - tolerance {
        let p = a + dir * reached;

        let next = segments
            .iter()
            .filter(|segment| distance_to_segment(p, segment) <= tolerance)
            .filter_map(|segment| {
                let (c, d) = segment.ab();
                let far = (dvec(c) - a)
                    .dot(dir)
                    .max((dvec(d) - a).dot(dir))
                    .min(length);
                let is_along = distance_to_segment(a + dir * far, segment) <= tolerance;
                (far > reached && is_along).then_some(far)
            })
            .max_by(f64::total_cmp);

        match next {
            Some(far) => reached = far,
            None => return false,
        }
    }

    true
}

pub fn dvec(p: Point) -> DVec2 {
    DVec2::new(p.x as f64, p.y as f64)
}

pub fn distance_to_segment(p: DVec2, segment: &Segment) -> f64 {
    let (a, b) = segment.ab();
    let (a, b) = (dvec(a), dvec(b));
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

/// Directions from `q` (radians in `0..TAU`) that stay further than `tolerance` from the ends
/// of `segments`: the middle of each free angular range, and those of `extra` that are free.
pub fn sample_directions<'s>(
    q: Point,
    segments: impl Iterator<Item = &'s Segment>,
    extra: &[f64],
    tolerance: f64,
) -> Vec<f64> {
    let q = dvec(q);

    let mut blocked = Vec::new();

    for segment in segments {
        let (a, b) = segment.ab();

        for p in [dvec(a), dvec(b)] {
            let r = q.distance(p);

            if r <= tolerance {
                return Vec::new();
            }

            let angle = (p - q).to_angle().rem_euclid(TAU);
            let half_width = (tolerance / r).asin();

            for turn in [-TAU, 0.0, TAU] {
                blocked.push((angle + turn - half_width, angle + turn + half_width));
            }
        }
    }

    blocked.sort_by(|a, b| a.0.total_cmp(&b.0));

    let mut merged: Vec<(f64, f64)> = Vec::new();

    for (lo, hi) in blocked {
        match merged.last_mut() {
            Some(last) if lo <= last.1 => last.1 = last.1.max(hi),
            _ => merged.push((lo, hi)),
        }
    }

    let mut directions: Vec<f64> = merged
        .windows(2)
        .map(|w| (w[0].1 + w[1].0) / 2.0)
        .filter(|angle| (0.0..TAU).contains(angle))
        .collect();

    let is_free = |angle: &f64| merged.iter().all(|(lo, hi)| angle < lo || angle > hi);
    directions.extend(extra.iter().copied().filter(is_free));

    directions
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
        let grid = TileGrid::from_rows(&["...", ".#.", "..."]);

        // 4 tile edges plus 4 map edges
        assert_eq!(grid.segments().len(), 8);
        assert_eq!(grid.tile_origin(1, 1), Point::new(-0.5, -0.5));
        assert_eq!(grid.corner(1, 2), Point::new(-0.5, -0.5));
        assert_eq!(grid.to_string(), "...\n.#.\n...\n");
        assert!(grid.is_solid_at(Point::new(0.25, -0.25)));
        assert!(!grid.is_solid_at(Point::new(0.75, -0.25)));
    }

    #[test]
    fn tile_grid_segments_mirrored() {
        let left = TileGrid::from_rows(&["#."]).segments().len();
        let right = TileGrid::from_rows(&[".#"]).segments().len();
        assert_eq!(left, right);
    }
}
