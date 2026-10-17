use std::{
    f32::consts::{FRAC_PI_8, PI},
    time::Instant,
};

use anyhow::Result;
use glam::{Vec2, Vec3, Vec4, vec2};
use palette::{FromColor, LinSrgb, OklabHue, Oklch};

pub mod camera;

use crate::{
    assets::{Assets, LightSource, Map, Shape},
    game::camera::Camera,
    geo::{Point, ToricGeometry},
    phys::{
        Physics,
        object::{PhysObject, SceneObject},
    },
    view::{DeferredLight, QuadEmitter},
};

const LIGHT_COUNT: usize = 12;

pub enum GameObject<'assets> {
    Light {
        color: Vec4,
        light_id: u32,
        light_asset: &'assets LightSource,
    },
}

pub struct Game<'assets> {
    pub map: &'assets Map,
    pub camera: Camera,

    physics: Physics<GameObject<'assets>>,
    physics_scene: Vec<SceneObject>,

    start: Instant,
    last_advance: Instant,
}

impl<'assets> Game<'assets> {
    pub fn new(assets: &'assets Assets) -> Result<Self> {
        let map_name = "debug-01";
        let (_, map) = assets.find_map(map_name)?;

        let camera = Camera::new(vec2(0.0, 0.0), map.size_tiles());

        let light_name = "fire";
        let (light_id, light_asset) = assets.find_light(light_name)?;

        let map_size = map.size_tiles();
        let mut physics = Physics::new(
            assets.max_timestep,
            ToricGeometry {
                x: map_size.width as f32,
                y: map_size.height as f32,
            },
        );

        let light_brightness = 2.5;
        let light_angle_start = FRAC_PI_8;
        let light_angle_end = PI - FRAC_PI_8;

        for light_i in 0..LIGHT_COUNT {
            let angle = light_angle_start
                + (light_angle_end - light_angle_start)
                    * (1.0 - light_i as f32 / (LIGHT_COUNT - 1).max(1) as f32);
            let (angle_sin, angle_cos) = angle.sin_cos();
            let v0 = vec2(angle_cos, angle_sin) * 5.0;

            let time = 1.0;
            let origin = Point::ZERO + v0 * time + vec2(0.0, -3.0) * time * time;
            let velocity = v0 + 2.0 * vec2(0.0, -3.0) * time;

            let ok_hue = OklabHue::new(360.0 / (LIGHT_COUNT - 1) as f32 * light_i as f32);
            let ok_color = Oklch {
                l: 1.0,
                chroma: 0.4,
                hue: ok_hue,
            };
            let lsrgb_color = LinSrgb::from_color(ok_color);
            let color: [f32; 3] = lsrgb_color.into();
            let color: Vec3 = color.into();

            let meta = GameObject::Light {
                color: color.extend(light_brightness),
                light_id: light_id as u32,
                light_asset,
            };

            let Shape::Disc { radius } = light_asset.shape;

            let obj = PhysObject::new_disc(origin, radius, 1.0)
                .with_meta(meta)
                .with_velocity(velocity);

            physics.add(obj);
        }

        let physics_scene = map
            .occlusion_segments
            .iter()
            .map(|s| {
                let (a, b) = s.ab();
                SceneObject::new_segment(a, b)
            })
            .collect();

        Ok(Self {
            map,
            camera,
            physics,
            physics_scene,
            start: Instant::now(),
            last_advance: Instant::now(),
        })
    }

    pub fn advance(&mut self) {
        self.physics
            .advance_by(&self.physics_scene, self.last_advance.elapsed());
        self.last_advance = Instant::now();
    }

    pub fn light_deferred_data(&self) -> impl ExactSizeIterator<Item = DeferredLight> {
        self.physics.iter().map(|obj| {
            let GameObject::Light { color, .. } = obj.meta;
            let pos = obj.center;
            let visibility = self.map.visibility_for(pos).segments;
            DeferredLight {
                position: (pos.x, pos.y, 1.0, 1.0).into(),
                color,
                visibility,
            }
        })
    }

