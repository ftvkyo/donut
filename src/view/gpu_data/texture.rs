use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};

use crate::{
    assets::{TextureData, TexturePixel},
    view::gpu::GPU,
};

pub struct TextureMultiplexer {
    layout: wgpu::BindGroupLayout,
    groups: BTreeMap<String, (wgpu::BindGroup, TextureGroup)>,
}

impl TextureMultiplexer {
    pub fn new(gpu: &GPU, texture_groups: BTreeMap<String, TextureGroup>) -> Result<Self> {
        let (_, first) = texture_groups
            .first_key_value()
            .context("at least one texture group is expected")?;

        let textures_per_group = first.textures.len();
        for (_, tg) in texture_groups.iter() {
            ensure!(
                textures_per_group == tg.textures.len(),
                "all texture groups should have the same number of textures"
            );
        }

        let make_layout_entry = |index| wgpu::BindGroupLayoutEntry {
            binding: index,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                multisampled: false,
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
            },
            count: None,
        };

        let layout_entries: Vec<_> = (0..textures_per_group as u32)
            .map(make_layout_entry)
            .collect();

        let layout = gpu
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &layout_entries,
            });

        let mut groups = BTreeMap::new();

        for (name, texture_group) in texture_groups {
            let group_entries: Vec<_> = texture_group
                .textures
                .iter()
                .enumerate()
                .map(|(index, (_, view))| wgpu::BindGroupEntry {
                    binding: index as u32,
                    resource: wgpu::BindingResource::TextureView(&view),
                })
                .collect();

            let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &layout,
                entries: &group_entries,
            });

            groups.insert(name, (bind_group, texture_group));
        }

        Ok(Self { layout, groups })
    }

    pub fn get_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.layout
    }

    pub fn get_bind_group(&self, name: &String) -> Result<&wgpu::BindGroup> {
        let group = self
            .groups
            .get(name)
            .with_context(|| format!("No bind group with name '{name}'?"))?;
        Ok(&group.0)
    }

    pub fn get_texture_group(&self, name: &String) -> Result<&TextureGroup> {
        let group = self
            .groups
            .get(name)
            .with_context(|| format!("No texture group with name '{name}'?"))?;
        Ok(&group.1)
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

        let copy_buffer_layout = wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(dimensions.0 * size_of::<TexturePixel>() as u32),
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
