use std::{
    collections::BTreeSet,
    f32::consts::{PI, TAU},
    fmt::Display,
};

use glam::vec2;
use log::trace;

use crate::geo::ImpreciseEq;

use super::{
    point::Point,
    segment::{Segment, SegmentByDistance},
};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum EventKind {
    Start,
    End,
}

impl Display for EventKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventKind::Start => write!(f, "start"),
            EventKind::End => write!(f, "end"),
        }
    }
}

#[derive(Clone)]
struct Event {
    kind: EventKind,
    segment_index: usize,
    point: Point,
    point_angle: f32,
}

impl Event {
    fn cmp_angle(&self, other: &Self) -> std::cmp::Ordering {
        self.point_angle
            .total_cmp(&other.point_angle)
            .then(self.kind.cmp(&other.kind).reverse())
    }

    fn is_start(&self) -> bool {
        self.kind == EventKind::Start
    }
}

/// Note: segments must not intersect except at their endpoints.
pub fn compute_visibility(origin: Point, segments: &[Segment]) -> Vec<Segment> {
    let angle0 = vec2(1.0, 0.0);

    // Segment start and end events.
    let mut events = Vec::with_capacity(segments.len() * 2);

    for (segment_index, segment) in segments.iter().enumerate() {
        let (point_a, point_b) = segment.ab();

        let point_a_angle = angle0.angle_to(origin.dir(point_a));
        let point_b_angle = angle0.angle_to(origin.dir(point_b));

        let angle_diff = (point_a_angle - point_b_angle).abs();

        if angle_diff.is_basically_zero() || angle_diff.is_basically_equal(&TAU) {
            trace!("Skipping a segment {segment} aligned with the origin {origin}");
            continue;
        }

        if angle_diff.is_basically_equal(&PI) {
            panic!("Origin {origin} is on the segment {segment}");
        }

        let (point_start, point_start_angle, point_end, point_end_angle) =
            match (point_a_angle < point_b_angle, angle_diff > PI) {
                (true, false) | (false, true) => (point_a, point_a_angle, point_b, point_b_angle),
                (false, false) | (true, true) => (point_b, point_b_angle, point_a, point_a_angle),
            };

        events.push(Event {
            kind: EventKind::Start,
            segment_index,
            point: point_start,
            point_angle: point_start_angle,
        });

        events.push(Event {
            kind: EventKind::End,
            segment_index,
            point: point_end,
            point_angle: point_end_angle,
        });
    }

    assert!(events.len() >= 2);
    events.sort_by(|a, b| a.cmp_angle(&b));

    let mut segments_active_at_first_event = BTreeSet::<usize>::new();

    for event in events.iter().skip(1) {
        if event.is_start() {
            segments_active_at_first_event.insert(event.segment_index);
        } else {
            segments_active_at_first_event.remove(&event.segment_index);
        }
    }

    let mut segments_active = BTreeSet::<SegmentByDistance<'_, '_>>::new();

    for segment_i in segments_active_at_first_event {
        let segment = SegmentByDistance::new(&origin, &segments[segment_i]);
        segments_active.insert(segment);
    }

    let mut vis_acc_start = None;
    let mut vis_acc_end = None;

    let mut vis_segments = Vec::new();

    let mut add_vis_event = |point: Point, kind: EventKind| {
        match (kind, point, vis_acc_start, vis_acc_end) {
            (EventKind::Start, start, Some(existing_start), _) => {
                unreachable!("Got a double start. Existing: {existing_start}, New: {start}")
            }
            (EventKind::Start, start, None, _) => {
                trace!(" -> Saving {start} as a start of a segment!");
                vis_acc_start = Some(start);
            }
            (EventKind::End, end, None, Some(existing_end)) => {
                unreachable!("Got a double end. Existing: {existing_end}, New: {end}")
            }
            (EventKind::End, end, None, None) => {
                trace!(" -> Saving {end} as a start of a segment!");
                vis_acc_end = Some(end);
            }
            (EventKind::End, end, Some(start), _) => {
                vis_acc_start = None;
                if let Some(seg) = Segment::new(start, end) {
                    trace!(" -> Adding a completed segment {seg}!");
                    vis_segments.push(seg);
                } else {
                    trace!(" -> Generated an invalid segment from {start} & {end}, skipping");
                }
            }
        };
    };

    for (event_i, event) in events.into_iter().enumerate() {
        let event_segment = SegmentByDistance::new(&origin, &segments[event.segment_index]);
        let event_dir = origin.dir(event.point);

        trace!(
            "Processing event {event_i:2}: point {}, {} of {} (angle {:3.2} deg)",
            event.point,
            event.kind,
            event_segment.segment,
            event.point_angle.to_degrees(),
        );

        for segment in &segments_active {
            trace!(" -> There is an active segment: {}", segment.segment);
        }

        if event.is_start() {
            let is_new_nearest = segments_active
                .first()
                .map(|nearest| event_segment < *nearest)
                .unwrap_or(true);

            if is_new_nearest {
                trace!(" -> This event's segment will be the new nearest");

                if let Some(nearest) = segments_active.first() {
                    trace!(" -> Current nearest segment is {}", nearest.segment);
                    let intersection = nearest.segment.intersect_with_ray(origin, event_dir);
                    if let Some(point) = intersection {
                        trace!(" -> Intersection on that segment is: {point}");
                        add_vis_event(point, EventKind::End);
                    }
                }

                add_vis_event(event.point, EventKind::Start);
            }

            trace!(" -> Activating the segment...");
            segments_active.insert(event_segment);
        } else {
            let is_nearest = segments_active.first() == Some(&event_segment);

            trace!(" -> Deactivating the segment...");
            segments_active.remove(&event_segment);

            if is_nearest {
                trace!(" -> This event's segment is current nearest");

                add_vis_event(event.point, EventKind::End);

                if let Some(new_nearest) = segments_active.first() {
                    trace!(" -> The next nearest segment is: {}", new_nearest.segment);
                    let intersection = new_nearest.segment.intersect_with_ray(origin, event_dir);
                    if let Some(point) = intersection {
                        trace!(" -> Intersection on that segment is: {point}");
                        add_vis_event(point, EventKind::Start);
                    }
                }
            }
        }
    }

    match (vis_acc_start, vis_acc_end) {
        (Some(start), Some(end)) => {
            if let Some(seg) = Segment::new(start, end) {
                trace!("Adding a split final segment {seg}!");
                vis_segments.push(seg);
            } else {
                trace!("Got an invalid split final segment, skipping");
            }
        }
        (None, None) => (),
        (start_acc, end_acc) => {
            unreachable!(
                "Could not match the remaining points! Remaining start: {start_acc:?}, Remaining end: {end_acc:?}"
            )
        }
    }

    assert!(vis_segments.len() >= 1);

    return vis_segments;
}

