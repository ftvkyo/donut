use image::Rgba;

use crate::{assets::TextureData, view::renderer::Renderer};

pub struct GPUTextureData {
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    pub _texture_color: wgpu::Texture,
    pub _texture_color_view: wgpu::TextureView,
    pub _texture_normal: wgpu::Texture,
    pub _texture_normal_view: wgpu::TextureView,
}

impl GPUTextureData {
    pub fn new(
        renderer: &Renderer,
        color_data: &TextureData,
        normal_data: &TextureData,
    ) -> Self {
        if color_data.dimensions() != normal_data.dimensions() {
            panic!("Color data dimensions different from Normal data dimensions.");
        }

        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let view_format = wgpu::TextureFormat::Rgba8Unorm;

        let size = wgpu::Extent3d {
            width: color_data.width(),
            height: color_data.height(),
            depth_or_array_layers: 1,
        };

        let bytes_per_row = Some(color_data.width() * size_of::<Rgba<u8>>() as u32);

        let texture_color = renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Color texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[view_format],
        });

        renderer.queue.write_texture(
            texture_color.as_image_copy(),
            bytemuck::cast_slice(&color_data),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row,
                rows_per_image: None,
            },
            size,
        );

        let texture_color_view = texture_color.create_view(&wgpu::TextureViewDescriptor {
            format: Some(view_format),
            ..Default::default()
        });

        let texture_normal = renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Normal texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        renderer.queue.write_texture(
            texture_normal.as_image_copy(),
            bytemuck::cast_slice(&normal_data),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row,
                rows_per_image: None,
            },
            size,
        );

        let texture_normal_view = texture_normal.create_view(&Default::default());

        let bind_group_layout = renderer.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Texture bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        });

        let bind_group = renderer.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture bind group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_color_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture_normal_view),
                },
            ],
        });

        Self {
            bind_group_layout,
            bind_group,
            _texture_color: texture_color,
            _texture_color_view: texture_color_view,
            _texture_normal: texture_normal,
            _texture_normal_view: texture_normal_view,
        }
    }
}
