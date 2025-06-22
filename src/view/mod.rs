pub mod camera;
pub mod light;
pub mod surface;
pub mod texture;
pub mod vertex;

use std::{borrow::Cow, sync::Arc};

use anyhow::{Context, Result};
use winit::window::Window;

use crate::{
    assets::{Assets, TextureData},
    game::{Game, camera::Camera, light::Lights},
    view::{
        camera::GPUCameraData, light::GPULightsData, surface::Surface, texture::GPUTextureData,
        vertex::GPUVertexData,
    },
};

pub use vertex::Vertex;

pub struct View {
    pub renderer: Renderer,
    pub gpu_data: GpuData,
    pub pipeline: wgpu::RenderPipeline,
}

impl View {
    pub fn new(window: Arc<Window>, assets: &Assets, game: &Game) -> Result<Self> {
        let renderer = pollster::block_on(Renderer::new(window));

        let stage = assets
            .stages
            .get(&game.stage_name)
            .with_context(|| format!("No stage called '{}'?", game.stage_name))?;

        let (tile_set, sprites) = stage.layers[0].sprites(&assets.tile_sets, assets.tile_size)?;

        let mut vertex_data = Vec::with_capacity(sprites.len() * 4);
        let mut index_data = Vec::with_capacity(sprites.len() * 6);

        for (i, sprite) in sprites.iter().enumerate() {
            vertex_data.extend_from_slice(&sprite.vertex_data());
            index_data.extend_from_slice(&sprite.index_data(i as u16 * 4));
        }

        let gpu_data = renderer.create_data(
            &tile_set.texture_color,
            &tile_set.texture_normal,
            &game.camera,
            &game.lights,
            vertex_data,
            index_data,
        );

        let shader = assets
            .shaders
            .get("main")
            .context("No shader called 'main'?")?;

        let pipeline = renderer.create_pipeline(shader, &gpu_data);

        Ok(Self {
            renderer,
            gpu_data,
            pipeline,
        })
    }
}

pub struct GpuData {
    pub texture: GPUTextureData,
    pub camera: GPUCameraData,
    pub lights: GPULightsData,
    pub vertex: GPUVertexData,
}

pub struct Renderer {
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
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

    pub fn create_data(
        &self,
        texture_color: &TextureData,
        texture_normal: &TextureData,
        camera: &Camera,
        lights: &Lights,
        vertex_data: Vec<Vertex>,
        index_data: Vec<u16>,
    ) -> GpuData {
        let gpu_texture =
            GPUTextureData::new(&self.device, &self.queue, texture_color, texture_normal);
        let gpu_camera = GPUCameraData::new(
            &self.device,
            camera.matrix_view(),
            camera.matrix_proj(self.surface.aspect_ratio()),
        );
        let gpu_lights = GPULightsData::new(&self.device, &lights.data(camera.matrix_view()));
        let gpu_vertex = GPUVertexData::new(&self.device, vertex_data, index_data);

        GpuData {
            texture: gpu_texture,
            camera: gpu_camera,
            lights: gpu_lights,
            vertex: gpu_vertex,
        }
    }

    pub fn create_pipeline(&self, shader: &String, data: &GpuData) -> wgpu::RenderPipeline {
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
                    &data.camera.bind_group_layout,
                    &data.lights.bind_group_layout,
                    &data.texture.bind_group_layout,
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

    pub fn render(&mut self, pipeline: &wgpu::RenderPipeline, data: &GpuData) {
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

            rpass.push_debug_group("Prepare data for draw.");
            rpass.set_pipeline(pipeline);
            rpass.set_bind_group(0, &data.camera.bind_group, &[]);
            rpass.set_bind_group(1, &data.lights.bind_group, &[]);
            rpass.set_bind_group(2, &data.texture.bind_group, &[]);
            rpass.set_vertex_buffer(0, data.vertex.vertex_buffer.slice(..));
            rpass.set_index_buffer(
                data.vertex.index_buffer.slice(..),
                GPUVertexData::INDEX_FORMAT,
            );
            rpass.pop_debug_group();

            rpass.insert_debug_marker("Draw!");
            rpass.draw_indexed(0..data.vertex.index_count as u32, 0, 0..1);
        }

        self.queue.submit([encoder.finish()]);

        self.window.pre_present_notify();
        surface_texture.present();
    }
}
