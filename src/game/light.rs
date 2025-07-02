use anyhow::{Result, ensure};
use bytemuck::Zeroable;
use glam::{Mat4, Vec4, vec2};

use crate::view::{
    Quad,
    light::{UNIFORM_LIGHTS, UniformLight, UniformLights},
};

pub struct Light {
    pub position: Vec4,
    pub color: Vec4,
    pub rotation: f32,
}

pub struct Lights {
    pub inner: Vec<Light>,
    pub tex_num: u32,
    pub frame_count: usize,
    pub frame_size: [usize; 2],
}

impl Lights {
    pub fn new(tex_num: u32, frame_count: usize, frame_size: [usize; 2]) -> Self {
        Self {
            inner: Vec::with_capacity(UNIFORM_LIGHTS),
            tex_num,
            frame_count,
            frame_size,
        }
    }

    pub fn uniform_data(&self, view: &Mat4) -> Result<UniformLights> {
        ensure!(self.inner.len() <= UNIFORM_LIGHTS);

        let mut uniform = UniformLights::zeroed();
        for (index, light) in self.inner.iter().enumerate() {
            uniform.0[index] = UniformLight {
                position: *view * light.position,
                color: light.color,
            };
        }

        Ok(uniform)
    }

    pub fn quad_data(&self, frame: usize) -> Result<Vec<Quad>> {
        ensure!(frame < self.frame_count);

        let mut quads = Vec::with_capacity(self.inner.len());

        let frame = frame as f32;
        let frame_w = self.frame_size[0] as f32;
        let frame_h = self.frame_size[1] as f32;

        for light in self.inner.iter() {
            quads.push(Quad {
                pos: light.position.truncate(),
                dim: vec2(1.0, 1.0),
                rot: light.rotation,
                tex_num: self.tex_num,
                tex_pos: vec2(frame_w * frame, 0.0),
                tex_dim: vec2(frame_w, frame_h),
            });
        }

        Ok(quads)
    }
}
