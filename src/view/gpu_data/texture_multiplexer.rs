use anyhow::{Context, Result, ensure};

use crate::view::{gpu::GPU, gpu_data::TextureGroup};

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
