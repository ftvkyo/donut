use crate::view::{Vertex, VertexIndex};

#[derive(Debug)]
pub struct Sprite {
    pub x: f32,
    pub y: f32,
    pub z: f32,

    pub w: f32,
    pub h: f32,

    pub tex_x: f32,
    pub tex_y: f32,

    pub tex_w: f32,
    pub tex_h: f32,
}

impl Sprite {
    pub fn vertex_data(&self) -> [Vertex; 4] {
        [
            Vertex {
                pos: [self.x, self.y, self.z, 1.0],
                tex_coord: [self.tex_x, self.tex_y + self.tex_h],
            },
            Vertex {
                pos: [self.x + self.w, self.y, self.z, 1.0],
                tex_coord: [self.tex_x + self.tex_w, self.tex_y + self.tex_h],
            },
            Vertex {
                pos: [self.x + self.w, self.y + self.h, self.z, 1.0],
                tex_coord: [self.tex_x + self.tex_w, self.tex_y],
            },
            Vertex {
                pos: [self.x, self.y + self.h, self.z, 1.0],
                tex_coord: [self.tex_x, self.tex_y],
            },
        ]
    }

    pub fn index_data(&self, offset: u16) -> [VertexIndex; 6] {
        [
            offset + 0,
            offset + 1,
            offset + 2,
            offset + 2,
            offset + 3,
            offset + 0,
        ]
    }
}
