pub mod camera;
pub mod light;
pub mod render_target;
pub mod texture;
pub mod vertex;

use std::{borrow::Cow, sync::Arc};

use bytemuck::Zeroable;
use glam::vec2;
use winit::window::Window;

use crate::{
    game::Game,
    view::{
        camera::{Camera, GPUCameraData},
        light::{GPULightsData, LIGHT_COUNT, Light, Lights},
        render_target::RenderTarget,
        texture::GPUTextureData,
        vertex::GPUVertexData,
    },
};

pub use vertex::Vertex;

pub struct View {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,

    render_target: RenderTarget,

    camera: Camera,
    camera_data: GPUCameraData,
    lights: Lights,
    lights_data: GPULightsData,
    texture_data: GPUTextureData,
    vertex_data: GPUVertexData,
}

impl View {
    pub async fn new(
        window: Arc<Window>,
        game: &Game, /* TODO: only pass what's necessary */
    ) -> View {
        let instance = wgpu::Instance::new(&Default::default());
        let adapter = instance
            .request_adapter(&Default::default())
            .await
            .expect("Failed to acquire an adapter");
        let (device, queue) = adapter
            .request_device(&Default::default())
            .await
            .expect("Failed to acquire a device");

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&game.shader)),
        });

        let render_target = RenderTarget::new(&instance, &adapter, &device, window);

        /* TODO: extract the logic that initialises camera and light */

        let camera = Camera::new(render_target.get_aspect_ratio(), vec2(4.0, 4.0));
        let camera_data = GPUCameraData::new(&device, &camera);

        let lights = [Light::zeroed(); LIGHT_COUNT];
        let lights_data = GPULightsData::new(&device, &lights);

        let texture_data =
            GPUTextureData::new(&device, &queue, &game.texture_color, &game.texture_normal);

        let (vertex_data, index_data) = game.vertex_data();
        let vertex_data = GPUVertexData::new(&device, vertex_data, index_data);

        /* Set up the pipeline */

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[
                &camera_data.bind_group_layout,
                &lights_data.bind_group_layout,
                &texture_data.bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                targets: &[Some(render_target.surface_view_format.into())],
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

        Self {
            device,
            queue,
            pipeline,

            render_target,

            camera,
            camera_data,
            lights,
            lights_data,
            texture_data,
            vertex_data,
        }
    }

    pub fn update_camera(&mut self, f: impl Fn(&mut Camera)) {
        f(&mut self.camera);
        self.camera_data.update(&self.queue, &self.camera);
    }

    pub fn update_lights(&mut self, f: impl Fn(&Camera, &mut Lights)) {
        f(&self.camera, &mut self.lights);
        self.lights_data.update(&self.queue, &self.lights);
    }

    pub fn resize(&mut self) {
        self.render_target.configure(&self.device);
        let aspect_ratio = self.render_target.get_aspect_ratio();
        self.update_camera(|camera| {
            camera.aspect_ratio = aspect_ratio;
        });
    }

    pub fn render(&mut self, _game: &Game) {
        let (surface_texture, surface_view) = self.render_target.get_texture();

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
            rpass.set_pipeline(&self.pipeline);
            rpass.set_bind_group(0, &self.camera_data.bind_group, &[]);
            rpass.set_bind_group(1, &self.lights_data.bind_group, &[]);
            rpass.set_bind_group(2, &self.texture_data.bind_group, &[]);
            rpass.set_vertex_buffer(0, self.vertex_data.vertex_buffer.slice(..));
            rpass.set_index_buffer(
                self.vertex_data.index_buffer.slice(..),
                GPUVertexData::INDEX_FORMAT,
            );
            rpass.pop_debug_group();

            rpass.insert_debug_marker("Draw!");
            rpass.draw_indexed(0..self.vertex_data.index_count as u32, 0, 0..1);
        }

        self.queue.submit([encoder.finish()]);

        self.render_target.pre_present_notify();
        surface_texture.present();

        // Schedule rendering of the next frame
        self.render_target.request_redraw();
    }
}
