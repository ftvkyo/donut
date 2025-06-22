use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec4};

pub const LIGHT_COUNT: usize = 32;

#[derive(Pod, Zeroable, Clone, Copy)]
#[repr(C)]
pub struct Light {
    pub position: Vec4,
    pub color: Vec4,
}

pub struct Lights(pub [Light; LIGHT_COUNT]);

impl Lights {
    pub fn new() -> Self {
        Self([Light::zeroed(); LIGHT_COUNT])
    }

    pub fn data(&self, view: Mat4) -> Vec<u8> {
        let lights = self.0.map(|l| Light {
            position: view * l.position,
            color: l.color,
        });
        Vec::from(bytemuck::cast_slice(&lights))
    }
}
