use std::time::Instant;

use glam::{Vec2, vec2};

use crate::{
    assets::{Sprite, Texture},
    renderer::Vertex,
};

pub struct Movement {
    position: Vec2,
    velocity: Vec2,

    pub accel_u: bool,
    pub accel_r: bool,
    pub accel_d: bool,
    pub accel_l: bool,

    last_update: Instant,
}

impl Movement {
    const ACCEL: f32 = 4.0;
    const DECEL: f32 = 2.0;

    pub fn new_at(position: Vec2) -> Self {
        Self {
            position,
            velocity: Vec2::ZERO,
            accel_u: false,
            accel_r: false,
            accel_d: false,
            accel_l: false,
            last_update: Instant::now(),
        }
    }

    pub fn get_position(&self) -> Vec2 {
        self.position
    }

    pub fn advance(&mut self) {
        let time_delta = self.last_update.elapsed().as_millis() as f32 / 1000.0;
        self.last_update = Instant::now();

        let dir_x = match (self.accel_r, self.accel_l) {
            (true, false) => 1.0,
            (false, true) => -1.0,
            _ => 0.0,
        };

        let dir_y = match (self.accel_u, self.accel_d) {
            (true, false) => 1.0,
            (false, true) => -1.0,
            _ => 0.0,
        };

        let dir = vec2(dir_x, dir_y).normalize_or_zero();
        if dir.x == 0.0 && dir.y == 0.0 {
            let dir_decel = -self.velocity.normalize_or_zero();
            self.velocity += dir_decel * Self::DECEL * time_delta;
        } else {
            self.velocity += dir * Self::ACCEL * time_delta;
        }

        self.position += self.velocity * time_delta;
    }
}

pub struct Game {
    pub texture: Texture,
    pub sprites: Vec<Sprite>,
    pub movement: Movement,
}

impl Game {
    pub fn vertex_data(&self) -> (Vec<Vertex>, Vec<u16>) {
        let mut vertex_data = Vec::with_capacity(self.sprites.len() * 4);
        let mut index_data = Vec::with_capacity(self.sprites.len() * 6);

        for (i, sprite) in self.sprites.iter().enumerate() {
            vertex_data.extend_from_slice(&sprite.vertex_data());
            index_data.extend_from_slice(&sprite.index_data(i as u16 * 4));
        }

        (vertex_data, index_data)
    }
}
