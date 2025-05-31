use std::{borrow::Cow, mem::offset_of, sync::Arc};

use bytemuck::{Pod, Zeroable};
use log::info;
use rgb::Rgba;
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    _pos: [f32; 4],
    _tex_coord: [f32; 2],
}

fn create_vertices() -> (Vec<Vertex>, Vec<u16>) {
    fn v(pos: [i8; 3], tc: [i8; 2]) -> Vertex {
        Vertex {
            _pos: [pos[0] as f32, pos[1] as f32, pos[2] as f32, 1.0],
            _tex_coord: [tc[0] as f32, tc[1] as f32],
        }
    }

    let vertex_data = [
        v([-1, -1, 0], [0, 0]),
        v([1, -1, 0], [1, 0]),
        v([1, 1, 0], [1, 1]),
        v([-1, 1, 0], [0, 1]),
    ];

    let index_data: &[u16] = &[0, 1, 2, 2, 3, 0];

    (vertex_data.to_vec(), index_data.to_vec())
}

fn create_texels(size: usize) -> Vec<Rgba<u8>> {
    let line_factor = 1;
    let arrow_size = 10;

    let c = size as isize / 2;

    (0..size * size)
        .map(|i| {
            let x = (i % size) as isize - c;
            let y = (i / size) as isize - c;

            let mut r = 0;
            let mut g = 0;
            let mut b = 0;

            if x.abs() <= line_factor {
                r = 255;
            }

            if y.abs() <= line_factor {
                g = 255;
            }

            if ((y + x - c).abs() <= line_factor
                || (y - x + c).abs() <= line_factor
                || (y - x - c).abs() <= line_factor)
                && (x.abs() < arrow_size || y.abs() < arrow_size)
                && (y > -x)
            {
                b = 255;
            }

            return Rgba { r, g, b, a: 255 };
        })
        .collect()
}

fn generate_matrix(aspect_ratio: f32) -> glam::Mat4 {
    let projection =
        glam::Mat4::perspective_rh(std::f32::consts::FRAC_PI_4, aspect_ratio, 1.0, 10.0);
    let view = glam::Mat4::look_at_rh(
        glam::Vec3::new(0.0, 0.0, 3.0),
        glam::Vec3::ZERO,
        glam::Vec3::Y,
    );
    projection * view
}

struct State {
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,

    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: usize,
}

impl State {
    async fn new(window: Arc<Window>) -> State {
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

        let size = window.inner_size();

        let surface = instance.create_surface(window.clone()).unwrap();
        let capabilities = surface.get_capabilities(&adapter);

        let surface_format = *capabilities
            .formats
            .get(0)
            .expect("No supported surface formats");

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(size_of::<glam::Mat4>() as u64),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        sample_type: wgpu::TextureSampleType::Uint,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let matrix = generate_matrix(size.width as f32 / size.height as f32);
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(matrix.as_ref()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let texture_size = 256u32;
        let texture_extent = wgpu::Extent3d {
            width: texture_size,
            height: texture_size,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: texture_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let texels = create_texels(texture_size as usize);
        queue.write_texture(
            texture.as_image_copy(),
            bytemuck::cast_slice(&texels),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(texture_size * size_of_val(&texels[0]) as u32),
                rows_per_image: None,
            },
            texture_extent,
        );

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
            ],
        });

        let vertex_size = size_of::<Vertex>();
        let (vertex_data, index_data) = create_vertices();

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

        let vertex_buffers = [wgpu::VertexBufferLayout {
            array_stride: vertex_size as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: offset_of!(Vertex, _tex_coord) as u64,
                    shader_location: 1,
                },
            ],
        }];

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
                targets: &[Some(surface_format.into())],
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

        let state = State {
            window,
            device,
            queue,
            size,
            surface,
            surface_format,
            pipeline,

            bind_group,
            uniform_buffer,
            vertex_buffer,
            index_buffer,
            index_count: index_data.len(),
        };

        state.configure_surface();

        state
    }

    fn get_window(&self) -> &Window {
        &self.window
    }

    fn configure_surface(&self) {
        // Request compatibility with the sRGB-format texture view we're going to create later.
        let view_formats = vec![self.surface_format.add_srgb_suffix()];

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            view_formats,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.size.width,
            height: self.size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };

        self.surface.configure(&self.device, &surface_config);
    }

    fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        let matrix = generate_matrix(size.width as f32 / size.height as f32);

        self.size = size;
        self.configure_surface();

        self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(matrix.as_ref()),
        );
    }

    fn render(&mut self) {
        let surface_texture = self
            .surface
            .get_current_texture()
            .expect("failed to acquire next swapchain texture");

        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                // Without add_srgb_suffix() the image we will be working with might not be "gamma correct".
                format: Some(self.surface_format.add_srgb_suffix()),
                ..Default::default()
            });

        let mut encoder = self.device.create_command_encoder(&Default::default());

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
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
            rpass.set_bind_group(0, &self.bind_group, &[]);
            rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            rpass.pop_debug_group();

            rpass.insert_debug_marker("Draw!");
            rpass.draw_indexed(0..self.index_count as u32, 0, 0..1);
        }

        self.queue.submit([encoder.finish()]);

        self.window.pre_present_notify();
        surface_texture.present();
    }
}

#[derive(Default)]
pub struct App {
    state: Option<State>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_size = winit::dpi::LogicalSize::new(1366, 768);

        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_inner_size(window_size)
                        .with_resizable(false),
                )
                .unwrap(),
        );

        let state = pollster::block_on(State::new(window.clone()));
        self.state = Some(state);

        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let state = self.state.as_mut().unwrap();
        match event {
            WindowEvent::CloseRequested => {
                info!("Received close request, stopping...");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                state.render();
                state.get_window().request_redraw();
            }
            WindowEvent::Resized(size) => {
                state.resize(size);
                // No need to re-render as the next event will be RedrawRequested
            }
            _ => (),
        }
    }
}
