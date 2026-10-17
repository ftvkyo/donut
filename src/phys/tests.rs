//! Behaviour of the physics with the game's timestep and object sizes.

use std::time::Duration;

use glam::{Vec2, vec2};

use crate::{
    assets::collision_segments,
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
fn disc_centred_on_segment_stays_finite() {
    let mut d = disc((0.0, 0.0), vec2(0.0, -1.0));
    d.collide(&segment((-1.0, 0.0), (1.0, 0.0)));

    assert!(d.center.vec().is_finite(), "{}", d.center);

    // Pushed back against its velocity
    assert_eq!(d.center, Point::new(0.0, RADIUS));
    assert!(d.velocity_linear.y > 0.0, "{}", d.velocity_linear);
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

#[test]
fn disc_slides_without_friction() {
    let floor = [segment((-10.0, 0.0), (10.0, 0.0))];

    let mut physics = new_physics();
    physics.add(disc((-5.0, RADIUS), Vec2::X));

    for _ in 0..5000 / STEP.as_millis() {
        physics.advance_by(&floor, STEP);
    }

    assert!((physics.objects[0].velocity_linear.x - 1.0).abs() < 1e-5);
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

// Simultaneous contacts with segments

/// The disc after colliding with `segments` in the given order.
fn collided(mut d: PhysObject<()>, segments: &[&SceneObject]) -> PhysObject<()> {
    for s in segments {
        d.collide(*s);
    }
    d
}

#[test]
fn disc_in_inner_corner_is_order_independent() {
    let floor = segment((0.0, 0.0), (1.0, 0.0));
    let wall = segment((0.0, 0.0), (0.0, 1.0));
    let d = disc((0.2, 0.2), vec2(-1.0, -1.0));

    for d in [
        collided(d.clone(), &[&floor, &wall]),
        collided(d, &[&wall, &floor]),
    ] {
        assert_eq!(d.center, Point::new(RADIUS, RADIUS));
        assert_eq!(d.velocity_linear, vec2(0.9, 0.9));
    }
}

#[test]
fn disc_above_joint_is_corrected_once() {
    let left = segment((-1.0, 0.0), (0.0, 0.0));
    let right = segment((0.0, 0.0), (1.0, 0.0));
    let d = collided(disc((0.0, 0.2), vec2(0.0, -1.0)), &[&left, &right]);

    assert_eq!(d.center, Point::new(0.0, RADIUS));
    assert_eq!(d.velocity_linear, vec2(0.0, 0.9));
}

#[test]
#[ignore = "an endpoint contact processed before an inner contact tilts the response"]
fn disc_near_joint_is_order_independent() {
    let left = segment((-1.0, 0.0), (0.0, 0.0));
    let right = segment((0.0, 0.0), (1.0, 0.0));
    let d = disc((0.05, 0.2), vec2(-1.0, -1.0));

    let d1 = collided(d.clone(), &[&left, &right]);
    let d2 = collided(d, &[&right, &left]);

    println!("Endpoint first: {} {}", d1.center, d1.velocity_linear);
    println!("Inner first: {} {}", d2.center, d2.velocity_linear);

    assert!(d1.center.dist(d2.center) < 1e-6);
    assert!(d1.velocity_linear.distance(d2.velocity_linear) < 1e-6);
}

// Collisions between discs

/// Physics with `discs` after a single collision pass between them, without gravity or motion.
fn bounced<M>(discs: impl IntoIterator<Item = PhysObject<M>>) -> Physics<M> {
    let mut physics = new_physics();
    for d in discs {
        physics.add(d);
    }

    physics.for_each_distinct_pair_mut(|d1, d2| d1.collide(d2));
    physics.for_each_mut(PhysObject::flush_acc);

    physics
}

fn momentum<'d, M: 'd>(discs: impl IntoIterator<Item = &'d PhysObject<M>>) -> Vec2 {
    discs.into_iter().map(|d| d.velocity_linear * d.mass).sum()
}

fn energy<'d, M: 'd>(discs: impl IntoIterator<Item = &'d PhysObject<M>>) -> f32 {
    discs
        .into_iter()
        .map(|d| d.velocity_linear.length_squared() * d.mass / 2.0)
        .sum()
}

/// A disc moving at `(1, 0)` with mass `m1` into a disc at rest with mass `m2`.
fn head_on(m1: f32, m2: f32) -> [PhysObject<()>; 2] {
    [
        PhysObject::new_disc((0.0, 0.0).into(), RADIUS, m1).with_velocity(Vec2::X),
        PhysObject::new_disc((2.0 * RADIUS, 0.0).into(), RADIUS, m2),
    ]
}

#[test]
fn disc_collisions_conserve_momentum() {
    let oblique = [
        disc((0.0, 0.0), vec2(1.0, 0.5)),
        PhysObject::new_disc((0.4, 0.3).into(), RADIUS, 3.0).with_velocity(vec2(-0.5, 0.0)),
    ];

    for discs in [head_on(1.0, 1.0), head_on(2.0, 1.0), oblique] {
        let before = momentum(&discs);
        let after = momentum(bounced(discs).objects());
        assert!(before.distance(after) < 1e-6, "{before} -> {after}");
    }
}

#[test]
fn equal_disc_collision_conserves_energy() {
    let discs = head_on(1.0, 1.0);
    let before = energy(&discs);

    let physics = bounced(discs);
    let after = energy(physics.objects());

    assert_eq!(before, after);
    assert_eq!(physics.objects[0].velocity_linear, Vec2::ZERO);
    assert_eq!(physics.objects[1].velocity_linear, Vec2::X);
}

#[test]
fn unequal_disc_collision_conserves_energy() {
    let ratios: Vec<_> = [(2.0, 1.0), (1.0, 2.0)]
        .into_iter()
        .map(|(m1, m2)| {
            let discs = head_on(m1, m2);
            let before = energy(&discs);
            let after = energy(bounced(discs).objects());
            println!("Masses {m1} and {m2}: energy {before} -> {after}");
            after / before
        })
        .collect();

    assert!(ratios.iter().all(|r| (r - 1.0).abs() < 1e-6), "{ratios:?}");
}

#[test]
#[ignore = "discs touching on both sides at once stop instead of bouncing"]
fn simultaneous_disc_collisions_conserve_energy() {
    let discs = [
        disc((-2.0 * RADIUS, 0.0), Vec2::X),
        disc((0.0, 0.0), Vec2::ZERO),
        disc((2.0 * RADIUS, 0.0), Vec2::NEG_X),
    ];
    let before = energy(&discs);

    let physics = bounced(discs);
    let after = energy(physics.objects());

    for d in physics.objects() {
        println!("{} at {}", d.velocity_linear, d.center);
    }

    assert_eq!(before, after);
}

#[test]
fn disc_collisions_are_pair_order_independent() {
    // Three discs touching each other, with different masses and velocities
    let discs = [
        ((0.0, 0.0), vec2(1.0, 0.5), 1.0),
        ((2.0 * RADIUS, 0.0), vec2(-1.0, 0.0), 2.0),
        ((RADIUS, 3.0f32.sqrt() * RADIUS), vec2(0.0, -2.0), 3.0),
    ]
    .map(|(center, velocity, mass)| {
        PhysObject::new_disc(center.into(), RADIUS, mass).with_velocity(velocity)
    });

    let reference = bounced(discs.clone());

    for order in [[0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0]] {
        let physics = bounced(order.map(|i| discs[i].clone().with_meta(i)));

        for d in physics.objects() {
            let expected = reference.objects[d.meta].velocity_linear;
            assert!(d.velocity_linear.distance(expected) < 1e-6, "{order:?}");
        }
    }
}

#[test]
fn stacked_discs_keep_apart() {
    let floor = [segment((-2.0, 0.0), (2.0, 0.0))];

    let mut physics = new_physics();
    physics.add(disc((0.0, RADIUS), Vec2::ZERO));
    physics.add(disc((0.0, 3.0 * RADIUS), Vec2::ZERO));

    for _ in 0..10_000 / STEP.as_millis() {
        physics.advance_by(&floor, STEP);
    }

    let [d1, d2] = &physics.objects[..] else {
        unreachable!()
    };
    let distance = d1.center.dist(d2.center);
    println!("Discs at {} and {}, {distance} apart", d1.center, d2.center);

    // Resting discs overlap by about what they fall in a step
    assert!(distance > 2.0 * RADIUS - 2e-3);
}

// Toric wrapping

fn toric_physics<M>(grid: &TileGrid) -> Physics<M> {
    Physics::new(
        STEP.as_secs_f32(),
        ToricGeometry {
            x: grid.width as f32,
            y: grid.height as f32,
        },
    )
}

#[test]
fn wrapping_disc_never_reaches_map_edges() {
    let grid = TileGrid {
        width: 8,
        height: 6,
        solid: vec![false; 8 * 6],
    };
    let scene = scene(&grid.segments());
    let velocity = vec2(5.0, 2.0);

    let mut physics = toric_physics(&grid);
    physics.add(disc((0.0, 0.0), velocity));

    for step in 1..=5000 / STEP.as_millis() {
        physics.advance_by(&scene, STEP);

        let d = &physics.objects[0];
        let fall = Physics::<()>::G * STEP.as_secs_f32() * step as f32;
        assert!(d.center.x.abs() <= 4.0 && d.center.y.abs() <= 3.0);
        assert!(d.velocity_linear.distance(velocity + fall) < 1e-4);
    }
}

#[test]
fn disc_bounces_off_tiles_across_wrap_seam() {
    let (width, height) = (8, 4);

    // A floor, and a block on it at the right or the left map edge
    for (block, speed) in [(width - 1, -2.0), (0, 2.0)] {
        let mut solid = vec![false; width * height];
        solid[3 * width..].fill(true);
        solid[2 * width + block] = true;

        let grid = TileGrid {
            width: width as u32,
            height: height as u32,
            solid,
        };
        let segments = collision_segments(grid.width, grid.height, &[|x, y| grid.is_solid(x, y)]);
        let scene = scene(&segments);

        let mut physics = toric_physics(&grid);
        physics.add(disc((0.0, -1.0 + RADIUS), vec2(speed, 0.0)));

        let mut bounced = false;
        for step in 0..3000 / STEP.as_millis() {
            physics.advance_by(&scene, STEP);

            let d = &physics.objects[0];
            assert!(
                !grid.is_solid_at(d.center),
                "Block at {block}: in a solid tile at {} after {step} steps",
                d.center
            );

            bounced |= d.velocity_linear.x * speed < 0.0;
        }

        assert!(bounced, "Block at {block}: no bounce");
    }
}

#[test]
fn discs_collide_across_wrap_seam() {
    let grid = TileGrid::from_rows(&["........"; 4]);

    let mut physics = toric_physics(&grid);
    physics.add(disc((-4.0 + RADIUS * 0.75, 0.0), Vec2::NEG_X));
    physics.add(disc((4.0 - RADIUS * 0.75, 0.0), Vec2::X));

    physics.advance_by(&[], STEP);

    let [d1, d2] = &physics.objects[..] else {
        unreachable!()
    };
    assert!(d1.velocity_linear.x > 0.0, "{}", d1.velocity_linear);
    assert!(d2.velocity_linear.x < 0.0, "{}", d2.velocity_linear);
}
