//! Property tests for `SegmentByDistance` and the `Segment` primitives it relies on.
//!
//! Long-running variant, with any number of cases:
//! `PROPTEST_CASES=100000 cargo test --release --lib geo::fuzz_segment::long -- --ignored`

use std::{cmp::Ordering, collections::BTreeSet, f32::consts::PI, ops::Range};

use glam::{DVec2, Vec2};
use proptest::{collection::vec, prelude::*, sample::select, test_runner::RngSeed};

use crate::testing::{TileGrid, distance_to_segment, dvec, ray_cast, tile_grid};

use super::{
    VisibilityPolygon,
    point::Point,
    segment::{Segment, SegmentByDistance, SegmentSide},
};

/// Crossings closer than this along a ray are considered a tie.
const MARGIN: f64 = 1e-3;

/// Minimum distance between random segments in general position.
const GAP: f64 = 1e-3;

fn cmp(q: &Point, s1: &Segment, s2: &Segment) -> Ordering {
    SegmentByDistance::new(q, s1).cmp(&SegmentByDistance::new(q, s2))
}

/// Positive if `p` is to the left of the line from `a` to `b`, negative if to the right.
fn orientation(a: DVec2, b: DVec2, p: DVec2) -> f64 {
    (b - a).perp_dot(p - a)
}

fn distance(s1: &Segment, s2: &Segment) -> f64 {
    let (a, b) = s1.ab();
    let (c, d) = s2.ab();
    let [a, b, c, d] = [a, b, c, d].map(dvec);

    if orientation(a, b, c) * orientation(a, b, d) < 0.0
        && orientation(c, d, a) * orientation(c, d, b) < 0.0
    {
        return 0.0;
    }

    [
        distance_to_segment(a, s2),
        distance_to_segment(b, s2),
        distance_to_segment(c, s1),
        distance_to_segment(d, s1),
    ]
    .into_iter()
    .fold(f64::INFINITY, f64::min)
}

/// Segments crossed by the ray from `q` through `through`, with the distances to the crossings.
/// A segment with an endpoint at `through` is crossed there, even if `ray_cast` misses it to rounding.
fn crossed(q: Point, through: Point, segments: &[Segment]) -> Vec<(f64, &Segment)> {
    let dir = dvec(through) - dvec(q);

    segments
        .iter()
        .filter_map(|segment| {
            let (a, b) = segment.ab();
            if a == through || b == through {
                return Some((dir.length(), segment));
            }
            ray_cast(q, dir, std::slice::from_ref(segment)).map(|(t, _)| (t, segment))
        })
        .collect()
}

fn seg(a: (f32, f32), b: (f32, f32)) -> Segment {
    Segment::new(a, b).unwrap()
}

/// A tile map, a ray origin within it, and a point the ray passes through:
/// a segment endpoint, like the events of the visibility sweep, or one in a random direction.
fn tile_scene(max_size: u32) -> impl Strategy<Value = (TileGrid, Point, Point)> {
    tile_grid(max_size, 0.3)
        .prop_flat_map(|grid| {
            let w2 = grid.width as f32 / 2.0;
            let h2 = grid.height as f32 / 2.0;
            let endpoints: Vec<Point> = grid
                .segments()
                .iter()
                .flat_map(|s| <[Point; 2]>::from(s.ab()))
                .collect();

            (
                Just(grid),
                (-w2..w2, -h2..h2),
                proptest::option::of(select(endpoints)),
                -PI..PI,
            )
        })
        .prop_map(|(grid, (x, y), endpoint, angle)| {
            let q = Point::new(x, y);
            (grid, q, endpoint.unwrap_or(q + Vec2::from_angle(angle)))
        })
}

/// Endpoints of segments crossing the ray from `q` through `through`, with lengths log-uniform in `lengths`.
/// They may intersect each other; `disjoint` drops those that do.
fn general_scene(
    lengths: Range<f32>,
) -> impl Strategy<Value = (Vec<(Point, Point)>, Point, Point)> {
    // Distance along the ray, fraction of the segment before the crossing, angle, log10 of length
    let crossing = (
        0.1f32..30.0,
        0f32..1.0,
        -PI..PI,
        lengths.start.log10()..lengths.end.log10(),
    );

    ((-10f32..10.0, -10f32..10.0), -PI..PI, vec(crossing, 2..12)).prop_map(
        |((x, y), angle, crossings)| {
            let q = Point::new(x, y);
            let dir = Vec2::from_angle(angle);

            let ends = crossings
                .into_iter()
                .map(|(t, u, angle, length)| {
                    let hit = q + dir * t;
                    let s = Vec2::from_angle(angle) * 10f32.powf(length);
                    (hit - s * u, hit + s * (1.0 - u))
                })
                .collect();

            (ends, q, q + dir)
        },
    )
}

