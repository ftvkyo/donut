use bytemuck::{Pod, Zeroable};
use glam::Vec4;
use wgpu::util::DeviceExt;

pub const LIGHT_COUNT: usize = 32;

#[derive(Pod, Zeroable, Clone, Copy)]
#[repr(C)]
pub struct Light {
    pub position: Vec4,
    pub color: Vec4,
}

pub type Lights = [Light; LIGHT_COUNT];

/// To make calculations easier, the light info is uploaded in view coordinates
pub struct GPULightsData {
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    pub light_uniform: wgpu::Buffer,
}

impl GPULightsData {
    pub fn new(device: &wgpu::Device, lights: &[Light; LIGHT_COUNT]) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Light bind group layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(
                        (size_of::<Light>() * LIGHT_COUNT) as u64,
                    ),
                },
                count: None,
            }],
        });

        let light_uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light uniform"),
            contents: bytemuck::cast_slice(lights),
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

    pub fn update(&self, queue: &wgpu::Queue, lights: &[Light; LIGHT_COUNT]) {
        queue.write_buffer(&self.light_uniform, 0, bytemuck::cast_slice(lights));
    }
}