    pub fn light_quad_data(&self) -> impl ExactSizeIterator<Item = QuadEmitter> {
        let time_ms = (self.start.elapsed().as_millis() % usize::MAX as u128) as usize;
        self.physics.iter().map(move |obj| {
            let pos = obj.center;
            let GameObject::Light {
                color,
                light_id,
                light_asset,
            } = obj.meta;

            let rot = Vec2::X.angle_to(obj.velocity_linear);

            let frame = (time_ms / light_asset.ms_per_frame) % light_asset.frames;
            let frame_w = light_asset.frame_size[0] as f32;
            let frame_h = light_asset.frame_size[1] as f32;

            QuadEmitter {
                pos: (pos.x, pos.y, 1.0).into(),
                dim: vec2(1.0, 1.0),
                rot,
                tex_num: light_id,
                tex_pos: vec2(frame_w * frame as f32, 0.0),
                tex_dim: vec2(frame_w, frame_h),
                tint: color.truncate(),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{panic, path::Path, time::Duration};

    use glam::DVec2;

    use crate::{
        assets::Config,
        phys::object::PhysObjectShape,
        testing::{panic_message, ray_cast},
    };

    use super::*;

    fn assets() -> Assets {
        let dir_assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
        let config = Config::load(dir_assets.join("config.toml")).unwrap();
        Assets::resolve(config, dir_assets).unwrap()
    }

    #[test]
    fn lights_stay_below_tunnelling_speed() {
        let assets = assets();
        let mut game = Game::new(&assets).unwrap();

        // The longest step the physics takes with the game's `max_timestep`
        let step = Duration::from_millis(16);

        let mut fastest = 0.0f32;
        let mut tunnelled = 0;

        for _ in 0..60_000 / step.as_millis() {
            let before: Vec<_> = game.physics.iter().map(|obj| obj.center).collect();
            game.physics.advance_by(&game.physics_scene, step);

            for (obj, before) in game.physics.iter().zip(before) {
                fastest = fastest.max(obj.velocity_linear.length());

                // Moves across the wrap seam are ignored
                let moved = before.dir(obj.center);
                if moved.length() > 1.0 || moved == Vec2::ZERO {
                    continue;
                }

                let dir = DVec2::new(moved.x as f64, moved.y as f64);
                if ray_cast(before, dir, &game.map.occlusion_segments)
                    .is_some_and(|(t, _)| t <= moved.length() as f64)
                {
                    tunnelled += 1;
                }
            }
        }

        let PhysObjectShape::Disc { radius } = game.physics.iter().next().unwrap().shape;
        let threshold = radius / step.as_secs_f32();
        println!("Fastest light: {fastest} m/s, tunnelling from {threshold} m/s");
        println!("Light centres crossed a segment {tunnelled} times");

        assert!(fastest < threshold);
        assert_eq!(tunnelled, 0);
    }

    /// Plays `seconds` of the game in 16 ms steps, computing the visibility of every light
    /// after each step like `light_deferred_data` does.
    /// Returns the time, light position and message of every panic.
    fn play(seconds: u64) -> Vec<(Duration, Point, String)> {
        let assets = assets();
        let mut game = Game::new(&assets).unwrap();

        let step = Duration::from_millis(16);
        let mut time = Duration::ZERO;
        let mut panics = Vec::new();

        while time < Duration::from_secs(seconds) {
            game.physics.advance_by(&game.physics_scene, step);
            time += step;

            for obj in game.physics.iter() {
                if let Err(payload) = panic::catch_unwind(|| game.map.visibility_for(obj.center)) {
                    panics.push((time, obj.center, panic_message(payload)));
                }
            }
        }

        panics
    }

    /// `cargo test --release --lib lights_have_visibility -- --ignored`
    #[test]
    #[ignore = "slow, and panics once in 225000 light updates, at 119 s"]
    fn lights_have_visibility_for_5_minutes() {
        let panics = play(300);
        assert!(panics.is_empty(), "{} panics: {panics:#?}", panics.len());
    }
}