#[cfg(test)]
mod tests {
    use crate::init_logging;

    use super::*;

    macro_rules! seg {
        ($x1:expr, $y1:expr, $x2:expr, $y2:expr) => {
            Segment::new(
                Point::new($x1 as f32, $y1 as f32),
                Point::new($x2 as f32, $y2 as f32),
            )
            .expect("Non-zero-length segment")
        };
    }

    fn compute(origin: Point, input: &[Segment]) -> Vec<Segment> {
        init_logging();
        let vis = compute_visibility(origin, input);
        println!("Origin: {origin}");
        return vis;
    }

    fn compare(expected: &[Segment], result: &[Segment]) {
        let mut fail = false;

        println!("Expected:");
        for s in expected {
            if result.contains(s) {
                println!(" -> {s} - ok");
            } else {
                println!(" -> {s} - missing");
                fail = true;
            }
        }

        println!("Result:");
        for s in result {
            if expected.contains(s) {
                println!(" -> {s} - ok");
            } else {
                println!(" -> {s} - not expected");
                fail = true;
            }
        }

        assert!(!fail);
    }

    #[test]
    fn visibility_single() {
        let input = [seg!(1, 1, -1, 1)];
        let output = compute((0.0, 0.0).into(), &input);
        compare(&input, &output);
    }

    #[test]
    fn visibility_diamond_inside() {
        let input = [
            seg!(-1, 0, 0, -1),
            seg!(0, -1, 1, 0),
            seg!(1, 0, 0, 1),
            seg!(0, 1, -1, 0),
        ];
        let output = compute((0.0, 0.0).into(), &input);
        compare(&input, &output);
    }

    #[test]
    fn visibility_square_inside() {
        let input = [
            seg!(1, 1, -1, 1),
            seg!(-1, 1, -1, -1),
            seg!(-1, -1, 1, -1),
            seg!(1, -1, 1, 1),
        ];
        let output = compute((0.0, 0.0).into(), &input);
        compare(&input, &output);
    }

    #[test]
    fn visibility_square_multiple_origins_inside() {
        let input = [
            // Top
            seg!(-5, 5, 0, 5),
            seg!(0, 5, 5, 5),
            // Right
            seg!(5, 5, 5, 0),
            seg!(5, 0, 5, -5),
            // Bottom
            seg!(5, -5, 0, -5),
            seg!(0, -5, -5, -5),
            // Left
            seg!(-5, -5, -5, 0),
            seg!(-5, 0, -5, 5),
        ];
        for x in -4..=4 {
            for y in -4..=4 {
                let origin = (x as f32, y as f32).into();
                let output = compute(origin, &input);
                compare(&input, &output);
            }
        }
    }

