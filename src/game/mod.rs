use glam::vec2;

pub mod camera;
pub mod light;

use crate::game::{camera::Camera, light::Lights};

pub struct Game {
    pub stage_name: String,

    pub camera: Camera,
    pub lights: Lights,
}

impl Game {
    pub fn new() -> Self {
        let camera = Camera::new(vec2(4.0, 4.0));
        let lights = Lights::new();

        Self {
            stage_name: "debug-01".into(),
            camera,
            lights,
        }
    }
}
