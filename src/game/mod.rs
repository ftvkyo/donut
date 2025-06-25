use std::time::Instant;

use anyhow::{Context, Result};
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
    pub stage_name: String,

    pub camera: Camera,
    pub lights: Lights,

    start: Instant,
}

impl Game {
    pub fn new(assets: &Assets) -> Result<Self> {
        let camera = Camera::new(vec2(4.0, 4.0));

        let lights = assets
            .lights
            .get("fire")
            .context("No animation with the name 'fire'?")?;

        let mut game = Self {
            stage_name: "debug-01".into(),
            camera,
            lights: Lights::new(lights.frame_count, lights.frame_size),
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

        let origin = vec3(4.0, 4.0, 0.25);
        let gravity = vec2(0.0, -3.0);

        let v0s = [
            vec2(2.0, 1.0),
            vec2(-2.0, 1.0),
            vec2(0.0, 4.0),
            vec2(1.0, 3.0),
            vec2(-1.0, 3.0),
        ];

        let colors = [
            vec4(b, 0.1, 0.1, 1.0),
            vec4(0.1, b, 0.1, 1.0),
            vec4(0.1, 0.1, b, 1.0),
            vec4(0.1, b, b, 1.0),
            vec4(b, 0.1, b, 1.0),
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
