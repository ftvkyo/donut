use std::time::Instant;

use anyhow::Result;
use glam::{vec2, vec3, vec4};

pub mod camera;
pub mod light;

use crate::{
    assets::Assets,
    game::{
        camera::Camera,
        light::{Light, Lights},
    },
};

pub struct Game {
    pub map_num: usize,

    pub camera: Camera,
    pub lights: Lights,

    start: Instant,
}

impl Game {
    pub fn new(assets: &Assets) -> Result<Self> {
        let map_name = "debug-01";
        let (map_num, map) = assets.find_map(map_name)?;

        let camera = Camera::new(vec2(0.0, 0.0), map.size_tiles());

        let light_name = "fire";
        let (ilight, light) = assets.find_light(light_name)?;

        let mut game = Self {
            map_num,
            camera,
            lights: Lights::new(ilight as u32, light.frame_count, light.frame_size),
            start: Instant::now(),
        };
        game.set_lights_at(0);

        Ok(game)
    }

    pub fn elapsed(&self) -> std::time::Duration {
        self.start.elapsed()
    }

    pub fn advance(&mut self) {
        self.set_lights_at(self.elapsed().as_millis() % 3_000);
    }

    fn set_lights_at(&mut self, ms: u128) {
        self.lights.inner.clear();

        let t = ms as f32 / 1000.0;
        let b = 1.0 - (t / 3.0).min(0.9);

        let origin = vec3(0.0, 0.0, 0.25);
        let gravity = vec2(0.0, -3.0);

        let v0s = [
            vec2(2.0, 1.0),
            vec2(-2.0, 1.0),
            vec2(0.0, 4.0),
            vec2(1.0, 3.0),
            vec2(-1.0, 3.0),
            vec2(3.0, 2.0),
            vec2(-3.0, 2.0),
        ];

        let colors = [
            vec4(b, 0.1, 0.1, 1.0),
            vec4(0.1, b, 0.1, 1.0),
            vec4(0.1, 0.1, b, 1.0),
            vec4(0.1, b, b, 1.0),
            vec4(b, 0.1, b, 1.0),
            vec4(b, b, 0.0, 1.0),
            vec4(b, b, 0.0, 1.0),
        ];

        for (v0, color) in v0s.into_iter().zip(colors.into_iter()) {
            let pos = origin + v0.extend(0.0) * t + gravity.extend(0.0) * t * t;
            let vel = v0 + 2.0 * gravity * t;

            let rotation = vel.y.atan2(vel.x);

            self.lights.inner.push(Light {
                position: pos.extend(1.0),
                color,
                rotation,
            });
        }
    }
}
