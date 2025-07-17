use std::time::Duration;

use glam::{Vec4, Vec4Swizzles, vec2};

use crate::{
    assets::{LightSource, Map},
    view::{DeferredLight, QuadEmitter},
};

pub struct LightObject {
    pub position: Vec4,
    pub color: Vec4,
    pub rotation: f32,
}

pub struct LightCollection<'a> {
    pub objects: Vec<LightObject>,

    light_id: u32,
    light: &'a LightSource,

    ms_per_frame: usize,
}

impl<'a> LightCollection<'a> {
    pub fn new(light_id: u32, light: &'a LightSource) -> Self {
        let ms_per_frame = 1000 / light.frames_per_second;
        Self {
            objects: Vec::new(),
            light_id,
            light,
            ms_per_frame,
        }
    }

    pub fn deferred_data(&self, map: &Map) -> impl ExactSizeIterator<Item = DeferredLight> {
        let mut lights = Vec::with_capacity(self.objects.len());
        for light in &self.objects {
            let visibility = map.visibility_for(light.position.xy().into()).segments;
            lights.push(DeferredLight {
                position: light.position,
                color: light.color,
                visibility,
            });
        }
        lights.into_iter()
    }

    pub fn quad_data(&self, time: Duration) -> impl ExactSizeIterator<Item = QuadEmitter> {
        let mut quads = Vec::with_capacity(self.objects.len());

        let time_ms = (time.as_millis() % usize::MAX as u128) as usize;
        let frame = (time_ms / self.ms_per_frame) % self.light.frames;

        let frame_w = self.light.frame_size[0] as f32;
        let frame_h = self.light.frame_size[1] as f32;

        for light in self.objects.iter() {
            quads.push(QuadEmitter {
                pos: light.position.truncate(),
                dim: vec2(1.0, 1.0),
                rot: light.rotation,
                tex_num: self.light_id,
                tex_pos: vec2(frame_w * frame as f32, 0.0),
                tex_dim: vec2(frame_w, frame_h),
                tint: light.color.truncate(),
            });
        }

        quads.into_iter()
    }
}
