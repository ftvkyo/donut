use anyhow::{Result, ensure};
use winit::dpi::PhysicalSize;

use crate::view::gpu::GPU;

pub struct TextureDepth {
    pub texture: wgpu::Texture,
    pub texture_view: wgpu::TextureView,
}

impl TextureDepth {
    pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn new(gpu: &GPU, size: PhysicalSize<u32>) -> Result<Self> {
        ensure!(size.width > 0);
        ensure!(size.height > 0);

        let size = wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        };

        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&Default::default());

        Ok(Self {
            texture,
            texture_view,
        })
    }

    pub fn update(&mut self, gpu: &GPU, size: PhysicalSize<u32>) -> Result<()> {
        ensure!(size.width > 0);
        ensure!(size.height > 0);

        let size = wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        };

        self.texture.destroy();
        self.texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        self.texture_view = self.texture.create_view(&Default::default());

        Ok(())
    }
}

impl Drop for TextureDepth {
    fn drop(&mut self) {
        self.texture.destroy();
    }
}
