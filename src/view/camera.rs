use glam::Mat4;
use wgpu::util::DeviceExt;

use crate::{game::camera::Camera, view::renderer::Renderer};

pub struct GPUCameraData {
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    pub view_uniform: wgpu::Buffer,
    pub proj_uniform: wgpu::Buffer,
}

impl GPUCameraData {
    pub fn new(renderer: &Renderer, camera: &Camera) -> Self {
        let view = camera.matrix_view();
        let proj = camera.matrix_proj(renderer.aspect_ratio());
        
        let uniform_type = wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: wgpu::BufferSize::new(size_of::<Mat4>() as u64),
        };

        let bind_group_layout = renderer.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Camera bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
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

        let view_uniform = renderer.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera View uniform"),
            contents: bytemuck::cast_slice(&[view]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let proj_uniform = renderer.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Projection uniform"),
            contents: bytemuck::cast_slice(&[proj]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = renderer.device.create_bind_group(&wgpu::BindGroupDescriptor {
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

    pub fn update(&self, renderer: &Renderer, camera: &Camera) {
        let view = camera.matrix_view();
        let proj = camera.matrix_proj(renderer.aspect_ratio());
        renderer.queue.write_buffer(&self.view_uniform, 0, bytemuck::cast_slice(&[view]));
        renderer.queue.write_buffer(&self.proj_uniform, 0, bytemuck::cast_slice(&[proj]));
    }
}
