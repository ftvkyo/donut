use std::marker::PhantomData;

use anyhow::{Result, ensure};
use bytemuck::Pod;
use wgpu::util::DeviceExt;

use crate::view::{
    DeferredLight, Quad, QuadEmitter,
    gpu::GPU,
    gpu_struct::vertex::{Vertex, VertexDeferred, VertexEmitter, VertexIndex},
};

pub trait IndexFormat: Pod {
    const MAX: usize;

    fn format() -> wgpu::IndexFormat;
}

impl IndexFormat for u16 {
    const MAX: usize = Self::MAX as usize;

    fn format() -> wgpu::IndexFormat {
        wgpu::IndexFormat::Uint16
    }
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
        let data_size = data.len();
        let data_size_padded = Self::pad_buffer_size(data_size);

        let mut data_padded = vec![0u8; data_size_padded];
        for i in 0..data.len() {
            data_padded[i] = data[i];
        }

        gpu.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: &data_padded,
                usage: usage | wgpu::BufferUsages::COPY_DST,
            })
    }

    fn buffer_update(
        buffer: &mut wgpu::Buffer,
        gpu: &GPU,
        label: &str,
        usage: wgpu::BufferUsages,
        data: &[u8],
    ) {
        let buffer_size = buffer.size() as usize;

        let data_size = data.len();
        let data_size_padded = Self::pad_buffer_size(data_size).max(buffer_size);

        let mut data_padded = vec![0u8; data_size_padded];
        for i in 0..data.len() {
            data_padded[i] = data[i];
        }

        if data_size_padded <= buffer_size {
            gpu.queue
                .write_buffer(&buffer, 0, bytemuck::cast_slice(&data_padded));
        } else {
            buffer.destroy();
            *buffer = gpu
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(label),
                    contents: &data_padded,
                    usage: usage | wgpu::BufferUsages::COPY_DST,
                });
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

        gpu.queue.submit([]);

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

impl VertexBuffers<Vertex, u16> {
    fn convert(quads: &[Quad]) -> (Vec<Vertex>, Vec<VertexIndex>) {
        let mut vdata = Vec::with_capacity(quads.len() * 4);
        let mut idata = Vec::with_capacity(quads.len() * 6);
        for (i, quad) in quads.iter().enumerate() {
            vdata.extend_from_slice(&quad.vertex_data());
            idata.extend_from_slice(&quad.index_data(i as u16 * 4));
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

impl VertexBuffers<VertexEmitter, u16> {
    fn convert(quads: &[QuadEmitter]) -> (Vec<VertexEmitter>, Vec<VertexIndex>) {
        let mut vdata = Vec::with_capacity(quads.len() * 4);
        let mut idata = Vec::with_capacity(quads.len() * 6);
        for (i, quad) in quads.iter().enumerate() {
            vdata.extend_from_slice(&quad.vertex_data());
            idata.extend_from_slice(&quad.index_data(i as u16 * 4));
        }
        (vdata, idata)
    }

    pub fn new_emitters(gpu: &GPU, quads: &[QuadEmitter]) -> Result<Self> {
        let (vdata, idata) = Self::convert(quads);
        Self::new(gpu, &vdata, &idata)
    }

    pub fn update_emitters(&mut self, gpu: &GPU, quads: &[QuadEmitter]) -> Result<()> {
        let (vdata, idata) = Self::convert(quads);
        self.update(gpu, &vdata, &idata)
    }
}

impl VertexBuffers<VertexDeferred, u16> {
    fn convert(lights: &[DeferredLight]) -> (Vec<VertexDeferred>, Vec<VertexIndex>) {
        let mut vdata = Vec::new();
        let mut idata = Vec::new();
        for light in lights {
            let vertices_now = vdata.len() as u16;
            vdata.extend(light.vertex_data());
            idata.extend(light.index_data(vertices_now));
        }
        (vdata, idata)
    }

    pub fn new_lights(gpu: &GPU, lights: &[DeferredLight]) -> Result<Self> {
        let (vdata, idata) = Self::convert(lights);
        Self::new(gpu, &vdata, &idata)
    }

    pub fn update_lights(&mut self, gpu: &GPU, lights: &[DeferredLight]) -> Result<()> {
        let (vdata, idata) = Self::convert(lights);
        self.update(gpu, &vdata, &idata)
    }
}
