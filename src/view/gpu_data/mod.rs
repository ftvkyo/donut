pub mod texture;
pub mod uniform;
pub mod vertex;

pub use texture::TextureGroup;
pub use texture::TextureMultiplexer;

pub use uniform::UniformGroup;

pub use vertex::Vertex;
pub use vertex::VertexData;
pub use vertex::VertexIndex;

pub struct PipelineConfig<'s, 'g> {
    pub shader: &'s String,
    pub groups: &'g [&'g wgpu::BindGroupLayout],
    pub output: wgpu::TextureFormat,
}

pub struct PipelineExecution<'p, 'g, 'v> {
    pub pipeline: &'p wgpu::RenderPipeline,
    pub gdata: &'g [&'g wgpu::BindGroup],
    pub vdata: &'v VertexData,
}

pub struct RenderPass<'p> {
    pub pipelines: &'p [PipelineExecution<'p, 'p, 'p>],
}
