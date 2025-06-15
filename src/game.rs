use crate::{
    assets::{Sprite, TextureData},
    view::Vertex,
};

pub struct Game {
    pub texture_color: TextureData,
    pub texture_normal: TextureData,
    pub sprites: Vec<Sprite>,
    pub shader: String,
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
