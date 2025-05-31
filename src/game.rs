use crate::renderer::Vertex;

pub struct Tile {
    position: [i16; 2],
    texture: [u16; 2],
}

pub struct Level {
    tiles: Vec<Tile>,
}

impl Level {
    pub fn vertex_data(&self, tile_size: u16) -> (Vec<Vertex>, Vec<u16>) {
        let v = |pos: [i16; 3], tc: [u16; 2]| -> Vertex {
            Vertex {
                pos: [pos[0] as f32, pos[1] as f32, pos[2] as f32, 1.0],
                tex_coord: [(tc[0] * tile_size) as f32, (tc[1] * tile_size) as f32],
            }
        };

        let mut vertex_data = Vec::with_capacity(self.tiles.len() * 4);
        let mut index_data = Vec::with_capacity(self.tiles.len() * 6);

        for (i, tile) in self.tiles.iter().enumerate() {
            let i = i as u16;

            vertex_data.extend_from_slice(&[
                v(
                    [tile.position[0], tile.position[1], 0],
                    [tile.texture[0], tile.texture[1] + 1],
                ),
                v(
                    [tile.position[0] + 1, tile.position[1], 0],
                    [tile.texture[0] + 1, tile.texture[1] + 1],
                ),
                v(
                    [tile.position[0] + 1, tile.position[1] + 1, 0],
                    [tile.texture[0] + 1, tile.texture[1]],
                ),
                v(
                    [tile.position[0], tile.position[1] + 1, 0],
                    [tile.texture[0], tile.texture[1]],
                ),
            ]);

            index_data.extend_from_slice(&[
                i * 4 + 0,
                i * 4 + 1,
                i * 4 + 2,
                i * 4 + 2,
                i * 4 + 3,
                i * 4 + 0,
            ]);
        }

        (vertex_data.to_vec(), index_data.to_vec())
    }
}

impl Default for Level {
    fn default() -> Self {
        let tiles = vec![
            Tile {
                position: [0, 0],
                texture: [0, 0],
            },
            Tile {
                position: [1, 0],
                texture: [0, 1],
            },
            Tile {
                position: [0, 1],
                texture: [1, 0],
            },
        ];

        Self { tiles }
    }
}
