use std::array::from_fn;

use crate::renderer::Vertex;

pub const LEVEL_SIZE: usize = 16;

pub struct Tile {
    texture: [u16; 2],
}

pub struct Level {
    tiles: [[Option<Tile>; LEVEL_SIZE]; LEVEL_SIZE],
}

impl Level {
    pub fn vertex_data(&self, tile_size: u16) -> (Vec<Vertex>, Vec<u16>) {
        let v = |pos: [i16; 3], tc: [u16; 2]| -> Vertex {
            Vertex {
                pos: [pos[0] as f32, pos[1] as f32, pos[2] as f32, 1.0],
                tex_coord: [(tc[0] * tile_size) as f32, (tc[1] * tile_size) as f32],
            }
        };

        let mut i = 0;

        let mut vertex_data = Vec::with_capacity(self.tiles.len() * 4);
        let mut index_data = Vec::with_capacity(self.tiles.len() * 6);

        for (x, tile_column) in self.tiles.iter().enumerate() {
            for (y, tile) in tile_column.iter().enumerate() {
                if let Some(tile) = tile {
                    let x = x as i16;
                    let y = y as i16;

                    vertex_data.extend_from_slice(&[
                        v(
                            [x, y, 0],
                            [tile.texture[0], tile.texture[1] + 1],
                        ),
                        v(
                            [x + 1, y, 0],
                            [tile.texture[0] + 1, tile.texture[1] + 1],
                        ),
                        v(
                            [x + 1, y + 1, 0],
                            [tile.texture[0] + 1, tile.texture[1]],
                        ),
                        v(
                            [x, y + 1, 0],
                            [tile.texture[0], tile.texture[1]],
                        ),
                    ]);

                    index_data.extend_from_slice(&[
                        i + 0,
                        i + 1,
                        i + 2,
                        i + 2,
                        i + 3,
                        i + 0,
                    ]);

                    i += 4;
                }
            }
        }

        (vertex_data.to_vec(), index_data.to_vec())
    }
}

impl Default for Level {
    fn default() -> Self {
        let mut tiles = from_fn(|_| from_fn(|_| {
            None
        }));

        tiles[0][0] = Some(Tile {
            texture: [0, 0],
        });

        tiles[2][0] = Some(Tile {
            texture: [0, 3],
        });

        tiles[2][1] = Some(Tile {
            texture: [0, 4],
        });

        tiles[3][1] = Some(Tile {
            texture: [3, 0],
        });

        Self { tiles }
    }
}

#[derive(Default)]
pub struct Game {
    pub level: Level,
}
