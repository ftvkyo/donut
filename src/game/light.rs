use anyhow::{Result, ensure};
use glam::{Vec4, Vec4Swizzles, vec2};

use crate::{
    assets::Map,
    view::{DeferredLight, QuadEmitter},
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
            inner: Vec::new(),
            tex_num,
            frame_count,
            frame_size,
        }
    }

    pub fn deferred_data(&self, map: &Map) -> Result<Vec<DeferredLight>> {
        let mut lights = Vec::with_capacity(self.inner.len());
        for light in &self.inner {
            let visibility = map.visibility_for(light.position.xy().into()).segments;
            lights.push(DeferredLight {
                position: light.position,
                color: light.color,
                visibility,
            });
        }
        Ok(lights)
    }

    pub fn quad_data(&self, frame: usize) -> Result<Vec<QuadEmitter>> {
        ensure!(frame < self.frame_count);

        let mut quads = Vec::with_capacity(self.inner.len());

        let frame = frame as f32;
        let frame_w = self.frame_size[0] as f32;
        let frame_h = self.frame_size[1] as f32;

        for light in self.inner.iter() {
            quads.push(QuadEmitter {
                pos: light.position.truncate(),
                dim: vec2(1.0, 1.0),
                rot: light.rotation,
                tex_num: self.tex_num,
                tex_pos: vec2(frame_w * frame, 0.0),
                tex_dim: vec2(frame_w, frame_h),
                tint: light.color.truncate(),
            });
        }

        Ok(quads)
    }
}