/// Segments from `ends`, skipping any that would come within `gap` of one kept before it.
fn disjoint(ends: &[(Point, Point)], gap: f64) -> Vec<Segment> {
    let mut segments: Vec<Segment> = Vec::new();

    for &(a, b) in ends {
        if let Some(s) = Segment::new(a, b)
            && segments.iter().all(|kept| distance(kept, &s) > gap)
        {
            segments.push(s);
        }
    }

    segments
}

fn check_antisymmetric(q: &Point, crossed: &[(f64, &Segment)]) -> Result<(), TestCaseError> {
    for &(_, s1) in crossed {
        for &(_, s2) in crossed {
            let (o12, o21) = (cmp(q, s1, s2), cmp(q, s2, s1));
            prop_assert_eq!(o12, o21.reverse(), "Q={}, S1={}, S2={}", q, s1, s2);
        }
    }

    Ok(())
}

fn check_transitive(q: &Point, crossed: &[(f64, &Segment)]) -> Result<(), TestCaseError> {
    for &(_, s1) in crossed {
        for &(_, s2) in crossed {
            let o12 = cmp(q, s1, s2);
            if o12 == Ordering::Greater {
                continue;
            }

            for &(_, s3) in crossed {
                let o23 = cmp(q, s2, s3);
                if o23 == Ordering::Greater {
                    continue;
                }

                prop_assert_eq!(
                    cmp(q, s1, s3),
                    o12.then(o23),
                    "Q={}, S1={}, S2={}, S3={}, S1:S2={:?}, S2:S3={:?}",
                    q,
                    s1,
                    s2,
                    s3,
                    o12,
                    o23
                );
            }
        }
    }

    Ok(())
}

fn check_by_distance(q: &Point, crossed: &[(f64, &Segment)]) -> Result<(), TestCaseError> {
    for &(t1, s1) in crossed {
        for &(t2, s2) in crossed {
            if (t1 - t2).abs() > MARGIN {
                prop_assert_eq!(
                    cmp(q, s1, s2),
                    t1.total_cmp(&t2),
                    "Q={}, S1={} at {}, S2={} at {}",
                    q,
                    s1,
                    t1,
                    s2,
                    t2
                );
            }
        }
    }

    Ok(())
}

/// A `BTreeSet` of segments crossed at distinct distances, like `segments_active` in the visibility sweep,
/// keeps all of them in order of distance, and `remove` removes the given one.
fn check_set(q: &Point, crossed: &[(f64, &Segment)]) -> Result<(), TestCaseError> {
    let mut by_distance = crossed.to_vec();
    by_distance.sort_by(|(t1, _), (t2, _)| t1.total_cmp(t2));
    by_distance.dedup_by(|(t, _), (t_kept, _)| *t - *t_kept <= MARGIN);

    let kept: Vec<&Segment> = crossed
        .iter()
        .map(|&(_, s)| s)
        .filter(|&s| by_distance.iter().any(|&(_, k)| std::ptr::eq(k, s)))
        .collect();

    let mut set = BTreeSet::new();
    for &s in &kept {
        set.insert(SegmentByDistance::new(q, s));
    }

    let ordered: Vec<String> = set.iter().map(|s| s.segment.to_string()).collect();
    let expected: Vec<String> = by_distance.iter().map(|(_, s)| s.to_string()).collect();
    prop_assert_eq!(ordered, expected, "Q={}", q);

    for &s in &kept {
        prop_assert!(
            set.remove(&SegmentByDistance::new(q, s)),
            "Q={}, S={} not removed",
            q,
            s
        );
        prop_assert!(
            !set.iter().any(|x| std::ptr::eq(x.segment, s)),
            "Q={}, removed another segment instead of S={}",
            q,
            s
        );
    }

    Ok(())
}

fn check_all(q: &Point, crossed: &[(f64, &Segment)]) -> Result<(), TestCaseError> {
    check_antisymmetric(q, crossed)?;
    check_transitive(q, crossed)?;
    check_by_distance(q, crossed)?;
    check_set(q, crossed)
}

