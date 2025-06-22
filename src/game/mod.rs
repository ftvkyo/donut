use std::time::Instant;

use glam::{vec2, vec4};

pub mod camera;
pub mod light;

use crate::game::{camera::Camera, light::Lights};

pub struct Game {
    pub stage_name: String,

    pub camera: Camera,
    pub lights: Lights,

    start: Instant,
}

impl Game {
    pub fn new() -> Self {
        let camera = Camera::new(vec2(4.0, 4.0));
        let lights = lights_at(0);

        Self {
            stage_name: "debug-01".into(),
            camera,
            lights,
            start: Instant::now(),
        }
    }

    pub fn advance(&mut self) {
        let elapsed = self.start.elapsed();
        self.lights = lights_at(elapsed.as_millis() % 3_000);
    }
}

fn lights_at(ms: u128) -> Lights {
    let mut lights = Lights::new();

    let t = ms as f32 / 1000.0;
    let b = 1.0 - (t / 3.0).min(0.9);

    let origin = vec4(4.0, 4.0, 0.25, 1.0);
    let gravity = vec4(0.0, -3.0, 0.0, 0.0);

    lights.0[0].position = origin + gravity * t * t + vec4(2.0, 1.0, 0.0, 0.0) * t;
    lights.0[0].color = vec4(b, 0.1, 0.1, 1.0);

    lights.0[1].position = origin + gravity * t * t + vec4(-2.0, 1.0, 0.0, 0.0) * t;
    lights.0[1].color = vec4(0.1, b, 0.1, 1.0);

    lights.0[2].position = origin + gravity * t * t + vec4(0.0, 4.0, 0.0, 0.0) * t;
    lights.0[2].color = vec4(0.1, 0.1, b, 1.0);

    lights.0[3].position = origin + gravity * t * t + vec4(1.0, 3.0, 0.0, 0.0) * t;
    lights.0[3].color = vec4(0.1, b, b, 1.0);

    lights.0[4].position = origin + gravity * t * t + vec4(-1.0, 3.0, 0.0, 0.0) * t;
    lights.0[4].color = vec4(b, 0.1, b, 1.0);

    lights
}
