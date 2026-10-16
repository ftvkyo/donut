//! Behaviour of the physics with the game's timestep and object sizes.

use std::time::Duration;

use glam::{Vec2, vec2};

use crate::{
    geo::{Point, Segment, ToricGeometry},
    phys::{
        Physics,
        collision::CollideWith,
        object::{PhysObject, SceneObject},
    },
    testing::TileGrid,
};

/// Timestep and light radius of the game
const STEP: Duration = Duration::from_millis(16);
const RADIUS: f32 = 0.25;

/// Physics without wrapping anywhere near the scene.
fn new_physics<M>() -> Physics<M> {
    Physics::new(
        STEP.as_secs_f32(),
        ToricGeometry {
            x: 1000.0,
            y: 1000.0,
        },
    )
}

/// Scene objects for `segments`, built the same way as in the game.
fn scene(segments: &[Segment]) -> Vec<SceneObject> {
    segments
        .iter()
        .map(|s| {
            let (a, b) = s.ab();
            SceneObject::new_segment(a, b)
        })
        .collect()
}

fn segment(a: (f32, f32), b: (f32, f32)) -> SceneObject {
    SceneObject::new_segment(a.into(), b.into())
}

fn disc(center: (f32, f32), velocity: Vec2) -> PhysObject<()> {
    PhysObject::new_disc(center.into(), RADIUS, 1.0).with_velocity(velocity)
}

fn is_finite(p: Point) -> bool {
    p.x.is_finite() && p.y.is_finite()
}

// Timestep

/// Seconds simulated by advancing `frames` frames of `frame` each.
fn simulated_seconds(frame: Duration, frames: u32) -> f32 {
    let mut physics = new_physics();
    physics.add(disc((0.0, 0.0), Vec2::X));

    for _ in 0..frames {
        physics.advance_by(&[], frame);
    }

    physics.objects[0].center.x
}

#[test]
fn advance_by_whole_milliseconds_is_exact() {
    let simulated = simulated_seconds(Duration::from_millis(17), 1000);
    assert!((simulated - 17.0).abs() < 1e-3, "{simulated}");
}

#[test]
#[ignore = "advance_by truncates frame durations to whole milliseconds"]
fn advance_by_keeps_fractional_milliseconds() {
    let frames = 1000;

    let lost: Vec<_> = [900, 16_900]
        .map(Duration::from_micros)
        .into_iter()
        .map(|frame| {
            let real = frame.as_secs_f32() * frames as f32;
            let simulated = simulated_seconds(frame, frames);
            let lost = 1.0 - simulated / real;
            println!("{frame:?} frames: simulated {simulated} s of {real} s, lost {lost:.3}");
            lost
        })
        .collect();

    assert!(lost.iter().all(|lost| lost.abs() < 1e-3), "lost {lost:?}");
}

// Degenerate contacts

#[test]
#[ignore = "a disc centred exactly on a segment gets a NaN position"]
fn disc_centred_on_segment_stays_finite() {
    // `clamp_length` normalises the zero vector
    assert!(Vec2::ZERO.clamp_length(RADIUS, RADIUS).is_nan());

    let s = segment((-1.0, 0.0), (1.0, 0.0));

    // On the inner part and on the start of the segment
    for center in [(0.0, 0.0), (-1.0, 0.0)] {
        let mut d = disc(center, vec2(0.0, -1.0));
        d.collide(&s);
        assert!(is_finite(d.center), "{center:?} -> {}", d.center);
    }
}

// Tunnelling

/// Lowest of `speeds` at which a disc falling onto `scene` ends up below `top`, if any.
/// Each speed is tried with several phases of the timestep relative to the contact.
fn lowest_tunnelling_speed(
    scene: &[SceneObject],
    top: f32,
    mut speeds: impl Iterator<Item = f32>,
) -> Option<f32> {
    let phases = 16;

    speeds.find(|&speed| {
        (0..phases).any(|phase| {
            let offset = speed * STEP.as_secs_f32() * phase as f32 / phases as f32;

            let mut physics = new_physics();
            physics.add(disc((0.0, top + 1.0 + offset), vec2(0.0, -speed)));

            loop {
                physics.advance_by(scene, STEP);

                let d = &physics.objects[0];
                if d.velocity_linear.y > 0.0 || d.center.y < -5.0 {
                    break d.center.y < top;
                }
            }
        })
    })
}