    #[test]
    fn visibility_square_multiple_origins_outside() {
        let input = [
            // Top
            seg!(-5, 5, 0, 5),
            seg!(0, 5, 5, 5),
            // Right
            seg!(5, 5, 5, 0),
            seg!(5, 0, 5, -5),
            // Bottom
            seg!(5, -5, 0, -5),
            seg!(0, -5, -5, -5),
            // Left
            seg!(-5, -5, -5, 0),
            seg!(-5, 0, -5, 5),
        ];

        //     ^
        // 10  |  12  13  14  15   0
        //     |
        // 5   |  11   +---+---+   1
        //     |       |       |
        // 0   |  10   +       +   2
        //     |       |       |
        // -5  |   9   +---+---+   3
        //     |
        // -10 |   8   7   6   5   4
        //     |
        // ----+--------------------->
        //     | -10  -5   0   5  10

        {
            let origin = (10.0, 10.0).into();
            let output = compute(origin, &input);
            let expected = [
                // Top
                seg!(-5, 5, 0, 5),
                seg!(0, 5, 5, 5),
                // Right
                seg!(5, 5, 5, 0),
                seg!(5, 0, 5, -5),
            ];
            compare(&expected, &output);
        }

        {
            let origin = (10.0, 5.0).into();
            let output = compute(origin, &input);
            let expected = [
                // Right
                seg!(5, 5, 5, 0),
                seg!(5, 0, 5, -5),
            ];
            compare(&expected, &output);
        }

        {
            let origin = (10.0, 0.0).into();
            let output = compute(origin, &input);
            let expected = [
                // Right
                seg!(5, 5, 5, 0),
                seg!(5, 0, 5, -5),
            ];
            compare(&expected, &output);
        }

        {
            let origin = (10.0, -5.0).into();
            let output = compute(origin, &input);
            let expected = [
                // Right
                seg!(5, 5, 5, 0),
                seg!(5, 0, 5, -5),
            ];
            compare(&expected, &output);
        }

        {
            let origin = (10.0, -10.0).into();
            let output = compute(origin, &input);
            let expected = [
                // Right
                seg!(5, 5, 5, 0),
                seg!(5, 0, 5, -5),
                // Bottom
                seg!(5, -5, 0, -5),
                seg!(0, -5, -5, -5),
            ];
            compare(&expected, &output);
        }

        {
            let origin = (5.0, -10.0).into();
            let output = compute(origin, &input);
            let expected = [
                // Bottom
                seg!(5, -5, 0, -5),
                seg!(0, -5, -5, -5),
            ];
            compare(&expected, &output);
        }

        {
            let origin = (0.0, -10.0).into();
            let output = compute(origin, &input);
            let expected = [
                // Bottom
                seg!(5, -5, 0, -5),
                seg!(0, -5, -5, -5),
            ];
            compare(&expected, &output);
        }

        {
            let origin = (-5.0, -10.0).into();
            let output = compute(origin, &input);
            let expected = [
                // Bottom
                seg!(5, -5, 0, -5),
                seg!(0, -5, -5, -5),
            ];
            compare(&expected, &output);
        }

        {
            let origin = (-10.0, -10.0).into();
            let output = compute(origin, &input);
            let expected = [
                // Bottom
                seg!(5, -5, 0, -5),
                seg!(0, -5, -5, -5),
                // Left
                seg!(-5, -5, -5, 0),
                seg!(-5, 0, -5, 5),
            ];
            compare(&expected, &output);
        }

        {
            let origin = (-10.0, -5.0).into();
            let output = compute(origin, &input);
            let expected = [
                // Left
                seg!(-5, -5, -5, 0),
                seg!(-5, 0, -5, 5),
            ];
            compare(&expected, &output);
        }

        {
            let origin = (-10.0, 0.0).into();
            let output = compute(origin, &input);
            let expected = [
                // Left
                seg!(-5, -5, -5, 0),
                seg!(-5, 0, -5, 5),
            ];
            compare(&expected, &output);
        }

        {
            let origin = (-10.0, 5.0).into();
            let output = compute(origin, &input);
            let expected = [
                // Left
                seg!(-5, -5, -5, 0),
                seg!(-5, 0, -5, 5),
            ];
            compare(&expected, &output);
        }

        {
            let origin = (-10.0, 10.0).into();
            let output = compute(origin, &input);
            let expected = [
                // Left
                seg!(-5, -5, -5, 0),
                seg!(-5, 0, -5, 5),
                // Top
                seg!(-5, 5, 0, 5),
                seg!(0, 5, 5, 5),
            ];
            compare(&expected, &output);
        }

        {
            let origin = (-5.0, 10.0).into();
            let output = compute(origin, &input);
            let expected = [
                // Top
                seg!(-5, 5, 0, 5),
                seg!(0, 5, 5, 5),
            ];
            compare(&expected, &output);
        }

        {
            let origin = (0.0, 10.0).into();
            let output = compute(origin, &input);
            let expected = [
                // Top
                seg!(-5, 5, 0, 5),
                seg!(0, 5, 5, 5),
            ];
            compare(&expected, &output);
        }

        {
            let origin = (5.0, 10.0).into();
            let output = compute(origin, &input);
            let expected = [
                // Top
                seg!(-5, 5, 0, 5),
                seg!(0, 5, 5, 5),
            ];
            compare(&expected, &output);
        }
    }

