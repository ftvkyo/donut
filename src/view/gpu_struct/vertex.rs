use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub pos: [f32; 4],
    pub tex_coord: [f32; 2],
}

pub type VertexIndex = u16;
