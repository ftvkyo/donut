use crate::view::gpu::GPU;

use anyhow::{Result, ensure};
use winit::dpi::PhysicalSize;

pub struct DeferredInputViews<'a> {
    pub color: &'a wgpu::TextureView,
    pub normal_depth: &'a wgpu::TextureView,
}

pub struct DeferredInput {
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl DeferredInput {
    fn new_bind_group(
        gpu: &GPU,
        views: &DeferredInputViews,
        layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Deferred Inputs"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&views.color),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&views.normal_depth),
                },
            ],
        })
    }

    pub fn new(gpu: &GPU, views: &DeferredInputViews) -> Self {
        let bind_group_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[
                        // Color texture
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        // Normal & Depth texture
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                    ],
                });

        let bind_group = Self::new_bind_group(gpu, views, &bind_group_layout);

        Self {
            bind_group_layout,
            bind_group,
        }
    }

    pub fn update(&mut self, gpu: &GPU, views: &DeferredInputViews) {
        self.bind_group = Self::new_bind_group(gpu, views, &self.bind_group_layout);
    }
}

pub struct DeferredTextureGroup {
    pub color: wgpu::Texture,
    pub color_view: wgpu::TextureView,
    pub normal_depth: wgpu::Texture,
    pub normal_depth_view: wgpu::TextureView,
}

impl DeferredTextureGroup {
    pub const FORMAT_COLOR: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
    pub const FORMAT_NORMAL_DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

    fn new_texture(
        gpu: &GPU,
        label: &str,
        size: wgpu::Extent3d,
        format: wgpu::TextureFormat,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let texture_view = texture.create_view(&Default::default());

        (texture, texture_view)
    }

    pub fn new(gpu: &GPU, dimensions: PhysicalSize<u32>) -> Result<Self> {
        ensure!(dimensions.width > 0, "textures can't be 0 pixels wide");
        ensure!(dimensions.height > 0, "textures can't be 0 pixels tall");

        let PhysicalSize { width, height } = dimensions;

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let (color, color_view) =
            Self::new_texture(gpu, "Deferred Color Data", size, Self::FORMAT_COLOR);
        let (normal_depth, normal_depth_view) = Self::new_texture(
            gpu,
            "Deferred Normal & Depth Data",
            size,
            Self::FORMAT_NORMAL_DEPTH,
        );

        Ok(Self {
            color,
            color_view,
            normal_depth,
            normal_depth_view,
        })
    }

    pub fn update(&mut self, gpu: &GPU, dimensions: PhysicalSize<u32>) -> Result<()> {
        ensure!(dimensions.width > 0, "textures can't be 0 pixels wide");
        ensure!(dimensions.height > 0, "textures can't be 0 pixels tall");

        let PhysicalSize { width, height } = dimensions;

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        self.color.destroy();
        (self.color, self.color_view) =
            Self::new_texture(gpu, "Deferred Color Data", size, Self::FORMAT_COLOR);

        self.normal_depth.destroy();
        (self.normal_depth, self.normal_depth_view) = Self::new_texture(
            gpu,
            "Deferred Normal & Depth Data",
            size,
            Self::FORMAT_NORMAL_DEPTH,
        );

        Ok(())
    }
}

impl Drop for DeferredTextureGroup {
    fn drop(&mut self) {
        self.color.destroy();
        self.normal_depth.destroy();
    }
}
