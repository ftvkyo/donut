use std::{
    cell::Cell,
    fmt::{Debug, Display},
};

use glam::{DVec2, Vec2};
use log::trace;

use crate::geo::ImpreciseEq;

use super::point::Point;

#[derive(Debug, PartialEq, Eq)]
pub enum SegmentSide {
    Left,
    Right,
}

impl Display for SegmentSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SegmentSide::Left => write!(f, "left"),
            SegmentSide::Right => write!(f, "right"),
        }
    }
}

#[derive(Clone)]
pub struct Segment {
    a: Point,
    b: Point,
}

impl Segment {
    pub fn new<P: Into<Point>>(a: P, b: P) -> Option<Self> {
        let a = a.into();
        let b = b.into();

        if a.is_basically_equal(&b) {
            return None;
        }

        Some(Self { a, b })
    }

    pub fn ab(&self) -> (Point, Point) {
        (self.a, self.b)
    }

    /// Distance from the line defined by the segment, positive on its left.
    pub fn signed_dist(&self, point: Point) -> f32 {
        let ab = self.a.dir(self.b);
        ab.perp_dot(self.a.dir(point)) / ab.length()
    }

    pub fn dist(&self, point: Point) -> f32 {
        let ab = self.a.dir(self.b);
        let t = (self.a.dir(point).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
        point.dist(self.a + ab * t)
    }

    /// Where the line defined by the ray crosses the line defined by the segment,
    /// moved onto the segment if it misses it.
    pub fn intersect_with_ray(&self, origin: Point, direction: Vec2) -> Option<Point> {
        let ab = self.a.dir(self.b);
        let denominator = direction.perp_dot(ab);

        // The sine of the angle between the lines
        let sine = denominator / (direction.length() * ab.length());

        if sine.is_nan() || sine.is_basically_zero() {
            // Lines are parallel or coincident
            return None;
        }

        let u = origin.dir(self.a).perp_dot(direction) / denominator;

        return Some(self.a + ab * u.clamp(0.0, 1.0));
    }

    pub fn which_side(&self, point: Point) -> Option<SegmentSide> {
        if point.is_basically_equal(&self.a) {
            trace!("{point} is on the start of {self}");
            return None;
        }

        if point.is_basically_equal(&self.b) {
            trace!("{point} is on the end of {self}");
            return None;
        }

        assert!(!self.a.is_basically_equal(&self.b));

        let dir_segment = self.a.dir(self.b);
        let dir_point = self.a.dir(point);

        // Signed distance from the line defined by the segment, times its length
        let val = dir_segment.perp_dot(dir_point);

        if val * val <= f32::E * f32::E * dir_segment.length_squared() {
            return None;
        }

        if val > 0.0 {
            return Some(SegmentSide::Left);
        } else {
            return Some(SegmentSide::Right);
        }
    }
}

impl PartialEq for Segment {
    fn eq(&self, other: &Self) -> bool {
        (self.a == other.a && self.b == other.b) || (self.a == other.b && self.b == other.a)
    }
}

impl Display for Segment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}<=>{}", self.a, self.b,)
    }
}

/// A ray from the origin of a sweep, and the side of it on which ties are broken.
#[derive(Clone, Copy, Debug)]
pub struct SweepRay {
    pub dir: DVec2,
    /// 1.0 to break ties just counter-clockwise of the ray, -1.0 just clockwise of it
    pub side: f64,
}

impl SweepRay {
    /// Angle between the ray and the rays on its sides
    const TIE_ANGLE: f64 = 1e-6;

    /// Distances closer than this, relative to them, are a tie
    const TIE_DISTANCE: f64 = 1e-9;
}

/// Orders segments by the distance from `origin` along the current `ray` to them.
/// Segments that tie, such as those meeting where the ray passes, are ordered by the distance
/// along a ray just to the side of it, and then by `index`.
/// Note: segments must not intersect except at their endpoints, and must all cross the ray.
pub struct SegmentByDistance<'o, 's, 'r> {
    pub origin: &'o Point,
    pub segment: &'s Segment,
    pub index: usize,
    pub ray: &'r Cell<SweepRay>,
}

impl<'o, 's, 'r> SegmentByDistance<'o, 's, 'r> {
    pub fn new(
        origin: &'o Point,
        segment: &'s Segment,
        index: usize,
        ray: &'r Cell<SweepRay>,
    ) -> Self {
        Self {
            origin,
            segment,
            index,
            ray,
        }
    }

    /// Distance from the origin along `dir` to the line defined by the segment, in units of `dir`,
    /// or infinity if `on_segment` and the ray misses the segment itself.
    fn distance_along(&self, dir: DVec2, on_segment: bool) -> f64 {
        let q = self.origin.dvec();
        let (a, b) = (self.segment.a.dvec(), self.segment.b.dvec());
        let ab = b - a;

        let denominator = dir.perp_dot(ab);
        let t = (a - q).perp_dot(ab) / denominator;
        let u = (a - q).perp_dot(dir) / denominator;

        if on_segment && (t < 0.0 || !(0.0..=1.0).contains(&u)) {
            return f64::INFINITY;
        }

        t
    }

