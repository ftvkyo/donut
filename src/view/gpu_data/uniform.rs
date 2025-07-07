use anyhow::{Result, ensure};
use wgpu::util::DeviceExt;

use crate::view::gpu::GPU;

pub struct UniformGroup<const BINDINGS: usize> {
    layout: wgpu::BindGroupLayout,
    group: wgpu::BindGroup,
    buffers: [wgpu::Buffer; BINDINGS],
}

impl<const BINDINGS: usize> UniformGroup<BINDINGS> {
    pub fn new(gpu: &GPU, uniforms: &[&[u8]; BINDINGS]) -> Result<Self> {
        ensure!(uniforms.len() > 0, "at least one uniform is expected");

        let buffers = std::array::from_fn(|index| {
            gpu.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: uniforms[index],
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                })
        });

        let layout_entries =
            std::array::from_fn::<_, BINDINGS, _>(|index| wgpu::BindGroupLayoutEntry {
                binding: index as u32,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(uniforms[index].len() as u64),
                },
                count: None,
            });

        let layout = gpu
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &layout_entries,
            });

        let group_entries = std::array::from_fn::<_, BINDINGS, _>(|index| wgpu::BindGroupEntry {
            binding: index as u32,
            resource: buffers[index].as_entire_binding(),
        });

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

    pub fn update(&self, gpu: &GPU, uniforms: &[&[u8]; BINDINGS]) -> Result<()> {
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
