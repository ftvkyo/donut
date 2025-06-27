use bytemuck::{Pod, Zeroable};
use glam::Vec4;

/// Data for light calculations.
// Member alignment should be a power of 2: https://www.w3.org/TR/WGSL/#alignment-and-size
#[derive(Pod, Zeroable, Clone, Copy)]
#[repr(C)]
pub struct UniformLight {
    pub position: Vec4,
    pub color: Vec4,
}

pub const UNIFORM_LIGHTS: usize = 32;

#[derive(Pod, Zeroable, Clone, Copy)]
#[repr(C)]
pub struct UniformLights(pub [UniformLight; UNIFORM_LIGHTS]);
