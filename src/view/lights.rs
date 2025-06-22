use wgpu::util::DeviceExt;

use crate::{game::{camera::Camera, light::Lights}, view::renderer::Renderer};

/// To make calculations easier, the light info is uploaded in view coordinates
pub struct GPULightsData {
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    pub light_uniform: wgpu::Buffer,
}

impl GPULightsData {
    pub fn new(renderer: &Renderer, lights: &Lights, camera: &Camera) -> Self {
        let lights = lights.data(camera.matrix_view());
        
        let bind_group_layout = renderer.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Light bind group layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(lights.len() as u64),
                },
                count: None,
            }],
        });

        let light_uniform = renderer.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light uniform"),
            contents: &lights,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = renderer.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Light bind group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: light_uniform.as_entire_binding(),
            }],
        });

        Self {
            bind_group_layout,
            bind_group,
            light_uniform,
        }
    }

    pub fn update(&self, renderer: &Renderer, lights: &Lights, camera: &Camera) {
        let lights = lights.data(camera.matrix_view());
        renderer.queue.write_buffer(&self.light_uniform, 0, &lights);
    }
}
