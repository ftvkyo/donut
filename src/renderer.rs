use std::{borrow::Cow, mem::offset_of, sync::Arc};

use bytemuck::{Pod, Zeroable};
use rgb::Rgba;
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::game::Game;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub pos: [f32; 4],
    pub tex_coord: [f32; 2],
}

struct Surface {
    pub inner: wgpu::Surface<'static>,
    pub format: wgpu::TextureFormat,
    pub view_format: wgpu::TextureFormat,
}

struct Camera {
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    pub view_uniform: wgpu::Buffer,
    pub proj_uniform: wgpu::Buffer,
}

struct Vertices {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: usize,
}

pub struct Renderer {
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: Surface,
    pipeline: wgpu::RenderPipeline,

    texture_bind_group: wgpu::BindGroup,

    camera: Camera,
    vertices: Vertices,
}

impl Renderer {
    pub async fn new(
        window: Arc<Window>,
        game: &Game, /* TODO: only pass what's necessary */
    ) -> Renderer {
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

        /* Set up the camera stuff */

        let camera = {
            let uniform_type = wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(size_of::<glam::Mat4>() as u64),
            };

            let bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Camera BGL"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: uniform_type,
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: uniform_type,
                            count: None,
                        },
                    ],
                });

            let view_uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Camera View Uniform"),
                contents: bytemuck::cast_slice(glam::Mat4::ZERO.as_ref()),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

            let proj_uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Camera Projection Uniform"),
                contents: bytemuck::cast_slice(glam::Mat4::ZERO.as_ref()),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Camera BG"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: view_uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: proj_uniform.as_entire_binding(),
                    },
                ],
            });

            Camera {
                bind_group_layout,
                bind_group,
                view_uniform,
                proj_uniform,
            }
        };

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

        /* Set up the pipeline */

        let vertices = {
            let (vertex_data, index_data) = game.vertex_data();

            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertex_data),
                usage: wgpu::BufferUsages::VERTEX,
            });

            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&index_data),
                usage: wgpu::BufferUsages::INDEX,
            });

            Vertices {
                vertex_buffer,
                index_buffer,
                index_count: index_data.len(),
            }
        };

        let vertex_buffers = [wgpu::VertexBufferLayout {
            array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: offset_of!(Vertex, pos) as u64,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: offset_of!(Vertex, tex_coord) as u64,
                    shader_location: 1,
                },
            ],
        }];

        let surface = {
            let surface = instance.create_surface(window.clone()).unwrap();
            let capabilities = surface.get_capabilities(&adapter);

            let format = capabilities
                .formats
                .into_iter()
                .find(|fmt| fmt.has_color_aspect() && fmt.is_srgb())
                .expect("No suitable surface format found");

            let view_format = format.remove_srgb_suffix();

            Surface {
                inner: surface,
                format,
                view_format,
            }
        };

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&camera.bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &vertex_buffers,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(surface.view_format.into())],
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

        let mut state = Renderer {
            window,
            device,
            queue,
            surface,
            pipeline,

            texture_bind_group,

            camera,
            vertices,
        };

        state.configure_surface();
        state.update_camera(glam::vec2(4.0, 4.0));

        state
    }

    pub fn get_window(&self) -> &Window {
        &self.window
    }

    pub fn configure_surface(&self) {
        let size = self.window.inner_size();

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface.format,
            view_formats: vec![self.surface.view_format],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: size.width,
            height: size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };

        self.surface.inner.configure(&self.device, &surface_config);
    }

    pub fn update_camera(&mut self, position: glam::Vec2) {
        use glam::{Mat4, Vec3};

        let size = self.window.inner_size();

        let aspect_ratio = size.width as f32 / size.height as f32;

        let camera_fov = std::f32::consts::FRAC_PI_2;
        let camera_dist = 4.0; // with PI/2, this means we see 4 tiles up and 4 tiles down
        let camera_pos = position.extend(camera_dist);

        let view = Mat4::look_to_rh(camera_pos, Vec3::NEG_Z, Vec3::Y);
        let proj = Mat4::perspective_rh(camera_fov, aspect_ratio, 1.0, 10.0);

        self.queue
            .write_buffer(&self.camera.view_uniform, 0, bytemuck::cast_slice(&[view]));

        self.queue
            .write_buffer(&self.camera.proj_uniform, 0, bytemuck::cast_slice(&[proj]));
    }

    pub fn render(&mut self, _game: &Game) {
        let surface_texture = self
            .surface
            .inner
            .get_current_texture()
            .expect("failed to acquire next swapchain texture");

        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                format: Some(self.surface.view_format),
                ..Default::default()
            });

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
            rpass.set_bind_group(0, &self.camera.bind_group, &[]);
            rpass.set_bind_group(1, &self.texture_bind_group, &[]);
            rpass.set_vertex_buffer(0, self.vertices.vertex_buffer.slice(..));
            rpass.set_index_buffer(
                self.vertices.index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );
            rpass.pop_debug_group();

            rpass.insert_debug_marker("Draw!");
            rpass.draw_indexed(0..self.vertices.index_count as u32, 0, 0..1);
        }

        self.queue.submit([encoder.finish()]);

        self.window.pre_present_notify();
        surface_texture.present();
    }
}
