use glam::Vec4;

use crate::{
    geo::Segment,
    view::gpu_struct::vertex::{VertexDeferred, VertexIndex},
};

pub struct DeferredLight {
    pub position: Vec4,
    pub color: Vec4,
    pub visibility: Vec<Segment>,
}

impl DeferredLight {
    pub fn vertex_data(&self) -> impl Iterator<Item = VertexDeferred> {
        let points = self
            .visibility
            .iter()
            .flat_map(|s| {
                let (a, b) = s.ab();
                [[a.x, a.y, self.position.z], [b.x, b.y, self.position.z]]
            })
            .map(|p| VertexDeferred {
                pos: [p[0], p[1], p[2], 1.0].into(),
                light_pos: self.position.into(),
                light_color: self.color.into(),
            });

        std::iter::once(VertexDeferred {
            pos: self.position.into(),
            light_pos: self.position.into(),
            light_color: self.color.into(),
        })
        .chain(points)
    }

    pub fn index_data(&self, offset: VertexIndex) -> impl Iterator<Item = VertexIndex> {
        (0..self.visibility.len()).flat_map(move |segment_i| {
            [
                // The central vertex
                offset,
                // The second vertex
                offset + segment_i as u16 * 2 + 1,
                // The third vertex
                offset + segment_i as u16 * 2 + 2,
            ]
        })
    }
}
