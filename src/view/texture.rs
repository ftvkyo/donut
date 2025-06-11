use image::Rgba;

use crate::assets::TextureData;

pub struct GPUTextureData {
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    pub _texture_color: wgpu::Texture,
    pub _texture_color_view: wgpu::TextureView,
}

impl GPUTextureData {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, texture_data: &TextureData) -> Self {
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let view_format = wgpu::TextureFormat::Rgba8Unorm;

        let size = wgpu::Extent3d {
            width: texture_data.width(),
            height: texture_data.height(),
            depth_or_array_layers: 1,
        };

        let texture_color = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[view_format],
        });

        let bytes_per_row = Some(texture_data.width() * size_of::<Rgba<u8>>() as u32);

        queue.write_texture(
            texture_color.as_image_copy(),
            bytemuck::cast_slice(&texture_data),
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

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Texture BGL"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&texture_color_view),
            }],
        });

        Self {
            bind_group_layout,
            bind_group,
            _texture_color: texture_color,
            _texture_color_view: texture_color_view,
        }
    }
}
