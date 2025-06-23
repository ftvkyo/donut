use std::{any::type_name, mem::offset_of};

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::{assets::Sprite, view::renderer::Renderer};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub pos: [f32; 4],
    pub tex_coord: [f32; 2],
}

pub type VertexIndex = u16;

pub struct GPUVertexData {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: usize,
}

impl GPUVertexData {
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

    pub fn new(renderer: &Renderer, sprites: &Vec<Sprite>) -> Self {
        let mut vertex_data = Vec::with_capacity(sprites.len() * 4);
        let mut index_data = Vec::with_capacity(sprites.len() * 6);

        for (i, sprite) in sprites.iter().enumerate() {
            vertex_data.extend_from_slice(&sprite.vertex_data());
            index_data.extend_from_slice(&sprite.index_data(i as u16 * 4));
        }

        if vertex_data.len() >= VertexIndex::MAX as usize {
            panic!(
                "Too many vertices to index with {}",
                type_name::<VertexIndex>()
            );
        }

        let vertex_buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertex_data),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let index_buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&index_data),
                usage: wgpu::BufferUsages::INDEX,
            });

        Self {
            vertex_buffer,
            index_buffer,
            index_count: index_data.len(),
        }
    }
}