/// `intersect_with_ray` finds the crossings for the unnormalised directions the visibility sweep passes.
fn check_intersect_with_ray(
    q: Point,
    through: Point,
    crossed: &[(f64, &Segment)],
) -> Result<(), TestCaseError> {
    let unit = (dvec(through) - dvec(q)).normalize();

    for &(t, s) in crossed {
        let expected = dvec(q) + unit * t;
        let point = s.intersect_with_ray(q, q.dir(through));
        prop_assert!(
            point.is_some_and(|p| dvec(p).distance(expected) < MARGIN),
            "Q={}, through={}, S={}: got {:?}, expected {}",
            q,
            through,
            s,
            point,
            expected
        );
    }

    Ok(())
}

/// `RngSeed::Fixed` keeps rare failures from making `cargo test` flaky.
fn config(cases: u32, rng_seed: RngSeed) -> ProptestConfig {
    ProptestConfig {
        cases,
        failure_persistence: None,
        rng_seed,
        ..ProptestConfig::default()
    }
}

proptest! {
    #![proptest_config(config(256, RngSeed::Fixed(0)))]

    #[test]
    #[ignore = "convex tile corners compare Greater both ways"]
    fn tile_map_antisymmetric((grid, q, through) in tile_scene(12)) {
        let segments = grid.segments();
        check_antisymmetric(&q, &crossed(q, through, &segments))?;
    }

    #[test]
    #[ignore = "edges meeting on the ray compare inconsistently"]
    fn tile_map_transitive((grid, q, through) in tile_scene(12)) {
        let segments = grid.segments();
        check_transitive(&q, &crossed(q, through, &segments))?;
    }

    #[test]
    fn tile_map_by_distance((grid, q, through) in tile_scene(12)) {
        let segments = grid.segments();
        check_by_distance(&q, &crossed(q, through, &segments))?;
    }

    #[test]
    fn tile_map_set((grid, q, through) in tile_scene(12)) {
        let segments = grid.segments();
        check_set(&q, &crossed(q, through, &segments))?;
    }

    #[test]
    #[ignore = "panics: 'Tried to compare segments that intersect'"]
    fn tile_map_origin_near_corner(
        (grid, q, through) in tile_scene(12),
        dx in prop_oneof![-2e-4f32..-1e-5, 1e-5f32..2e-4],
        dy in prop_oneof![-2e-4f32..-1e-5, 1e-5f32..2e-4],
    ) {
        let corner = |v: f32, size: u32| (v + size as f32 / 2.0).round() - size as f32 / 2.0;
        let q = Point::new(corner(q.x, grid.width) + dx, corner(q.y, grid.height) + dy);

        let segments = grid.segments();
        let crossed = crossed(q, through, &segments);

        for &(_, s1) in &crossed {
            for &(_, s2) in &crossed {
                cmp(&q, s1, s2);
            }
        }
    }

    #[test]
    fn general_antisymmetric((ends, q, through) in general_scene(0.1..40.0)) {
        let segments = disjoint(&ends, GAP);
        check_antisymmetric(&q, &crossed(q, through, &segments))?;
    }

    #[test]
    fn general_transitive((ends, q, through) in general_scene(0.1..40.0)) {
        let segments = disjoint(&ends, GAP);
        check_transitive(&q, &crossed(q, through, &segments))?;
    }

    #[test]
    fn general_by_distance((ends, q, through) in general_scene(0.1..40.0)) {
        let segments = disjoint(&ends, GAP);
        check_by_distance(&q, &crossed(q, through, &segments))?;
    }

    #[test]
    fn general_set((ends, q, through) in general_scene(0.1..40.0)) {
        let segments = disjoint(&ends, GAP);
        check_set(&q, &crossed(q, through, &segments))?;
    }
}

