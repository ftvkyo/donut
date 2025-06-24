use anyhow::{Result, ensure};
use wgpu::util::DeviceExt;

use crate::view::gpu::GPU;

pub struct UniformGroup {
    layout: wgpu::BindGroupLayout,
    group: wgpu::BindGroup,
    buffers: Vec<wgpu::Buffer>,
}

impl UniformGroup {
    pub fn new(gpu: &GPU, uniforms: &[&[u8]]) -> Result<Self> {
        ensure!(uniforms.len() > 0, "at least one uniform is expected");

        let mut buffers = Vec::with_capacity(uniforms.len());
        let mut layout_entries = Vec::with_capacity(uniforms.len());

        for (index, data) in uniforms.iter().enumerate() {
            let buffer = gpu
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: data,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

            let layout_entry = wgpu::BindGroupLayoutEntry {
                binding: index as u32,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(data.len() as u64),
                },
                count: None,
            };

            buffers.push(buffer);
            layout_entries.push(layout_entry);
        }

        let layout = gpu
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &layout_entries,
            });

        let mut group_entries = Vec::with_capacity(uniforms.len());

        for (index, buffer) in buffers.iter().enumerate() {
            group_entries.push(wgpu::BindGroupEntry {
                binding: index as u32,
                resource: buffer.as_entire_binding(),
            });
        }

        let group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &group_entries,
        });

        Ok(Self {
            layout,
            group,
            buffers,
        })
    }

    pub fn update(&self, gpu: &GPU, uniforms: &[&[u8]]) -> Result<()> {
        ensure!(uniforms.len() == self.buffers.len());

        for (data, buffer) in uniforms.iter().zip(self.buffers.iter()) {
            ensure!(data.len() <= buffer.size() as usize);
            gpu.queue.write_buffer(buffer, 0, data);
        }

        gpu.queue.submit([]);

        Ok(())
    }

    pub fn get_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.layout
    }

    pub fn get_bind_group(&self) -> &wgpu::BindGroup {
        &self.group
    }
}
