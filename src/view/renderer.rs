use std::{borrow::Cow, sync::Arc};

use winit::window::Window;

use crate::view::{
    camera::GPUCameraData, lights::GPULightsData, surface::Surface, texture::GPUTextureData,
    vertex::GPUVertexData,
};

pub struct GPUPipelineData {
    pub texture_data: GPUTextureData,
    pub vertex_data: GPUVertexData,
    pub pipeline: wgpu::RenderPipeline,
}

pub struct Renderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    window: Arc<Window>,
    surface: Surface,
}

impl Renderer {
    pub async fn new(window: Arc<Window>) -> Self {
        let instance = wgpu::Instance::new(&Default::default());
        let adapter = instance
            .request_adapter(&Default::default())
            .await
            .expect("Failed to acquire an adapter");
        let (device, queue) = adapter
            .request_device(&Default::default())
            .await
            .expect("Failed to acquire a device");

        let surface = Surface::new(&instance, &adapter, &device, &window);

        Self {
            window,
            device,
            queue,
            surface,
        }
    }

    pub fn create_pipeline(
        &self,
        shader: &String,
        camera: &GPUCameraData,
        lights: &GPULightsData,
        texture: &GPUTextureData,
    ) -> wgpu::RenderPipeline {
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(shader)),
            });

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[
                    &camera.bind_group_layout,
                    &lights.bind_group_layout,
                    &texture.bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[GPUVertexData::LAYOUT],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(self.surface.surface_view_format.into())],
                }),
                primitive: wgpu::PrimitiveState {
                    cull_mode: Some(wgpu::Face::Back),
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: Default::default(),
                multiview: None,
                cache: None,
            });

        pipeline
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    pub fn resize(&mut self) {
        self.surface.resize(&self.device, self.window.inner_size());
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.surface.aspect_ratio()
    }

    pub fn render(
        &mut self,
        camera: &GPUCameraData,
        lights: &GPULightsData,
        pipelines: &Vec<GPUPipelineData>,
    ) {
        let (surface_texture, surface_view) = self.surface.texture();

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
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            for p in pipelines {
                rpass.push_debug_group("Prepare data for draw.");
                rpass.set_pipeline(&p.pipeline);
                rpass.set_bind_group(0, &camera.bind_group, &[]);
                rpass.set_bind_group(1, &lights.bind_group, &[]);
                rpass.set_bind_group(2, &p.texture_data.bind_group, &[]);
                rpass.set_vertex_buffer(0, p.vertex_data.vertex_buffer.slice(..));
                rpass.set_index_buffer(
                    p.vertex_data.index_buffer.slice(..),
                    GPUVertexData::INDEX_FORMAT,
                );
                rpass.pop_debug_group();

                rpass.insert_debug_marker("Draw!");
                rpass.draw_indexed(0..p.vertex_data.index_count as u32, 0, 0..1);
            }
        }

        self.queue.submit([encoder.finish()]);

        self.window.pre_present_notify();
        surface_texture.present();
    }
}
