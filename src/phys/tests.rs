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
fn physics<M>() -> Physics<M> {
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
    let mut physics = physics();
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
