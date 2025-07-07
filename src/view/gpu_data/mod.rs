//! Structures that manage data on the GPU

mod deferred;
mod texture;
mod texture_depth;
mod texture_multiplexer;
mod uniform;
mod vertex;

pub use deferred::DeferredInput;
pub use deferred::DeferredInputViews;
pub use deferred::DeferredTextureGroup;

pub use texture::TextureGroup;
pub use texture_depth::TextureDepth;
pub use texture_multiplexer::TextureMultiplexer;

pub use uniform::UniformGroup;

pub use vertex::IndexFormat;
pub use vertex::VertexBuffers;
pub use vertex::VertexData;