proptest! {
    #![proptest_config(config(ProptestConfig::default().cases, RngSeed::Random))]

    /// Fails in about 66 of 10000 cases.
    #[test]
    #[ignore = "segments shorter than 0.1 are misordered, compare Equal, or panic"]
    fn short_segments((ends, q, through) in general_scene(0.01..0.1)) {
        let segments = disjoint(&ends, GAP);
        check_all(&q, &crossed(q, through, &segments))?;
    }

    /// Fails in about 3 of 100000 cases.
    #[test]
    #[ignore = "grazing rays: None, or off by more than 1e-3"]
    fn tile_map_intersect_with_ray((grid, q, through) in tile_scene(12)) {
        let segments = grid.segments();
        check_intersect_with_ray(q, through, &crossed(q, through, &segments))?;
    }

    #[test]
    #[ignore = "long-running"]
    fn long_tile_maps((grid, q, through) in tile_scene(32)) {
        let segments = grid.segments();
        let crossed = crossed(q, through, &segments);
        check_by_distance(&q, &crossed)?;
        check_set(&q, &crossed)?;
    }

    #[test]
    #[ignore = "long-running"]
    fn long_general_segments((ends, q, through) in general_scene(0.1..40.0)) {
        let segments = disjoint(&ends, GAP);
        check_all(&q, &crossed(q, through, &segments))?;
    }
}

#[test]
#[ignore = "cmp is Greater both ways"]
fn convex_corner_antisymmetric() {
    // Bottom and right edges of a tile, seen across its corner
    let bottom = seg((0.0, 0.0), (1.0, 0.0));
    let right = seg((1.0, 0.0), (1.0, 1.0));
    let q = Point::new(2.0, -1.0);

    assert_eq!(cmp(&q, &bottom, &right), cmp(&q, &right, &bottom).reverse());
}

#[test]
#[ignore = "panics: 'Tried to compare segments that intersect'"]
fn origin_near_corner() {
    let bottom = seg((-0.5, -0.5), (0.5, -0.5));
    let right = seg((0.5, -0.5), (0.5, 0.5));
    let q = Point::new(0.49995, -0.50005);

    cmp(&q, &bottom, &right);
}

#[test]
#[ignore = "panics: 'Tried to compare segments that intersect'"]
fn visibility_origin_near_corner() {
    let grid = TileGrid {
        width: 3,
        height: 3,
        solid: vec![false, false, false, false, true, false, false, false, false],
    };

    VisibilityPolygon::compute(Point::new(0.49995, -0.50005), &grid.segments());
}

#[test]
#[ignore = "cmp is Equal"]
fn short_parallel_segments_ordered() {
    let near = seg((1.0, -0.005), (1.0, 0.005));
    let far = seg((1.008, -0.005), (1.008, 0.005));

    assert_eq!(cmp(&Point::ZERO, &near, &far), Ordering::Less);
}

#[test]
#[ignore = "the second insert is dropped"]
fn short_parallel_segments_in_set() {
    let near = seg((1.0, -0.005), (1.0, 0.005));
    let far = seg((1.008, -0.005), (1.008, 0.005));

    let set = BTreeSet::from([
        SegmentByDistance::new(&Point::ZERO, &near),
        SegmentByDistance::new(&Point::ZERO, &far),
    ]);

    assert_eq!(set.len(), 2);
}

#[test]
#[ignore = "removes the other segment"]
fn short_parallel_segments_removed_from_set() {
    let near = seg((1.0, -0.005), (1.0, 0.005));
    let far = seg((1.008, -0.005), (1.008, 0.005));

    let mut set = BTreeSet::from([SegmentByDistance::new(&Point::ZERO, &near)]);

    assert!(!set.remove(&SegmentByDistance::new(&Point::ZERO, &far)));
}

#[test]
#[ignore = "cmp is Less one way and Equal the other"]
fn short_segment_antisymmetric() {
    let short = seg((1.0, -0.005), (1.0, 0.005));
    let long = seg((1.005, -0.5), (1.005, 0.5));

    assert_eq!(cmp(&Point::ZERO, &short, &long), Ordering::Less);
    assert_eq!(cmp(&Point::ZERO, &long, &short), Ordering::Greater);
}

#[test]
#[ignore = "panics: 'S2 can't be on the same line as S1'"]
fn short_segments_near_t() {
    // S1 points at the middle of S2 from 0.005 away
    let s1 = seg((0.005, 0.0), (0.015, 0.0));
    let s2 = seg((0.0, -0.005), (0.0, 0.005));
    let q = Point::new(-1.0, -0.404);

    assert_eq!(cmp(&q, &s1, &s2), Ordering::Greater);
}

