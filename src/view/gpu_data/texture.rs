use anyhow::{Context, Result, ensure};

use crate::{
    assets::{TextureData, TexturePixel},
    view::gpu::GPU,
};

pub struct TextureMultiplexer {
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    groups: Vec<TextureGroup>,
}

impl TextureMultiplexer {
    pub fn new(gpu: &GPU, groups: Vec<TextureGroup>) -> Result<Self> {
        let first = groups
            .first()
            .context("at least one texture group is expected")?;

        let textures_per_group = first.textures.len();
        for g in groups.iter() {
            ensure!(
                textures_per_group == g.textures.len(),
                "all texture groups should have the same number of textures"
            );
        }

        let count = Some((groups.len() as u32).try_into()?);

        let make_layout_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                multisampled: false,
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
            },
            count,
        };

        let layout_entries: Vec<_> = (0..textures_per_group as u32)
            .map(make_layout_entry)
            .collect();

        let bind_group_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &layout_entries,
                });

        let mut texture_view_groups = vec![vec![]; textures_per_group];
        for group in &groups {
            for (itexture, (_, texture_view)) in group.textures.iter().enumerate() {
                texture_view_groups[itexture].push(texture_view);
            }
        }

        let mut entries = Vec::new();
        for (i, texture_view_group) in texture_view_groups.iter().enumerate() {
            entries.push(wgpu::BindGroupEntry {
                binding: i as u32,
                resource: wgpu::BindingResource::TextureViewArray(texture_view_group),
            });
        }

        let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &entries,
        });

        Ok(Self {
            bind_group_layout,
            bind_group,
            groups,
        })
    }

    pub fn get_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    pub fn get_bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    pub fn get_texture_group(&self, index: usize) -> Option<&TextureGroup> {
        self.groups.get(index)
    }
}

pub struct TextureGroup {
    textures: Vec<(wgpu::Texture, wgpu::TextureView)>,
}

impl TextureGroup {
    const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
    const VIEW_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

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
