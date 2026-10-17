use std::f32::consts::FRAC_1_SQRT_2;

use glam::Vec2;

use crate::geo::{Point, Segment};

/// Segments near each point of an integer grid, so that the visibility up to `radius` from a point
/// can be computed from the few segments near the grid point closest to it.
pub struct SegmentBuckets {
    x_min: i32,
    y_min: i32,
    width: i32,
    height: i32,
    /// Column-major, starting from `(x_min, y_min)`
    buckets: Vec<Vec<Segment>>,
}

impl SegmentBuckets {
    /// Covers points from `min` to `max`.
    /// `segments` must not intersect except at their endpoints, and must include all segments
    /// within `radius` of the covered points.
    pub fn new(segments: &[Segment], min: Point, max: Point, radius: f32) -> Self {
        let (x_min, y_min) = (min.x.round() as i32, min.y.round() as i32);
        let (x_max, y_max) = (max.x.round() as i32, max.y.round() as i32);

        // Points that round to a grid point are within this distance of it
        let reach = radius + FRAC_1_SQRT_2;

        let mut buckets = Vec::new();

        for x in x_min..=x_max {
            for y in y_min..=y_max {
                let center = Point::new(x as f32, y as f32);

                let mut bucket: Vec<Segment> = segments
                    .iter()
                    .filter(|s| s.dist(center) <= reach)
                    .cloned()
                    .collect();

                // A box around the bucket stops the rays that hit nothing else.
                // It surrounds all the segments, so that it does not intersect any of them.

                let (mut lo, mut hi) = (center - Vec2::splat(reach), center + Vec2::splat(reach));
                for s in &bucket {
                    let (a, b) = s.ab();
                    lo = Point::new(lo.x.min(a.x).min(b.x), lo.y.min(a.y).min(b.y));
                    hi = Point::new(hi.x.max(a.x).max(b.x), hi.y.max(a.y).max(b.y));
                }
                let (lo, hi) = (lo - Vec2::ONE, hi + Vec2::ONE);

                let corners = [(lo.x, lo.y), (hi.x, lo.y), (hi.x, hi.y), (lo.x, hi.y)];
                for i in 0..corners.len() {
                    let next = (i + 1) % corners.len();
                    bucket.push(Segment::new(corners[i], corners[next]).unwrap());
                }

                buckets.push(bucket);
            }
        }

        Self {
            x_min,
            y_min,
            width: x_max - x_min + 1,
            height: y_max - y_min + 1,
            buckets,
        }
    }

    /// Segments that can be seen within `radius` of `point`, or `None` if it is not covered.
    pub fn get(&self, point: Point) -> Option<&[Segment]> {
        let x = point.x.round() as i32 - self.x_min;
        let y = point.y.round() as i32 - self.y_min;

        if !(0..self.width).contains(&x) || !(0..self.height).contains(&y) {
            return None;
        }

        Some(&self.buckets[(x * self.height + y) as usize])
    }
}
