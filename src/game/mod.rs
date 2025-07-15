use std::{
    f32::consts::{FRAC_PI_8, PI},
    time::Instant,
};

use anyhow::Result;
use glam::{Vec3, vec2, vec3};

pub mod camera;
pub mod geo;
pub mod light;

use crate::{
    assets::{Assets, Map},
    game::{
        camera::Camera,
        light::{Light, Lights},
    },
};

const LIGHT_COUNT: usize = 16;
const LIGHT_COLORS: [Vec3; LIGHT_COUNT] = [
    vec3(1.0, 0.1, 0.1),
    vec3(0.1, 1.0, 0.1),
    vec3(0.1, 0.1, 1.0),
    vec3(0.1, 1.0, 1.0),
    vec3(1.0, 0.1, 1.0),
    vec3(1.0, 1.0, 0.0),
    vec3(1.0, 1.0, 0.0),
    vec3(1.0, 0.1, 0.1),
    vec3(0.1, 1.0, 0.1),
    vec3(0.1, 0.1, 1.0),
    vec3(0.1, 1.0, 1.0),
    vec3(1.0, 0.1, 1.0),
    vec3(1.0, 1.0, 0.0),
    vec3(1.0, 1.0, 0.0),
    vec3(1.0, 1.0, 1.0),
    vec3(1.0, 1.0, 1.0),
];

pub struct Game<'m> {
    pub map: &'m Map,

    pub camera: Camera,
    pub lights: Lights,

    start: Instant,
}

impl<'a> Game<'a> {
    pub fn new(assets: &'a Assets) -> Result<Self> {
        let map_name = "debug-01";
        let (_, map) = assets.find_map(map_name)?;

        let camera = Camera::new(vec2(0.0, 0.0), map.size_tiles());

        let light_name = "fire";
        let (ilight, light) = assets.find_light(light_name)?;

        let mut game = Self {
            map,
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
        // self.set_lights_at(self.elapsed().as_millis() % 4_000);
        self.set_lights_at(1500);
    }

    fn set_lights_at(&mut self, ms: u128) {
        self.lights.inner = Vec::with_capacity(LIGHT_COUNT);

        let time = ms as f32 / 1000.0;
        let brightness = 1.0 - (time / 5.0).min(0.9);

        let origin = vec3(0.0, 0.0, 0.25);
        let gravity = vec2(0.0, -3.0);

        let v0 = 9.0;
        let angle_start = FRAC_PI_8;
        let angle_end = PI - FRAC_PI_8;

        for light_i in 0..LIGHT_COUNT {
            let angle = angle_start
                + (angle_end - angle_start)
                    * (1.0 - light_i as f32 / (LIGHT_COUNT - 1).max(1) as f32);
            let (angle_sin, angle_cos) = angle.sin_cos();
            let v0 = vec2(angle_cos, angle_sin) * v0;

            let color = palette::Oklch {
                l: 0.7,
                chroma: 0.2,
                hue: 360.0 / (LIGHT_COUNT - 1) as f32 * light_i as f32,
            };

            let position = origin + v0.extend(0.0) * time + gravity.extend(0.0) * time * time;

            let velocity = v0 + 2.0 * gravity * time;
            let rotation = velocity.y.atan2(velocity.x);

            self.lights.inner.push(Light {
                position: position.extend(1.0),
                color,
                rotation,
            });
        }
    }
}
