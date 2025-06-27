use glam::Vec2;
use glam::Vec3;
use glam::vec2;

use crate::view::gpu_struct::vertex::Vertex;
use crate::view::gpu_struct::vertex::VertexIndex;

pub struct Quad {
    /// Position of the center of the quad
    pub pos: Vec3,
    /// Width and Height of the quad
    pub dim: Vec2,
    /// Rotation of the quad around the Z axis.
    /// Positive values correspond to clockwise rotation.
    // TODO: account for rotation in light calculation (normal map is not rotated)
    pub rot: f32,

    /// Position of the top-left corner of the corresponding texture quad
    pub tex_pos: Vec2,
    /// Width and Height of the corresponding texture quad
    pub tex_dim: Vec2,
}

impl Quad {
    pub fn vertex_data(&self) -> [Vertex; 4] {
        let normal = [0.0, 0.0, 1.0];

        let w2 = self.dim.x / 2.0;
        let h2 = self.dim.y / 2.0;

        let (rot_sin, rot_cos) = self.rot.sin_cos();
        let rot = vec2(rot_cos, rot_sin);

        let vpos = [vec2(-w2, -h2), vec2(w2, -h2), vec2(w2, h2), vec2(-w2, h2)].map(|offset| {
            let offset = rot.rotate(offset).extend(0.0);
            (self.pos + offset).extend(1.0)
        });

        let tw = self.tex_dim.x;
        let th = self.tex_dim.y;

        let tpos = [vec2(0.0, th), vec2(tw, th), vec2(tw, 0.0), vec2(0.0, 0.0)]
            .map(|offset| self.tex_pos + offset);

        [
            Vertex {
                pos: vpos[0].into(),
                normal,
                tex_coord: tpos[0].into(),
            },
            Vertex {
                pos: vpos[1].into(),
                normal,
                tex_coord: tpos[1].into(),
            },
            Vertex {
                pos: vpos[2].into(),
                normal,
                tex_coord: tpos[2].into(),
            },
            Vertex {
                pos: vpos[3].into(),
                normal,
                tex_coord: tpos[3].into(),
            },
        ]
    }

    pub fn index_data(&self, offset: VertexIndex) -> [VertexIndex; 6] {
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

pub fn quads2vidata(quads: &[Quad]) -> (Vec<Vertex>, Vec<VertexIndex>) {
    let mut vdata = Vec::with_capacity(quads.len() * 4);
    let mut idata = Vec::with_capacity(quads.len() * 6);
    for (i, quad) in quads.iter().enumerate() {
        vdata.extend_from_slice(&quad.vertex_data());
        idata.extend_from_slice(&quad.index_data(i as u16 * 4));
    }
    (vdata, idata)
}
