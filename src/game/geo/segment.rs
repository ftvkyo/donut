use std::{
    f32::consts::PI,
    fmt::{Debug, Display},
};

use glam::Vec2;
use log::trace;

use crate::game::geo::ERR;

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

pub struct Segment {
    a: Point,
    b: Point,
}

impl Segment {
    pub fn new<P: Into<Point>>(a: P, b: P) -> Option<Self> {
        let a = a.into();
        let b = b.into();

        if a.distance_squared(*b) < ERR {
            return None;
        }

        Some(Self { a, b })
    }

    pub fn ab(&self) -> (Point, Point) {
        (self.a, self.b)
    }

    pub fn intersect_with_ray(&self, origin: Point, direction: Vec2) -> Option<Point> {
        let (x1, y1) = self.a.into();
        let (x2, y2) = self.b.into();
        let (x3, y3) = origin.into();
        let (x4, y4) = (origin + direction.into()).into();

        let denominator = (x1 - x2) * (y3 - y4) - (y1 - y2) * (x3 - x4);

        if denominator.abs() < ERR {
            // Lines are parallel or coincident
            return None;
        }

        let x = ((x1 * y2 - y1 * x2) * (x3 - x4) - (x1 - x2) * (x3 * y4 - y3 * x4)) / denominator;
        let y = ((x1 * y2 - y1 * x2) * (y3 - y4) - (y1 - y2) * (x3 * y4 - y3 * x4)) / denominator;

        return Some(Point::new(x, y));
    }