#[test]
#[ignore = "cmp depends on the direction of S2"]
fn t_junction_ordered() {
    // S1 ends on the middle of S2, and the ray through (0, 1.5) crosses S1 first
    let s1 = seg((0.0, 1.0), (0.0, 2.0));
    let s2 = seg((-1.0, 2.0), (1.0, 2.0));
    let s2_reversed = seg((1.0, 2.0), (-1.0, 2.0));
    let q = Point::new(0.5, 0.0);

    assert_eq!(cmp(&q, &s1, &s2), Ordering::Less);
    assert_eq!(cmp(&q, &s1, &s2_reversed), Ordering::Less);
}

#[test]
fn s1_straddling_line_of_s2() {
    let q = Point::ZERO;

    // S2 is between Q and S1
    let s1 = seg((2.0, -1.0), (2.0, 3.0));
    let s2 = seg((0.5, 1.0), (1.5, 1.0));
    assert_eq!(cmp(&q, &s1, &s2), Ordering::Greater);

    // S2 is behind S1
    let s1 = seg((1.0, -1.0), (1.0, 3.0));
    let s2 = seg((1.5, 2.0), (2.5, 2.0));
    assert_eq!(cmp(&q, &s1, &s2), Ordering::Less);
}

/// Nearly parallel segments with S2 entirely behind S1, where rounding puts `S2.a` on the side of Q.
fn nearly_parallel() -> (Point, Point, [Segment; 2]) {
    let q = Point::new(9.769606, 27.473782);
    let through = Point::new(-5.192809, 16.254354);
    let s1 = seg((22.849222, -21.134676), (-24.81857, 42.435883));
    let s2 = seg((0.38970214, 8.817713), (-10.775321, 23.690992));

    (q, through, [s1, s2])
}

#[test]
fn s1_straddling_line_of_s2_rounding() {
    let (q, through, segments) = nearly_parallel();
    let [s1, s2] = &segments;

    let crossed = crossed(q, through, &segments);
    assert!(distance(s1, s2) > 0.0);
    assert!(crossed.len() == 2 && crossed[0].0 + MARGIN < crossed[1].0);

    assert_eq!(cmp(&q, s1, s2), Ordering::Less);
}

#[test]
#[ignore = "rounding in perp_dot exceeds the epsilon"]
fn which_side_rounding() {
    let (q, _, [s1, s2]) = nearly_parallel();
    let (a, b) = s1.ab();
    let s2a = s2.ab().0;

    let orientation_q = orientation(dvec(a), dvec(b), dvec(q));
    let orientation_s2a = orientation(dvec(a), dvec(b), dvec(s2a));
    assert!(orientation_q * orientation_s2a < 0.0);

    assert_ne!(s1.which_side(s2a), s1.which_side(q));
}

#[test]
#[ignore = "absolute epsilon on the unnormalised perp_dot"]
fn which_side_scale_invariant() {
    let long = seg((0.0, 0.0), (1.0, 0.0));
    assert_eq!(
        long.which_side(Point::new(0.5, 0.5)),
        Some(SegmentSide::Left)
    );

    // The same shape, 100 times smaller
    let short = seg((0.0, 0.0), (0.01, 0.0));
    assert_eq!(
        short.which_side(Point::new(0.005, 0.005)),
        Some(SegmentSide::Left)
    );
}

#[test]
#[ignore = "absolute epsilon on the denominator"]
fn intersect_with_ray_direction_length() {
    // The ray hits the segment at (5, 0.5) at an angle of 5.7 degrees
    let segment = seg((4.5, 0.5), (5.5, 0.5));
    let dir = Vec2::new(1.0, 0.1);

    let hit = segment.intersect_with_ray(Point::ZERO, dir).unwrap();
    assert!(hit.vec().distance(Vec2::new(5.0, 0.5)) < 1e-4);

    // The same ray, with the direction to an event point 0.0005 away
    assert!(
        segment
            .intersect_with_ray(Point::ZERO, dir * 5e-4)
            .is_some()
    );
}

#[test]
#[ignore = "intersects lines, if the hit should lie on the segment and the ray"]
fn intersect_with_ray_outside_segment() {
    let segment = seg((1.0, 1.0), (1.0, 2.0));

    assert_eq!(segment.intersect_with_ray(Point::ZERO, Vec2::X), None);
}

#[test]
#[ignore = "intersects lines, if the hit should lie on the segment and the ray"]
fn intersect_with_ray_behind_origin() {
    let segment = seg((1.0, -1.0), (1.0, 1.0));

    assert_eq!(segment.intersect_with_ray(Point::ZERO, Vec2::NEG_X), None);
}
