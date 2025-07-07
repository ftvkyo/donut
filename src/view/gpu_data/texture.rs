use anyhow::{Result, ensure};

use crate::{
    assets::{TextureData, TexturePixel},
    view::gpu::GPU,
};

// TODO: parametrize with const usize like I did for the Uniform Group

pub struct TextureGroup {
    pub(super) textures: Vec<(wgpu::Texture, wgpu::TextureView)>,
}

impl TextureGroup {
    pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
    pub const VIEW_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

    pub fn new(gpu: &GPU, texture_data: &[&TextureData]) -> Result<Self> {
        ensure!(texture_data.len() > 0, "at least one texture is expected");

        let dimensions = texture_data[0].dimensions();
        ensure!(dimensions.0 > 0, "textures can't be 0 pixels wide");
        ensure!(dimensions.1 > 0, "textures can't be 0 pixels tall");
        for td in texture_data[1..texture_data.len()].iter() {
            ensure!(
                dimensions == td.dimensions(),
                "all textures should have the same dimensions"
            );
        }

        let mut textures = Vec::with_capacity(texture_data.len());

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let bytes_per_row = dimensions.0 * size_of::<TexturePixel>() as u32;
        ensure!(bytes_per_row % 256 == 0);

        let copy_buffer_layout = wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(bytes_per_row),
            rows_per_image: None,
        };

        for td in texture_data {
            let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: Self::FORMAT,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[Self::VIEW_FORMAT],
            });

            gpu.queue.write_texture(
                texture.as_image_copy(),
                bytemuck::cast_slice(&td),
                copy_buffer_layout,
                size,
            );

            let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
                format: Some(Self::VIEW_FORMAT),
                ..Default::default()
            });

            textures.push((texture, texture_view));
        }

        Ok(Self { textures })
    }
}

impl Drop for TextureGroup {
    fn drop(&mut self) {
        for (texture, _) in &self.textures {
            texture.destroy();
        }
    }
}