/// A wide 1-unit-thick wall with its top at `y = 0.5`, and the map edges far away.
fn wall() -> Vec<SceneObject> {
    let (width, height) = (5, 15);
    let mut solid = vec![false; width * height];
    solid[7 * width..8 * width].fill(true);

    let grid = TileGrid {
        width: width as u32,
        height: height as u32,
        solid,
    };

    scene(&grid.segments())
}

#[test]
fn slow_disc_does_not_tunnel() {
    let speeds = (1..=60).map(|s| s as f32 * 0.25);

    let s = [segment((-2.0, 0.0), (2.0, 0.0))];
    assert_eq!(lowest_tunnelling_speed(&s, 0.0, speeds.clone()), None);
    assert_eq!(lowest_tunnelling_speed(&wall(), 0.5, speeds), None);
}

#[test]
#[ignore = "a disc moving more than its radius per step tunnels through segments and walls"]
fn fast_disc_does_not_tunnel() {
    let speeds = || (1..=200).map(|s| s as f32 * 0.25);

    let s = [segment((-2.0, 0.0), (2.0, 0.0))];
    let segment_speed = lowest_tunnelling_speed(&s, 0.0, speeds());
    let wall_speed = lowest_tunnelling_speed(&wall(), 0.5, speeds());

    println!("Radius per step: {} m/s", RADIUS / STEP.as_secs_f32());
    println!("Tunnels through a segment at {segment_speed:?} m/s, a wall at {wall_speed:?} m/s");

    assert_eq!((segment_speed, wall_speed), (None, None));
}

// Resting and sliding contact

#[test]
fn dropped_disc_comes_to_rest() {
    let floor = [segment((-2.0, 0.0), (2.0, 0.0))];

    let mut physics = new_physics();
    physics.add(disc((0.0, 1.0), Vec2::ZERO));

    let mut heights = Vec::new();
    for _ in 0..60_000 / STEP.as_millis() {
        physics.advance_by(&floor, STEP);
        heights.push(physics.objects[0].center.y);
    }

    let d = &physics.objects[0];
    let settled = heights.iter().rposition(|&y| y != d.center.y).unwrap() + 1;
    let settled = settled as f32 * STEP.as_secs_f32();
    let depth = RADIUS - d.center.y;
    let velocity = d.velocity_linear;

    println!("Settled after {settled} s, {depth} into the floor");
    println!("Resting with {velocity} m/s at the end of each step");

    assert!(settled < 30.0);
    assert!(depth.abs() < 1e-3);
    assert_eq!(d.center.x, 0.0);
}

/// Largest vertical speed of a disc sliding across `floor` with its top at `y = 0.5`.
fn sliding_vertical_speed(floor: &[SceneObject], speed: f32) -> f32 {
    let start = (-1.5 * speed.signum(), 0.5 + RADIUS);

    let mut physics = new_physics();
    physics.add(disc(start, vec2(speed, 0.0)));

    let mut highest = 0.0f32;
    for _ in 0..(3.0 / speed.abs() / STEP.as_secs_f32()) as usize {
        physics.advance_by(floor, STEP);
        highest = highest.max(physics.objects[0].velocity_linear.y.abs());
    }

    highest
}

#[test]
#[ignore = "endpoints between adjacent tile segments kick a sliding disc"]
fn sliding_across_tile_edges_is_smooth() {
    let single = [segment((-2.5, 0.5), (2.5, 0.5))];

    let mut kicks = Vec::new();
    for speed in [1.0, -1.0, 2.0, -2.0, 5.0, -5.0] {
        let smooth = sliding_vertical_speed(&single, speed);
        let tiles = sliding_vertical_speed(&wall(), speed);
        println!("Sliding at {speed} m/s: |vy| <= {tiles} on tiles, {smooth} on a single segment");
        kicks.push(tiles - smooth);
    }

    assert!(kicks.iter().all(|&kick| kick < 1e-3), "{kicks:?}");
}
