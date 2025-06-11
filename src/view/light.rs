use glam::{Vec2, Vec4};
use wgpu::util::DeviceExt;

pub struct Light {
    pub position: Vec2,
}

impl Light {
    pub fn new(position: Vec2) -> Self {
        Self { position }
    }

    pub fn position(&self) -> Vec4 {
        self.position.extend(0.2).extend(1.0)
    }
}

pub struct GPULightData {
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    pub light_uniform: wgpu::Buffer,
}

impl GPULightData {
    pub fn new(device: &wgpu::Device, light: &Light) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Light bind group layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(size_of::<Vec4>() as u64),
                },
                count: None,
            }],
        });

        let light_uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light uniform"),
            contents: bytemuck::cast_slice(&[light.position()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
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

    pub fn update(&self, queue: &wgpu::Queue, light: &Light) {
        queue.write_buffer(
            &self.light_uniform,
            0,
            bytemuck::cast_slice(&[light.position()]),
        );
    }
}
