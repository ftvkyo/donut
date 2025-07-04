use anyhow::{Context, Result, bail};

use wgpu::Features as Fs;

use crate::view::{
    gpu_data::{TextureDepth, VertexData},
    gpu_struct::vertex::Vertex,
    window::Window,
};

pub struct PipelineShaderConfig<'s, 'v, 'f> {
    pub shader: &'s wgpu::ShaderModule,
    pub entrypoint_vertex: Option<&'v str>,
    pub entrypoint_fragment: Option<&'f str>,
}

pub struct PipelineConfig<'s, 'g> {
    pub shader: &'s PipelineShaderConfig<'s, 's, 's>,
    pub groups: &'g [&'g wgpu::BindGroupLayout],
    pub output: wgpu::TextureFormat,
}

pub struct PipelineExecution<'p, 'g, 'v> {
    pub pipeline: &'p wgpu::RenderPipeline,
    pub gdata: &'g [&'g wgpu::BindGroup],
    pub vdata: &'v VertexData,
}

pub struct RenderPass<'d, 'p> {
    pub depth: &'d wgpu::TextureView,
    pub pipelines: &'p [PipelineExecution<'p, 'p, 'p>],
}

pub struct GPU {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl GPU {
    /// Features to request from the adapter
    // https://docs.rs/wgpu/latest/wgpu/struct.Features.html
    const FEATURES: &[wgpu::Features] = &[
        Fs::TEXTURE_BINDING_ARRAY,
        Fs::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
    ];

    pub async fn new() -> Result<Self> {
        let instance = wgpu::Instance::new(&Default::default());
        let adapter = instance
            .request_adapter(&Default::default())
            .await
            .context("Failed to acquire an adapter")?;

        let required_features = Self::FEATURES.iter().copied().fold(Fs::empty(), Fs::union);
        let adapter_features = adapter.features();
        let missing_features = required_features.difference(adapter_features);

        if !missing_features.is_empty() {
            bail!("The adapter is missing the following features: {missing_features}");
        }

        let mut required_limits = wgpu::Limits::default();
        required_limits.max_binding_array_elements_per_shader_stage = 4;

        let (device, queue) = adapter
            .request_device(&wgpu::wgt::DeviceDescriptor {
                required_features,
                required_limits,
                ..Default::default()
            })
            .await
            .context("Failed to acquire a device")?;

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
        })
    }

    pub fn create_shader(&self, source: &String) -> wgpu::ShaderModule {
        self.device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(source)),
            })
    }

    pub fn create_pipeline(&self, config: &PipelineConfig) -> wgpu::RenderPipeline {
        let layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &config.groups,
                push_constant_ranges: &[],
            });

        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &config.shader.shader,
                    entry_point: config.shader.entrypoint_vertex,
                    compilation_options: Default::default(),
                    buffers: &[Vertex::LAYOUT],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &config.shader.shader,
                    entry_point: config.shader.entrypoint_fragment,
                    compilation_options: Default::default(),
                    targets: &[Some(config.output.into())],
                }),
                primitive: wgpu::PrimitiveState {
                    cull_mode: Some(wgpu::Face::Back),
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: TextureDepth::FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                multiview: None,
                cache: None,
            });

        pipeline
    }

    pub fn render(&self, window: &Window, rpc: &RenderPass) -> Result<()> {
        let (surface_texture, surface_view) = window.texture()?;

        let mut encoder = self.device.create_command_encoder(&Default::default());

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: rpc.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            for pconfig in rpc.pipelines {
                rpass.set_pipeline(&pconfig.pipeline);

                for (index, group) in pconfig.gdata.iter().enumerate() {
                    rpass.set_bind_group(index as u32, *group, &[]);
                }

                rpass.set_vertex_buffer(0, pconfig.vdata.get_vertex_buffer().slice(..));
                rpass.set_index_buffer(
                    pconfig.vdata.get_index_buffer().slice(..),
                    Vertex::INDEX_FORMAT,
                );

                rpass.draw_indexed(0..pconfig.vdata.get_index_count(), 0, 0..1);
            }
        }

        self.queue.submit([encoder.finish()]);

        window.pre_present_notify();
        surface_texture.present();

        Ok(())
    }
}
