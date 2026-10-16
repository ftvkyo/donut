//! Property tests of `VisibilityPolygon::compute` on random tile maps, against `ray_cast`.
//! Light positions are split into categories, so that a failure in one does not hide the others.

use std::{collections::BTreeMap, f64::consts::TAU, panic};

use glam::{Vec2, vec2};
use proptest::{
    prelude::*,
    sample::select,
    strategy::ValueTree,
    test_runner::{Config, RngSeed, TestError, TestRunner},
};

use crate::{
    geo::{Point, Segment},
    testing::{TileGrid, check_visibility, tile_grid},
};

/// Map units; 1/16 of a pixel at 16 pixels per tile, and far above f32 rounding of map coordinates.
const TOLERANCE: f64 = 1e-3;

/// Up to the size of the game's map.
const MAX_SIZE: u32 = 32;

/// Distances of lights from walls, in map units.
const DISTANCES: [f32; 4] = [1e-6, 1e-5, 1e-4, 1e-3];

/// Takes the number of cases from `PROPTEST_CASES`, 256 by default.
/// The seed is fixed, so that rare failures do not make `cargo test` flaky.
fn config() -> Config {
    Config {
        failure_persistence: None,
        rng_seed: RngSeed::Fixed(0),
        ..Config::default()
    }
}

/// Scale, then offset, applied to a whole scene.
#[derive(Clone, Copy, Debug)]
struct Transform {
    scale: f32,
    offset: Vec2,
}

impl Transform {
    const IDENTITY: Self = Self::new(1.0, 0.0);

    const fn new(scale: f32, offset: f32) -> Self {
        Self {
            scale,
            offset: Vec2::new(offset, offset),
        }
    }

    fn apply(self, p: Point) -> Point {
        Point::new(p.x * self.scale, p.y * self.scale) + self.offset
    }
}

/// Checks the visibility of `grid` from `q`, with both transformed by `transform`.
fn check(
    grid: &TileGrid,
    q: Point,
    transform: Transform,
    extra: &[f64],
) -> Result<(), TestCaseError> {
    let segments: Vec<_> = grid
        .segments()
        .iter()
        .map(|s| {
            let (a, b) = s.ab();
            Segment::new(transform.apply(a), transform.apply(b)).unwrap()
        })
        .collect();

    let tolerance = TOLERANCE * transform.scale as f64;
    check_visibility(transform.apply(q), &segments, extra, tolerance)
}

type Light = fn(&TileGrid) -> Option<BoxedStrategy<Point>>;

/// Random grids with a light from `light`, skipping grids where it has no positions,
/// and random extra directions to check.
fn scenes(light: Light) -> impl Strategy<Value = ((TileGrid, Point), Vec<f64>)> {
    let grids = (0.1..0.6).prop_flat_map(|density| tile_grid(MAX_SIZE, density));

    let scenes = grids
        .prop_filter("No light positions", move |grid| light(grid).is_some())
        .prop_flat_map(move |grid| light(&grid).unwrap().prop_map(move |q| (grid.clone(), q)));

    (scenes, proptest::collection::vec(0.0..TAU, 8))
}

fn property(light: Light, transform: Transform) {
    let mut runner = TestRunner::new(config());

    let result = runner.run(&scenes(light), |((grid, q), extra)| {
        check(&grid, q, transform, &extra)
    });

    match result {
        Ok(()) => (),
        Err(TestError::Fail(reason, ((grid, q), _))) => {
            panic!("{reason}\nMinimal failing input: {q:?} in\n{grid}")
        }
        Err(TestError::Abort(reason)) => panic!("{reason}"),
    }
}

