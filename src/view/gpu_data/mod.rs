//! Structures that manage data on the GPU

mod depth;
mod texture;
mod uniform;
mod vertex;

pub use depth::TextureDepth;

pub use texture::TextureGroup;
pub use texture::TextureMultiplexer;

pub use uniform::UniformGroup;

pub use vertex::VertexData;
