use anyhow::{Result, ensure};
use wgpu::util::DeviceExt;

use crate::view::{
    gpu::GPU,
    gpu_struct::{
        quad::{Quad, quads2vidata},
        vertex::{Vertex, VertexIndex},
    },
};

pub struct VertexData {
    vbuffer: wgpu::Buffer,
    ibuffer: wgpu::Buffer,
    icount: usize,
}

impl VertexData {
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

    pub fn new_quads(gpu: &GPU, quads: &[Quad]) -> Result<Self> {
        let (vdata, idata) = quads2vidata(quads);
        Self::new(gpu, &vdata, &idata)
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

    pub fn update_quads(&mut self, gpu: &GPU, quads: &[Quad]) -> Result<()> {
        let (vdata, idata) = quads2vidata(quads);
        self.update(gpu, &vdata, &idata)
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