    fn cmp_along(&self, other: &Self, dir: DVec2, on_segment: bool) -> std::cmp::Ordering {
        let t1 = self.distance_along(dir, on_segment);
        let t2 = other.distance_along(dir, on_segment);

        let tolerance = SweepRay::TIE_DISTANCE * t1.abs().max(t2.abs());
        if t1 == t2 || (tolerance.is_finite() && (t1 - t2).abs() <= tolerance) {
            return std::cmp::Ordering::Equal;
        }

        t1.partial_cmp(&t2).unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl PartialOrd for SegmentByDistance<'_, '_, '_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SegmentByDistance<'_, '_, '_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if !std::ptr::eq(self.origin, other.origin) || !std::ptr::eq(self.ray, other.ray) {
            panic!(
                "Tried to compare segments by distance from different origins or along different rays"
            );
        }

        if std::ptr::eq(self.segment, other.segment) {
            return std::cmp::Ordering::Equal;
        }

        let ray = self.ray.get();
        let dir_aside = ray.dir + ray.dir.perp() * ray.side * SweepRay::TIE_ANGLE;

        let order = self
            .cmp_along(other, ray.dir, false)
            .then_with(|| self.cmp_along(other, dir_aside, true))
            .then(self.index.cmp(&other.index));

        trace!(
            "Compared S1={} & S2={} along {:?}: {order:?}",
            self.segment, other.segment, ray
        );

        order
    }
}

impl PartialEq for SegmentByDistance<'_, '_, '_> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}

impl Eq for SegmentByDistance<'_, '_, '_> {}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use crate::init_logging;

    use super::*;

    /// Compares `s1` and `s2` along the ray from `q` through `through`.
    fn cmp(q: Point, through: (f32, f32), side: f64, s1: &Segment, s2: &Segment) -> Ordering {
        init_logging();

        let dir = Point::from(through).dvec() - q.dvec();
        let ray = Cell::new(SweepRay { dir, side });

        let order =
            SegmentByDistance::new(&q, s1, 1, &ray).cmp(&SegmentByDistance::new(&q, s2, 2, &ray));
        let reverse =
            SegmentByDistance::new(&q, s2, 2, &ray).cmp(&SegmentByDistance::new(&q, s1, 1, &ray));
        assert_eq!(order, reverse.reverse(), "S1={s1}, S2={s2}");

        order
    }

    fn seg(a: (f32, f32), b: (f32, f32)) -> Segment {
        Segment::new(a, b).unwrap()
    }

    #[test]
    fn segment_ord() {
        let q = Point::ZERO;

        // Parallel, one behind the other
        let near = seg((-2.0, 2.0), (2.0, 2.0));
        let far = seg((-2.0, 3.0), (2.0, 3.0));
        assert_eq!(cmp(q, (0.5, 1.0), 1.0, &near, &far), Ordering::Less);

        // Slanted, partly overlapping as seen from Q
        let s1 = seg((1.0, 4.0), (3.0, 7.0));
        let s2 = seg((0.0, 8.0), (2.0, 6.0));
        assert_eq!(cmp(q, (1.0, 5.0), 1.0, &s1, &s2), Ordering::Less);

        let s1 = seg((2.0, 4.0), (4.0, 6.0));
        assert_eq!(cmp(q, (3.0, 5.0), 1.0, &s1, &s2), Ordering::Less);
    }

    #[test]
    fn segment_ord_weird() {
        let s1 = seg((-2.0, 2.0), (-2.0, 0.0));
        let s2 = seg((-4.0, 0.0), (-5.0, 0.0));
        let q = Point::new(0.0, 1.0);

        assert_eq!(cmp(q, (-2.0, 0.5), 1.0, &s1, &s2), Ordering::Less);
    }

    #[test]
    fn segments_meeting_on_the_ray() {
        let q = Point::ZERO;

        // A '^' shape with its apex on the ray: just counter-clockwise only the left segment is
        // crossed, and just clockwise only the right one
        let left = seg((-2.0, 6.0), (0.0, 8.0));
        let right = seg((0.0, 8.0), (2.0, 6.0));
        assert_eq!(cmp(q, (0.0, 8.0), 1.0, &left, &right), Ordering::Less);
        assert_eq!(cmp(q, (0.0, 8.0), -1.0, &left, &right), Ordering::Greater);

        // Touching sideways
        let s1 = seg((-2.0, 2.0), (-2.0, 0.0));
        let s2 = seg((-2.0, 2.0), (-3.0, 2.0));
        let q = Point::new(0.0, 1.0);
        assert_eq!(cmp(q, (-2.0, 2.0), 1.0, &s1, &s2), Ordering::Less);
    }

    #[test]
    fn collinear_segments_meeting_on_the_ray() {
        // Both lines pass through the meeting point, so the index decides
        let s1 = seg((-2.0, 2.0), (0.0, 2.0));
        let s2 = seg((0.0, 2.0), (2.0, 2.0));
        assert_eq!(cmp(Point::ZERO, (0.0, 2.0), 1.0, &s1, &s2), Ordering::Less);
    }
}
