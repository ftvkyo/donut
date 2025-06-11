use glam::{Mat4, Vec2, Vec3};
use wgpu::util::DeviceExt;

pub struct Camera {
    pub aspect_ratio: f32,
    pub position: Vec2,
}

impl Camera {
    const FOV: f32 = std::f32::consts::FRAC_PI_2;
    const DISTANCE: f32 = 4.0; // with PI/2, this means we see 4 tiles up and 4 tiles down
    const NEAR: f32 = 1.0;
    const FAR: f32 = 10.0;

    pub fn new(aspect_ratio: f32, position: Vec2) -> Self {
        Self {
            aspect_ratio,
            position,
        }
    }

    pub fn matrix_view(&self) -> Mat4 {
        let position = self.position.extend(Self::DISTANCE);
        Mat4::look_to_rh(position, Vec3::NEG_Z, Vec3::Y)
    }

    pub fn matrix_proj(&self) -> Mat4 {
        Mat4::perspective_rh(Self::FOV, self.aspect_ratio, Self::NEAR, Self::FAR)
    }
}

pub struct GPUCameraData {
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    pub view_uniform: wgpu::Buffer,
    pub proj_uniform: wgpu::Buffer,
}

impl GPUCameraData {
    pub fn new(device: &wgpu::Device, camera: &Camera) -> Self {
        let view = camera.matrix_view();
        let proj = camera.matrix_proj();

        let uniform_type = wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: wgpu::BufferSize::new(size_of::<Mat4>() as u64),
        };

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Camera bind group layout"),
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
            label: Some("Camera View uniform"),
            contents: bytemuck::cast_slice(&[view]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let proj_uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Projection uniform"),
            contents: bytemuck::cast_slice(&[proj]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera bind group"),
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

        Self {
            bind_group_layout,
            bind_group,
            view_uniform,
            proj_uniform,
        }
    }

    pub fn update(&self, queue: &wgpu::Queue, camera: &Camera) {
        let view = camera.matrix_view();
        let proj = camera.matrix_proj();

        queue.write_buffer(&self.view_uniform, 0, bytemuck::cast_slice(&[view]));
        queue.write_buffer(&self.proj_uniform, 0, bytemuck::cast_slice(&[proj]));
    }
}
