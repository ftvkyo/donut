//! Structures that manage data on the GPU

mod texture;
mod uniform;
mod vertex;

pub use texture::TextureGroup;
pub use texture::TextureMultiplexer;

pub use uniform::UniformGroup;

pub use vertex::VertexData;