    pub fn which_side(&self, point: Point) -> Option<SegmentSide> {
        if point.dir(self.a).length() < ERR {
            trace!("{point} is on the start of {self}");
            return None;
        }

        if point.dir(self.b).length() < ERR {
            trace!("{point} is on the end of {self}");
            return None;
        }

        // TODO: more efficient?

        let dir_point = self.a.dir(point);
        let dir_segment = self.a.dir(self.b);
        assert!(dir_segment.length() > ERR);

        let angle = dir_segment.angle_to(dir_point);

        if angle.abs() < ERR || (angle.abs() - PI).abs() < ERR {
            trace!("{point} is on the line {self}");
            return None;
        }

        if angle > 0.0 {
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

/// Note: segments must not intersect except at their endpoints.
/// Note: it should be possible to draw a straight line from `origin` through both segments that are being compared.
pub struct SegmentByDistance<'o, 's> {
    pub origin: &'o Point,
    pub segment: &'s Segment,
}

impl<'o, 's> SegmentByDistance<'o, 's> {
    pub fn new(origin: &'o Point, segment: &'s Segment) -> Self {
        Self { origin, segment }
    }
}

impl PartialOrd for SegmentByDistance<'_, '_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SegmentByDistance<'_, '_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering as O;

        if !std::ptr::eq(self.origin, other.origin) {
            panic!("Tried to compare segments by distance to different origins");
        }

        let q = *self.origin;
        let s1 = self.segment;
        let s2 = other.segment;

        trace!("Comparing S1={s1} & S2={s2} with Q={q}");

        if std::ptr::eq(self.segment, other.segment) {
            trace!(" -> S1 and S2 are the same segment");
            return O::Equal;
        }

        let s2a_to_s1 = s1.which_side(s2.a);
        let s2b_to_s1 = s1.which_side(s2.b);

        let s1a_to_s2 = s2.which_side(s1.a);
        let s1b_to_s2 = s2.which_side(s1.b);

        // The core assumption for the comparison logic is that it should be possible to draw
        // a line that goes through q and intersects both of the segments.

        match (s1.which_side(q), s2.which_side(q)) {
            (None, None) => {
                // `q` IS on the line defined by `s1`.
                // `q` IS on the line defined by `s2`.
                // Therefore the closest segment is the one that contains the closest point.
                trace!(" -> Q is on the lines S1 & S2");
                let dist2_s1 = q.distance_squared(*s1.a).min(q.distance_squared(*s1.b));
                let dist2_s2 = q.distance_squared(*s2.a).min(q.distance_squared(*s2.b));
                if dist2_s1 < dist2_s2 {
                    return O::Less;
                } else if dist2_s1 > dist2_s2 {
                    return O::Greater;
                } else {
                    panic!("Tried to compare segments that intersect");
                }
            }
            (Some(q_to_s1), None) => {
                // `q` IS NOT on the line defined by `s1`.
                // `q` IS on the line defined by `s2`.
                // `s2.a` & `s2.b` would be on the same side of `s1`.
                trace!(" -> Q is on the line S2");
                match (s2a_to_s1, s2b_to_s1) {
                    (Some(s2_to_s1), _) | (_, Some(s2_to_s1)) => {
                        if s2_to_s1 == q_to_s1 {
                            return O::Greater;
                        } else {
                            return O::Less;
                        }
                    }
                    _ => {
                        // If `q` is not on the line defined by `s1`,
                        // `q` is on the line defined by `s2`,
                        // and `s2` is on neither side of `s1`,
                        // both points of `s2` must lay on `s1` and therefore its length is zero.
                        unreachable!("Got a segment of length zero");
                    }
                }
            }
            (None, Some(q_to_s2)) => {
                // `q` IS on the line defined by `s1`.
                // `q` IS NOT on the line defined by `s2`.
                // `s1.a` & `s1.b` would be on the same side of `s2`.
                trace!(" -> Q is on the line S1");
                match (s1a_to_s2, s1b_to_s2) {
                    (Some(s1_to_s2), _) | (_, Some(s1_to_s2)) => {
                        if s1_to_s2 == q_to_s2 {
                            return O::Less;
                        } else {
                            return O::Greater;
                        }
                    }
                    _ => {
                        // If `q` is on the line defined by `s1`,
                        // `q` is not on the line defined by `s2`,
                        // and `s1` is on neither side of `s2`,
                        // both points of `s1` must lay on `s2` and therefore its length is zero.
                        unreachable!("Got a segment of length zero");
                    }
                }
            }
            (Some(q_to_s1), Some(q_to_s2)) => {
                trace!(" -> Q is on the {q_to_s1} of S1");
                trace!(" -> Q is on the {q_to_s2} of S2");
                match (s1a_to_s2, s1b_to_s2) {
                    (Some(s1a_to_s2), Some(s1b_to_s2)) => {
                        trace!(" -> S1a is on the {s1a_to_s2} of S2");
                        trace!(" -> S1b is on the {s1b_to_s2} of S2");

                        if q_to_s2 == s1a_to_s2 && q_to_s2 == s1b_to_s2 {
                            return O::Less;
                        }

                        if s1a_to_s2 == s1b_to_s2 {
                            return O::Greater;
                        }

                        match (s2a_to_s1, s2b_to_s1) {
                            (Some(s2a_to_s1), Some(s2b_to_s1)) => {
                                trace!(" -> S2a is on the {s2a_to_s1} of S1");
                                trace!(" -> S2b is on the {s2b_to_s1} of S1");

                                if q_to_s1 == s2a_to_s1 && q_to_s1 == s2a_to_s1 {
                                    return O::Greater;
                                }

                                return O::Less;
                            }
                            (Some(s2_to_s1), _) | (_, Some(s2_to_s1)) => {
                                trace!(" -> one of the points of S2 is on the line defined by S1"); // (but not the other)
                                if s2_to_s1 == q_to_s1 {
                                    return O::Greater;
                                } else {
                                    return O::Less;
                                }
                            }
                            _ => unreachable!(
                                "S2 can't be on the same line as S1, as S1 is already not on the same line as S2"
                            ),
                        }
                    }
                    (Some(s1_to_s2), _) | (_, Some(s1_to_s2)) => {
                        trace!(" -> one of the points of S1 is on the line defined by S2"); // (but not the other)

                        match (s2a_to_s1, s2b_to_s1) {
                            (Some(s2a_to_s1), Some(_)) => {
                                trace!(" -> no point of S2 is on the line defined by S1");
                                if s2a_to_s1 == q_to_s1 {
                                    trace!(" -> S2 & Q are on the same side of S1");
                                    return O::Greater;
                                } else {
                                    trace!(" -> S2 & Q are on different sides of S1");
                                    return O::Less;
                                }
                            }
                            (Some(s2_to_s1), _) | (_, Some(s2_to_s1)) => {
                                trace!(" -> one of the points of S2 is on the line defined by S1"); // (but not the other)
                                trace!(" -> ends of S1 & S2 are touching");

                                if s1_to_s2 == q_to_s2 {
                                    if s2_to_s1 == q_to_s1 {
                                        trace!(
                                            " -> S1 & S2 form a '> Q' or a '< Q' shape and are therefore equal"
                                        );
                                    } else {
                                        trace!(
                                            " -> S1 & S2 form a '<' or '>' shape and S1 is closer"
                                        );
                                        return O::Less;
                                    }
                                } else {
                                    trace!(" -> S1 & S2 form a '<' or '>' shape and S2 is closer");
                                    return O::Greater;
                                }

                                return O::Equal;
                            }
                            _ => {
                                unreachable!(
                                    "S2 can't be on the same line as S1, as S1 is already is not on the same line as S2"
                                )
                            }
                        }
                    }
                    _ => {
                        trace!(" -> S1 & S2 are on the same line and their ends are touching");
                        return O::Equal;
                    }
                }
            }
        }
    }
}

impl PartialEq for SegmentByDistance<'_, '_> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}

