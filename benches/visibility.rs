use std::{hint::black_box, path::Path};

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use donut::{
    Assets, Config,
    geo::{Point, SegmentBuckets},
    occlusion_buckets,
};
use rand::{Rng, SeedableRng, rngs::StdRng};

/// Lights per map in the `compute` group.
const LIGHTS: usize = 16;

/// Empty tiles of `debug-01` as `(x, y)`, `y = 0` is the top row.
const DEBUG_01_LIGHTS: [(u32, u32); LIGHTS] = [
    (2, 1),
    (20, 1),
    (10, 2),
    (29, 2),
    (4, 7),
    (15, 7),
    (26, 7),
    (12, 9),
    (19, 9),
    (3, 10),
    (16, 10),
    (28, 10),
    (6, 13),
    (25, 13),
    (16, 14),
    (16, 16),
];

/// Position of a light in the tile at `(x, y)` of a `width` by `height` map.
/// The offset within the tile keeps lights off the lines through tile corners.
fn light_in_tile(width: u32, height: u32, x: u32, y: u32) -> Point {
    Point::new(
        x as f32 - width as f32 / 2.0 + 0.31,
        (height - 1 - y) as f32 - height as f32 / 2.0 + 0.57,
    )
}

struct Map<'a> {
    name: String,
    occlusion: &'a SegmentBuckets,
    lights: Vec<Point>,
}

impl<'a> Map<'a> {
    fn debug_01(assets: &'a Assets) -> Self {
        let (_, map) = assets.find_map("debug-01").unwrap();
        let size = map.size_tiles();

        Self {
            name: map.name.clone(),
            occlusion: map.occlusion(),
            lights: DEBUG_01_LIGHTS
                .iter()
                .map(|&(x, y)| light_in_tile(size.width, size.height, x, y))
                .collect(),
        }
    }
}

/// Cave-like map: random noise smoothed by a cellular automaton.
struct Cave {
    name: String,
    occlusion: SegmentBuckets,
    lights: Vec<Point>,
}

impl Cave {
    fn new(width: u32, height: u32, max_visibility: f32) -> Self {
        let mut rng = StdRng::seed_from_u64(1);

        let (w, h) = (width as i32, height as i32);
        let index = |x: i32, y: i32| (y * w + x) as usize;

        let mut solid: Vec<bool> = (0..w * h).map(|_| rng.random_bool(0.45)).collect();

        for _ in 0..4 {
            let prev = solid.clone();
            let is_solid = |x: i32, y: i32| x < 0 || y < 0 || x >= w || y >= h || prev[index(x, y)];

            for y in 0..h {
                for x in 0..w {
                    let neighbours = (-1..=1)
                        .flat_map(|dy| (-1..=1).map(move |dx| (dx, dy)))
                        .filter(|&(dx, dy)| is_solid(x + dx, y + dy))
                        .count();
                    solid[index(x, y)] = neighbours >= 5;
                }
            }
        }

        let is_solid = |x: i32, y: i32| x >= 0 && y >= 0 && x < w && y < h && solid[index(x, y)];

        let mut lights = Vec::with_capacity(LIGHTS);
        while lights.len() < LIGHTS {
            let (x, y) = (rng.random_range(0..width), rng.random_range(0..height));
            if !is_solid(x as i32, y as i32) {
                lights.push(light_in_tile(width, height, x, y));
            }
        }

        Self {
            name: format!("cave-{width}x{height}"),
            occlusion: occlusion_buckets(width, height, &[is_solid], max_visibility),
            lights,
        }
    }

    fn map(&self) -> Map<'_> {
        Map {
            name: self.name.clone(),
            occlusion: &self.occlusion,
            lights: self.lights.clone(),
        }
    }
}

fn compute_all(lights: &[Point], occlusion: &SegmentBuckets) {
    for &light in lights {
        black_box(occlusion.visibility(light).unwrap());
    }
}

fn bench_visibility(c: &mut Criterion) {
    let dir_assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    let config = Config::load(dir_assets.join("config.toml")).unwrap();
    let assets = Assets::resolve(config, dir_assets).unwrap();

    let caves = [
        Cave::new(64, 36, assets.max_visibility),
        Cave::new(128, 72, assets.max_visibility),
    ];
    let maps = [Map::debug_01(&assets), caves[0].map(), caves[1].map()];

    // Time per light is the reported time divided by `LIGHTS`
    let mut group = c.benchmark_group("compute");
    group.throughput(Throughput::Elements(LIGHTS as u64));
    for map in &maps {
        group.bench_with_input(BenchmarkId::from_parameter(&map.name), map, |b, map| {
            b.iter(|| compute_all(&map.lights, map.occlusion))
        });
    }
    group.finish();

    // Lights repeat the fixed positions on debug-01
    let debug_01 = &maps[0];
    let mut group = c.benchmark_group("frame");
    for count in [12, 64, 256] {
        let lights: Vec<Point> = debug_01
            .lights
            .iter()
            .copied()
            .cycle()
            .take(count)
            .collect();
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::new(&debug_01.name, count),
            &lights,
            |b, lights| b.iter(|| compute_all(lights, debug_01.occlusion)),
        );
    }
    group.finish();
}

criterion_group!(benches, bench_visibility);
criterion_main!(benches);