/// Defines a test for each light category, and `CATEGORIES` listing them all.
macro_rules! categories {
    ($($(#[$meta:meta])* $name:ident: $light:expr, $transform:expr;)*) => {
        $(
            #[test]
            $(#[$meta])*
            fn $name() {
                property($light, $transform);
            }
        )*

        const CATEGORIES: &[(&str, Light, Transform)] = &[$((stringify!($name), $light, $transform)),*];
    };
}

categories! {
    light_in_empty_tiles: in_empty_tiles, Transform::IDENTITY;
    light_in_solid_tiles: in_solid_tiles, Transform::IDENTITY;
    light_on_empty_corners: on_empty_corners, Transform::IDENTITY;
    light_on_grid_lines: on_grid_lines, Transform::IDENTITY;

    #[ignore = "fails 83%: mostly `Origin .. is on the segment`, intersecting segments in `cmp`"]
    light_on_segments: on_segments, Transform::IDENTITY;
    #[ignore = "fails 46%: mostly `Origin .. is on the segment`, double starts and ends"]
    light_on_segment_lines: on_segment_lines, Transform::IDENTITY;
    #[ignore = "fails 52%: mostly `Origin .. is on the segment`, double starts and ends"]
    light_near_walls: near_walls, Transform::IDENTITY;
    #[ignore = "fails 46%: intersecting segments in `cmp`, double starts and ends, stray segments"]
    light_near_corners: near_corners, Transform::IDENTITY;
    #[ignore = "fails 44%: `Origin .. is on the segment`"]
    light_near_map_edges: near_map_edges, Transform::IDENTITY;

    #[ignore = "fails 99%: panics in `cmp` and in the sweep"]
    light_in_empty_tiles_scaled_by_0_01: in_empty_tiles, Transform::new(0.01, 0.0);
    light_in_empty_tiles_scaled_by_0_1: in_empty_tiles, Transform::new(0.1, 0.0);
    light_in_empty_tiles_scaled_by_10: in_empty_tiles, Transform::new(10.0, 0.0);
    light_in_empty_tiles_scaled_by_100: in_empty_tiles, Transform::new(100.0, 0.0);
    #[ignore = "fails 29%: stray and overlapping output segments"]
    light_in_empty_tiles_translated_by_100: in_empty_tiles, Transform::new(1.0, 100.0);
    #[ignore = "fails 99%: stray and overlapping output segments, gaps"]
    light_in_empty_tiles_translated_by_1000: in_empty_tiles, Transform::new(1.0, 1000.0);
    #[ignore = "fails 99%: stray output segments"]
    light_in_empty_tiles_translated_by_10000: in_empty_tiles, Transform::new(1.0, 10000.0);
}

/// Runs every light category with a random seed, without stopping at failures,
/// then fails listing how often each kind of failure occurred, with its smallest example.
/// Takes about a minute per 10000 cases:
/// `PROPTEST_CASES=1000000 cargo test --release --lib overnight -- --ignored --nocapture`
#[test]
#[ignore = "long-running"]
fn overnight() {
    let mut runner = TestRunner::new(Config {
        rng_seed: RngSeed::Random,
        ..config()
    });
    let cases = runner.config().cases;

    let mut failures = BTreeMap::<(&str, String), (u32, TileGrid, Point, String)>::new();

    let hook = panic::take_hook();
    panic::set_hook(Box::new(|_| ()));

    for &(name, light, transform) in CATEGORIES {
        let strategy = scenes(light);

        for _ in 0..cases {
            let ((grid, q), extra) = strategy.new_tree(&mut runner).unwrap().current();

            let Some(message) = failure(&grid, q, transform, &extra) else {
                continue;
            };

            let failure = failures
                .entry((name, kind(&message)))
                .or_insert_with(|| (0, grid.clone(), q, message.clone()));

            failure.0 += 1;

            if grid.solid.len() < failure.1.solid.len() {
                (failure.1, failure.2, failure.3) = (grid, q, message);
            }
        }
    }

    panic::set_hook(hook);

    for ((name, kind), (count, grid, q, message)) in &failures {
        println!("{name}: {count}/{cases}: {kind}\nSmallest: {q:?} in\n{grid}{message}\n");
    }

    assert!(failures.is_empty(), "{} kinds of failures", failures.len());
}

/// The message of a failed check or of a panic in it, which the panic hook still prints.
fn failure(grid: &TileGrid, q: Point, transform: Transform, extra: &[f64]) -> Option<String> {
    match panic::catch_unwind(|| check(grid, q, transform, extra)) {
        Ok(Ok(())) => None,
        Ok(Err(TestCaseError::Fail(reason) | TestCaseError::Reject(reason))) => {
            Some(reason.message().to_string())
        }
        Err(payload) => match payload.downcast::<String>() {
            Ok(message) => Some(*message),
            Err(payload) => payload
                .downcast::<&str>()
                .ok()
                .map(|message| message.to_string()),
        },
    }
}

/// The start of a failure message, up to the details.
fn kind(message: &str) -> String {
    let message = message.trim_start_matches("internal error: entered unreachable code: ");
    let kind = message.split([':', '[', ',']).next().unwrap();
    kind.trim().replace(|c: char| c.is_ascii_digit(), "#")
}

// Light positions

fn in_empty_tiles(grid: &TileGrid) -> Option<BoxedStrategy<Point>> {
    in_tiles(tiles(grid, false))
}

fn in_solid_tiles(grid: &TileGrid) -> Option<BoxedStrategy<Point>> {
    in_tiles(tiles(grid, true))
}

/// Grid corners with no solid tiles around them.
fn on_empty_corners(grid: &TileGrid) -> Option<BoxedStrategy<Point>> {
    let mut corners = Vec::new();

    for x in 0..=grid.width {
        for y in 0..=grid.height {
            let (xi, yi) = (x as i32, y as i32);
            let tiles = [(xi - 1, yi - 1), (xi, yi - 1), (xi - 1, yi), (xi, yi)];
            if tiles.iter().all(|(x, y)| !grid.is_solid(*x, *y)) {
                corners.push(grid.corner(x, y));
            }
        }
    }

    (!corners.is_empty()).then(|| select(corners).boxed())
}

/// Grid lines between two empty tiles, at least 0.01 from the grid corners.
fn on_grid_lines(grid: &TileGrid) -> Option<BoxedStrategy<Point>> {
    let mut lines = Vec::new();

    for x in 0..=grid.width {
        for y in 0..grid.height {
            let (xi, yi) = (x as i32, y as i32);
            if !grid.is_solid(xi - 1, yi) && !grid.is_solid(xi, yi) {
                lines.push((grid.corner(x, y), vec2(0.0, -1.0)));
            }
        }
    }

    for x in 0..grid.width {
        for y in 0..=grid.height {
            let (xi, yi) = (x as i32, y as i32);
            if !grid.is_solid(xi, yi - 1) && !grid.is_solid(xi, yi) {
                lines.push((grid.corner(x, y), vec2(1.0, 0.0)));
            }
        }
    }

    if lines.is_empty() {
        return None;
    }

    let strategy = (select(lines), 0.01f32..0.99).prop_map(|((corner, dir), t)| corner + dir * t);

    Some(strategy.boxed())
}

/// Points on segments, including their ends.
fn on_segments(grid: &TileGrid) -> Option<BoxedStrategy<Point>> {
    let t = prop_oneof![Just(0.0), Just(1.0), 0.0f32..1.0];

    let strategy = (select(segment_ends(grid)), t).prop_map(|((a, b), t)| {
        let mut q = a.lerp(b, t);

        // Exactly on the axis-aligned segments
        if a.x == b.x {
            q.x = a.x;
        }
        if a.y == b.y {
            q.y = a.y;
        }

        q
    });

    Some(strategy.boxed())
}

/// Points on the lines through segments, beyond their ends.
fn on_segment_lines(grid: &TileGrid) -> Option<BoxedStrategy<Point>> {
    let beyond = prop_oneof![select(DISTANCES.to_vec()), 1e-6f32..3.0];

    let strategy =
        (select(segment_ends(grid)), any::<bool>(), beyond).prop_map(|((a, b), flip, beyond)| {
            let (a, b) = if flip { (b, a) } else { (a, b) };
            let mut q = b + a.dir(b).normalize() * beyond;

            // Exactly on the axis-aligned lines
            if a.x == b.x {
                q.x = a.x;
            }
            if a.y == b.y {
                q.y = a.y;
            }

            q
        });

    Some(strategy.boxed())
}

/// Points near tile edges.
fn near_walls(grid: &TileGrid) -> Option<BoxedStrategy<Point>> {
    let walls = segment_ends(grid)
        .into_iter()
        .filter(|(a, b)| a.dist(*b) <= 1.0)
        .collect();

    near(grid, walls, DISTANCES.to_vec(), None)
}

/// Points near map edges, on the inside.
fn near_map_edges(grid: &TileGrid) -> Option<BoxedStrategy<Point>> {
    let distances = [DISTANCES.as_slice(), &[1e-2, 0.1, 0.5]].concat();
    near(grid, map_edges(grid), distances, Some(1.0))
}

/// Points diagonally off segment ends.
fn near_corners(grid: &TileGrid) -> Option<BoxedStrategy<Point>> {
    let corners: Vec<Point> = segment_ends(grid)
        .into_iter()
        .flat_map(|(a, b)| [a, b])
        .collect();

    let diagonals = vec![
        vec2(1.0, 1.0),
        vec2(-1.0, 1.0),
        vec2(-1.0, -1.0),
        vec2(1.0, -1.0),
    ];

    let strategy = (
        select(corners),
        select(DISTANCES.to_vec()),
        select(diagonals),
    )
        .prop_map(|(corner, distance, diagonal)| corner + diagonal * distance);

    Some(in_empty_space(grid, strategy))
}

// Helpers for light positions

fn segment_ends(grid: &TileGrid) -> Vec<(Point, Point)> {
    grid.segments().iter().map(|s| s.ab()).collect()
}

/// Counter-clockwise, so that their left side is inside.
fn map_edges(grid: &TileGrid) -> Vec<(Point, Point)> {
    let x = grid.width as f32 / 2.0 + 1.0;
    let y = grid.height as f32 / 2.0 + 1.0;

    vec![
        (Point::new(x, y), Point::new(-x, y)),
        (Point::new(-x, y), Point::new(-x, -y)),
        (Point::new(-x, -y), Point::new(x, -y)),
        (Point::new(x, -y), Point::new(x, y)),
    ]
}

fn tiles(grid: &TileGrid, solid: bool) -> Vec<Point> {
    let mut tiles = Vec::new();

    for x in 0..grid.width {
        for y in 0..grid.height {
            if grid.is_solid(x as i32, y as i32) == solid {
                tiles.push(grid.tile_origin(x, y));
            }
        }
    }

    tiles
}

/// Points at least 0.01 from the edges of `tiles`.
fn in_tiles(tiles: Vec<Point>) -> Option<BoxedStrategy<Point>> {
    if tiles.is_empty() {
        return None;
    }

    let strategy = (select(tiles), 0.01f32..0.99, 0.01f32..0.99)
        .prop_map(|(origin, x, y)| origin + vec2(x, y));

    Some(strategy.boxed())
}

/// Points at `distances` from `walls`, at least 0.01 from their ends,
/// on their left side if `side` is 1, right if -1, or either if `None`.
fn near(
    grid: &TileGrid,
    walls: Vec<(Point, Point)>,
    distances: Vec<f32>,
    side: Option<f32>,
) -> Option<BoxedStrategy<Point>> {
    if walls.is_empty() {
        return None;
    }

    let side = match side {
        Some(side) => Just(side).boxed(),
        None => select(vec![-1.0, 1.0]).boxed(),
    };

    let strategy = (select(walls), 0.01f32..0.99, select(distances), side).prop_map(
        |((a, b), t, distance, side)| a.lerp(b, t) + a.dir(b).normalize().perp() * distance * side,
    );

    Some(in_empty_space(grid, strategy))
}

/// Keeps points of `strategy` in empty tiles inside the map edges.
fn in_empty_space(
    grid: &TileGrid,
    strategy: impl Strategy<Value = Point> + 'static,
) -> BoxedStrategy<Point> {
    let grid = grid.clone();
    let inside = vec2(grid.width as f32, grid.height as f32) / 2.0 + 1.0;

    strategy
        .prop_filter("Not in empty space", move |q| {
            q.x.abs() < inside.x && q.y.abs() < inside.y && !grid.is_solid_at(*q)
        })
        .boxed()
}
