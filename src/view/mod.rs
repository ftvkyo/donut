pub mod camera;
pub mod render_target;
pub mod vertex;

use std::{borrow::Cow, sync::Arc};

use rgb::Rgba;
use winit::window::Window;

use crate::{
    game::Game,
    view::{
        camera::{Camera, GPUCameraData},
        render_target::RenderTarget,
        vertex::GPUVertexData,
    },
};

pub use vertex::Vertex;

pub struct View {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,

    render_target: RenderTarget,

    texture_bind_group: wgpu::BindGroup,

    camera: Camera,
    camera_data: GPUCameraData,

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
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
        });

        let render_target = RenderTarget::new(&instance, &adapter, &device, window);

        let camera = Camera::new(
            render_target.get_aspect_ratio(),
            game.movement.get_position(),
        );
        let camera_data = GPUCameraData::new(&device, &camera);

        /* Set up the texture stuff */

        let texture_bytes_per_row = game.texture.width() * size_of::<Rgba<u8>>() as u32;

        let texture_extent = wgpu::Extent3d {
            width: game.texture.width(),
            height: game.texture.height(),
            depth_or_array_layers: 1,
        };

        let texture_view_format = wgpu::TextureFormat::Rgba8Unorm;

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: texture_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[texture_view_format],
        });

        queue.write_texture(
            texture.as_image_copy(),
            bytemuck::cast_slice(&game.texture.clone().into_raw()),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(texture_bytes_per_row),
                rows_per_image: None,
            },
            texture_extent,
        );

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(texture_view_format),
            ..Default::default()
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture BGL"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                }],
            });

        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &texture_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&texture_view),
            }],
        });

        let (vertex_data, index_data) = game.vertex_data();
        let vertex_data = GPUVertexData::new(&device, vertex_data, index_data);

        /* Set up the pipeline */

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&camera_data.bind_group_layout, &texture_bind_group_layout],
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
            render_target,
            pipeline,

            texture_bind_group,

            camera,
            camera_data,
            vertex_data,
        }
    }

    pub fn update_camera(&mut self, f: impl Fn(&mut Camera)) {
        f(&mut self.camera);
        self.camera_data.update(&self.queue, &self.camera);
    }

    pub fn resize(&mut self) {
        self.render_target.configure(&self.device);
        let aspect_ratio = self.render_target.get_aspect_ratio();
        self.update_camera(|camera| {
            camera.set_aspect_ratio(aspect_ratio);
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
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.5,
                            g: 0.5,
                            b: 0.5,
                            a: 1.0,
                        }),
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
            rpass.set_bind_group(1, &self.texture_bind_group, &[]);
            rpass.set_vertex_buffer(0, self.vertex_data.vertex_buffer.slice(..));
            rpass.set_index_buffer(
                self.vertex_data.index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
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
