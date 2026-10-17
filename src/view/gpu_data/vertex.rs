use std::marker::PhantomData;

use anyhow::{Result, ensure};
use bytemuck::Pod;

use crate::view::{
    DeferredLight, Quad, QuadEmitter,
    gpu::GPU,
    gpu_struct::vertex::{Vertex, VertexDeferred, VertexEmitter, VertexIndex},
};

pub trait IndexFormat: Pod {
    const MAX: usize;

    fn format() -> wgpu::IndexFormat;
}

impl IndexFormat for u32 {
    const MAX: usize = Self::MAX as usize;

    fn format() -> wgpu::IndexFormat {
        wgpu::IndexFormat::Uint32
    }
}

pub trait VertexData {
    fn get_vertex_buffer(&self) -> &wgpu::Buffer;
    fn get_index_buffer(&self) -> &wgpu::Buffer;
    fn get_index_count(&self) -> u32;
    fn get_index_format(&self) -> wgpu::IndexFormat;
}

pub struct VertexBuffers<V: Pod, I: IndexFormat> {
    vbuffer: wgpu::Buffer,
    ibuffer: wgpu::Buffer,
    icount: usize,
    vformat: PhantomData<V>,
    iformat: PhantomData<I>,
}

impl<V: Pod, I: IndexFormat> VertexBuffers<V, I> {
    const BUFFER_PADDING: usize = 256;

    fn pad_buffer_size(size: usize) -> usize {
        (size + Self::BUFFER_PADDING - 1) & !(Self::BUFFER_PADDING - 1)
    }

    fn buffer_init(gpu: &GPU, label: &str, usage: wgpu::BufferUsages, data: &[u8]) -> wgpu::Buffer {
        let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: Self::pad_buffer_size(data.len()) as u64,
            usage: usage | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        gpu.queue.write_buffer(&buffer, 0, data);

        buffer
    }

    fn buffer_update(
        buffer: &mut wgpu::Buffer,
        gpu: &GPU,
        label: &str,
        usage: wgpu::BufferUsages,
        data: &[u8],
    ) {
        if data.len() <= buffer.size() as usize {
            gpu.queue.write_buffer(buffer, 0, data);
        } else {
            buffer.destroy();
            *buffer = Self::buffer_init(gpu, label, usage, data);
        }
    }

    pub fn new(gpu: &GPU, vdata: &[V], idata: &[I]) -> Result<Self> {
        ensure!(vdata.len() < I::MAX as usize);

        let vbuffer = Self::buffer_init(
            gpu,
            "Vertex Buffer",
            wgpu::BufferUsages::VERTEX,
            bytemuck::cast_slice(&vdata),
        );

        let ibuffer = Self::buffer_init(
            gpu,
            "Index Buffer",
            wgpu::BufferUsages::INDEX,
            bytemuck::cast_slice(&idata),
        );

        Ok(Self {
            vbuffer,
            ibuffer,
            icount: idata.len(),
            iformat: PhantomData,
            vformat: PhantomData,
        })
    }

    pub fn update(&mut self, gpu: &GPU, vdata: &[V], idata: &[I]) -> Result<()> {
        ensure!(vdata.len() < I::MAX as usize);

        Self::buffer_update(
            &mut self.vbuffer,
            gpu,
            "Vertex Buffer",
            wgpu::BufferUsages::VERTEX,
            bytemuck::cast_slice(&vdata),
        );

        Self::buffer_update(
            &mut self.ibuffer,
            gpu,
            "Index Buffer",
            wgpu::BufferUsages::INDEX,
            bytemuck::cast_slice(&idata),
        );

        self.icount = idata.len();

        Ok(())
    }
}

impl<V: Pod, I: IndexFormat> VertexData for VertexBuffers<V, I> {
    fn get_vertex_buffer(&self) -> &wgpu::Buffer {
        &self.vbuffer
    }

    fn get_index_buffer(&self) -> &wgpu::Buffer {
        &self.ibuffer
    }

    fn get_index_count(&self) -> u32 {
        self.icount as u32
    }

    fn get_index_format(&self) -> wgpu::IndexFormat {
        I::format()
    }
}

impl VertexBuffers<Vertex, VertexIndex> {
    fn convert(quads: &[Quad]) -> (Vec<Vertex>, Vec<VertexIndex>) {
        let mut vdata = Vec::with_capacity(quads.len() * 4);
        let mut idata = Vec::with_capacity(quads.len() * 6);
        for (i, quad) in quads.iter().enumerate() {
            vdata.extend_from_slice(&quad.vertex_data());
            idata.extend_from_slice(&quad.index_data(i as VertexIndex * 4));
        }
        (vdata, idata)
    }

    pub fn new_quads(gpu: &GPU, quads: &[Quad]) -> Result<Self> {
        let (vdata, idata) = Self::convert(quads);
        Self::new(gpu, &vdata, &idata)
    }

    pub fn update_quads(&mut self, gpu: &GPU, quads: &[Quad]) -> Result<()> {
        let (vdata, idata) = Self::convert(quads);
        self.update(gpu, &vdata, &idata)
    }
}

impl VertexBuffers<VertexEmitter, VertexIndex> {
    fn convert(
        quads: impl ExactSizeIterator<Item = QuadEmitter>,
    ) -> (Vec<VertexEmitter>, Vec<VertexIndex>) {
        let mut vdata = Vec::with_capacity(quads.len() * 4);
        let mut idata = Vec::with_capacity(quads.len() * 6);
        for (i, quad) in quads.enumerate() {
            vdata.extend_from_slice(&quad.vertex_data());
            idata.extend_from_slice(&quad.index_data(i as VertexIndex * 4));
        }
        (vdata, idata)
    }

    pub fn new_emitters(
        gpu: &GPU,
        quads: impl ExactSizeIterator<Item = QuadEmitter>,
    ) -> Result<Self> {
        let (vdata, idata) = Self::convert(quads);
        Self::new(gpu, &vdata, &idata)
    }

    pub fn update_emitters(
        &mut self,
        gpu: &GPU,
        quads: impl ExactSizeIterator<Item = QuadEmitter>,
    ) -> Result<()> {
        let (vdata, idata) = Self::convert(quads);
        self.update(gpu, &vdata, &idata)
    }
}

impl VertexBuffers<VertexDeferred, VertexIndex> {
    fn convert(
        lights: impl ExactSizeIterator<Item = DeferredLight>,
    ) -> (Vec<VertexDeferred>, Vec<VertexIndex>) {
        let mut vdata = Vec::new();
        let mut idata = Vec::new();
        for light in lights {
            let vertices_now = vdata.len() as VertexIndex;
            vdata.extend(light.vertex_data());
            idata.extend(light.index_data(vertices_now));
        }
        (vdata, idata)
    }

    pub fn new_lights(
        gpu: &GPU,
        lights: impl ExactSizeIterator<Item = DeferredLight>,
    ) -> Result<Self> {
        let (vdata, idata) = Self::convert(lights);
        Self::new(gpu, &vdata, &idata)
    }

    pub fn update_lights(
        &mut self,
        gpu: &GPU,
        lights: impl ExactSizeIterator<Item = DeferredLight>,
    ) -> Result<()> {
        let (vdata, idata) = Self::convert(lights);
        self.update(gpu, &vdata, &idata)
    }
}