    #[test]
    fn visibility_no_isolated_events() {
        let input = [
            seg!(1, -2, 2, 1),
            seg!(5, 1, 1, 5),
            seg!(1, 2, -1, 2),
            seg!(-1, 5, -5, 1),
            seg!(-2, 1, -1, -2),
            seg!(-3, -4, 3, -4),
        ];
        let output = compute((0.0, 0.0).into(), &input);
        let expected = [
            seg!(1, -2, 2, 1),
            seg!(4, 2, 2, 4),
            seg!(1, 2, -1, 2),
            seg!(-2, 4, -4, 2),
            seg!(-2, 1, -1, -2),
            seg!(-2, -4, 2, -4),
        ];
        compare(&expected, &output);
    }

    #[test]
    fn visibility_complicated() {
        let input = [
            seg!(-2, -4, 13, -1),
            seg!(2, 0, 2, 1),
            seg!(4, -1, 4, 3),
            seg!(2, 2, 0, 2),
            seg!(2, 3, 1, 3),
            seg!(0, 3, -2, 0),
            seg!(-1, 0, 2, -2),
        ];
        let output = compute((0.0, 0.0).into(), &input);
        let expected = [
            seg!(2, 0, 2, 1),
            seg!(4, 2, 4, 3),
            seg!(2, 2, 0, 2),
            seg!(0, 3, -2, 0),
            seg!(-1, 0, 2, -2),
            seg!(3, -3, 8, -2),
            seg!(4, -1, 4, 0),
        ];
        compare(&expected, &output);
    }

    #[test]
    fn visibility_realistic() {
        // xxxxxxxxxxxxxx
        // x            x
        // x     444 3  x
        // x            x
        // x   0        x
        // x 1 0        x
        // x            x
        // x     222    x
        // x       22   x
        // x            x
        // x            x
        // xxxxxxxxxxxxxx

        let input = [
            // Shape 0
            seg!(-2, 2, -3, 2),
            seg!(-2, 2, -2, 0),
            seg!(-2, 0, -3, 0),
            seg!(-3, 0, -3, 2),
            // Shape 1
            seg!(-4, 0, -4, 1),
            seg!(-4, 0, -5, 0),
            seg!(-4, 1, -5, 1),
            seg!(-5, 0, -5, 1),
            // Shape 2
            seg!(-1, -1, 2, -1),
            seg!(2, -1, 2, -2),
            seg!(2, -2, 3, -2),
            seg!(3, -2, 3, -3),
            seg!(3, -3, 1, -3),
            seg!(1, -3, 1, -2),
            seg!(1, -2, -1, -2),
            seg!(-1, -2, -1, -1),
            // Shape 3
            seg!(4, 3, 4, 4),
            seg!(4, 3, 3, 3),
            seg!(3, 3, 3, 4),
            seg!(3, 4, 4, 4),
            // Shape 4
            seg!(-1, 3, 2, 3),
            seg!(2, 3, 2, 4),
            seg!(2, 4, -1, 4),
            seg!(-1, 4, -1, 3),
            // Walls
            seg!(-6, 5, 6, 5),
            seg!(-6, 5, -6, -5),
            seg!(-6, -5, 6, -5),
            seg!(6, 5, 6, -5),
        ];
        let output = compute((0.0, 1.0).into(), &input);
        let expected = [
            seg!(-2, 2, -2, 0),
            seg!(-6, -2, -6, -5),
            seg!(-6, -5, -3, -5),
            seg!(-1, -1, 2, -1),
            seg!(6, -5, 6, 4),
            seg!(4, 3, 3, 3),
            seg!(3, 3, 3, 4),
            seg!(2, 3, -1, 3),
            seg!(-2, 5, -6, 5),
            seg!(-6, 5, -6, 4),
        ];
        compare(&expected, &output);
    }
}
