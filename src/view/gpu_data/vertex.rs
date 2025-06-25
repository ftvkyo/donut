use std::mem::offset_of;

use anyhow::{Result, ensure};
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::view::gpu::GPU;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub pos: [f32; 4],
    pub tex_coord: [f32; 2],
}

pub type VertexIndex = u16;

pub struct VertexData {
    vbuffer: wgpu::Buffer,
    ibuffer: wgpu::Buffer,
    icount: usize,
}

impl VertexData {
    pub const INDEX_FORMAT: wgpu::IndexFormat = wgpu::IndexFormat::Uint16;

    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
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
    };

    pub fn new(gpu: &GPU, vdata: &[Vertex], idata: &[VertexIndex]) -> Result<Self> {
        ensure!(vdata.len() < VertexIndex::MAX as usize);

        let vbuffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&vdata),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });

        let ibuffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&idata),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            });

        Ok(Self {
            vbuffer,
            ibuffer,
            icount: idata.len(),
        })
    }

    pub fn update(&mut self, gpu: &GPU, vdata: &[Vertex], idata: &[VertexIndex]) -> Result<()> {
        ensure!(vdata.len() < VertexIndex::MAX as usize);
        ensure!(vdata.len() * size_of::<Vertex>() <= self.vbuffer.size() as usize);
        ensure!(idata.len() * size_of::<VertexIndex>() <= self.ibuffer.size() as usize);

        gpu.queue
            .write_buffer(&self.vbuffer, 0, bytemuck::cast_slice(&vdata));
        gpu.queue
            .write_buffer(&self.ibuffer, 0, bytemuck::cast_slice(&idata));
        gpu.queue.submit([]);

        self.icount = idata.len();

        Ok(())
    }

    pub fn get_vertex_buffer(&self) -> &wgpu::Buffer {
        &self.vbuffer
    }

    pub fn get_index_buffer(&self) -> &wgpu::Buffer {
        &self.ibuffer
    }

    pub fn get_index_count(&self) -> u32 {
        self.icount as u32
    }
}