impl Eq for SegmentByDistance<'_, '_> {}

#[cfg(test)]
mod tests {
    use crate::init_logging;

    use super::*;

    const O: Point = Point::new(0.0, 0.0);

    macro_rules! seg {
        ($x1:expr, $y1:expr, $x2:expr, $y2:expr) => {
            SegmentByDistance {
                origin: &O,
                segment: Box::leak(Box::new(Segment {
                    a: Point::new($x1 as f32, $y1 as f32),
                    b: Point::new($x2 as f32, $y2 as f32),
                })),
            }
        };
    }

    #[test]
    fn segment_ord() {
        init_logging();

        let s = [
            seg!(-2, 2, 0, 2),
            seg!(0, 2, 2, 2),
            seg!(-2, 3, 2, 3),
            seg!(-3, 6, -1, 4),
            seg!(0, 4, 0, 6),
            seg!(1, 4, 3, 7),
            seg!(-2, 6, 0, 8),
            seg!(0, 8, 2, 6),
            seg!(0, 9, 0, 11),
            seg!(2, 4, 4, 6),
        ];

        let expect_equal = |i1: usize, i2: usize| {
            assert!(
                s[i1] == s[i2],
                "Expected equality:\ns{i1}={}\ns{i2}={}",
                s[i1].segment,
                s[i2].segment
            );
            assert!(
                s[i2] == s[i1],
                "Expected equality:\ns{i2}={}\ns{i1}={}",
                s[i2].segment,
                s[i1].segment
            );
        };

        let expect_less = |i1: usize, i2: usize| {
            assert!(
                s[i1] < s[i2],
                "Expected left < right:\ns{i1}={}\ns{i2}={}",
                s[i1].segment,
                s[i2].segment
            );
            assert!(
                s[i2] > s[i1],
                "Expected left > right:\ns{i2}={}\ns{i1}={}",
                s[i2].segment,
                s[i1].segment
            );
        };

        for i in 0..s.len() {
            expect_equal(i, i);
        }

        expect_equal(0, 1);
        expect_equal(6, 7);

        expect_less(0, 2);
        expect_less(0, 3);
        expect_less(0, 4);
        expect_less(0, 6);
        expect_less(0, 7);

        expect_less(1, 2);
        expect_less(1, 4);
        expect_less(1, 5);
        expect_less(1, 6);
        expect_less(1, 7);

        expect_less(3, 6);

        expect_less(5, 7);

        expect_less(4, 8);

        expect_less(9, 5);
    }

    #[test]
    fn segment_ord_weird() {
        init_logging();

        let s1 = Segment {
            a: Point::new(-2.0, 2.0),
            b: Point::new(-2.0, 0.0),
        };
        let s2 = Segment {
            a: Point::new(-4.0, 0.0),
            b: Point::new(-5.0, 0.0),
        };
        let origin = Point::new(0.0, 1.0);

        let sd1 = SegmentByDistance {
            origin: &origin,
            segment: &s1,
        };
        let sd2 = SegmentByDistance {
            origin: &origin,
            segment: &s2,
        };

        assert!(sd1 < sd2, "\norigin={origin}\nsd1={s1}\nsd2={s2}");
        assert!(sd2 > sd1, "\norigin={origin}\nsd1={s1}\nsd2={s2}");
    }

    #[test]
    fn segment_ord_touching_sideways() {
        init_logging();

        let s1 = Segment {
            a: Point::new(-2.0, 2.0),
            b: Point::new(-2.0, 0.0),
        };
        let s2 = Segment {
            a: Point::new(-2.0, 2.0),
            b: Point::new(-3.0, 2.0),
        };
        let origin = Point::new(0.0, 1.0);

        let sd1 = SegmentByDistance {
            origin: &origin,
            segment: &s1,
        };
        let sd2 = SegmentByDistance {
            origin: &origin,
            segment: &s2,
        };

        assert!(sd1 < sd2, "\norigin={origin}\nsd1={s1}\nsd2={s2}");
        assert!(sd2 > sd1, "\norigin={origin}\nsd1={s1}\nsd2={s2}");
    }
}
