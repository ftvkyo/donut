use anyhow::{Context, Result, bail};

use wgpu::Features as Fs;

use crate::view::gpu_data::VertexData;

pub struct PipelineConfig<'s, 'v, 'g, 'o> {
    pub label: &'static str,
    pub shader: &'s wgpu::ShaderModule,
    pub groups: &'g [&'g wgpu::BindGroupLayout],
    pub targets: &'o [wgpu::ColorTargetState],
    pub depth_stencil: Option<wgpu::DepthStencilState>,
    pub vertex_layout: wgpu::VertexBufferLayout<'v>,
}

pub struct RenderPass<'desc, 'pip, 'gdat, 'vdat> {
    pub descriptor: &'desc wgpu::RenderPassDescriptor<'desc>,
    pub pipeline: &'pip wgpu::RenderPipeline,
    pub gdata: &'gdat [&'gdat wgpu::BindGroup],
    pub vdata: &'vdat dyn VertexData,
}

pub struct RenderConfig<'p> {
    pub passes: &'p [&'p RenderPass<'p, 'p, 'p, 'p>],
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

    pub fn create_shader(&self, name: &str, source: &String) -> wgpu::ShaderModule {
        self.device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(name),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(source)),
            })
    }

    pub fn create_pipeline(&self, config: PipelineConfig) -> wgpu::RenderPipeline {
        let layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &config.groups,
                push_constant_ranges: &[],
            });

        let targets: Vec<_> = config
            .targets
            .into_iter()
            .map(|target| Some(target.clone()))
            .collect();

        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(config.label),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &config.shader,
                    entry_point: None,
                    compilation_options: Default::default(),
                    buffers: &[config.vertex_layout],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &config.shader,
                    entry_point: None,
                    compilation_options: Default::default(),
                    targets: &targets,
                }),
                depth_stencil: config.depth_stencil,
                primitive: Default::default(),
                multisample: Default::default(),
                multiview: None,
                cache: None,
            });

        pipeline
    }

    pub fn render(&self, config: &RenderConfig) -> Result<()> {
        let mut encoder = self.device.create_command_encoder(&Default::default());

        for &pass in config.passes {
            let mut rpass = encoder.begin_render_pass(pass.descriptor);

            rpass.set_pipeline(&pass.pipeline);
            for (index, group) in pass.gdata.iter().enumerate() {
                rpass.set_bind_group(index as u32, *group, &[]);
            }

            rpass.set_vertex_buffer(0, pass.vdata.get_vertex_buffer().slice(..));
            rpass.set_index_buffer(
                pass.vdata.get_index_buffer().slice(..),
                pass.vdata.get_index_format(),
            );

            rpass.draw_indexed(0..pass.vdata.get_index_count(), 0, 0..1);
        }

        self.queue.submit([encoder.finish()]);

        Ok(())
    }
}
