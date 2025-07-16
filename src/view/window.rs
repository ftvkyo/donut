use std::sync::Arc;

use anyhow::{Context, Result};
use winit::dpi::PhysicalSize;

use crate::view::gpu::GPU;

pub struct Window {
    window: Arc<winit::window::Window>,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    surface_view_format: wgpu::TextureFormat,
}

impl Window {
    pub fn new(gpu: &GPU, window: Arc<winit::window::Window>) -> Result<Self> {
        let surface = gpu.instance.create_surface(window.clone()).unwrap();
        let capabilities = surface.get_capabilities(&gpu.adapter);

        let surface_format = capabilities
            .formats
            .into_iter()
            .find(|fmt| fmt.has_color_aspect() && fmt.is_srgb())
            .context("No suitable surface format found")?;

        let surface_view_format = surface_format.remove_srgb_suffix();

        Ok(Self {
            window,
            surface,
            surface_format,
            surface_view_format,
        })
    }

    pub fn configure(&self, gpu: &GPU) {
        let size = self.size();

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            view_formats: vec![self.surface_view_format],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: size.width,
            height: size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };

        self.surface.configure(&gpu.device, &surface_config);
    }

    pub fn texture(&self) -> Result<(wgpu::SurfaceTexture, wgpu::TextureView)> {
        let surface_texture = self
            .surface
            .get_current_texture()
            .context("failed to acquire next swapchain texture")?;

        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                format: Some(self.surface_view_format),
                ..Default::default()
            });

        Ok((surface_texture, surface_view))
    }

    pub fn size(&self) -> PhysicalSize<u32> {
        self.window.inner_size()
    }

    pub fn output_format(&self) -> wgpu::TextureFormat {
        self.surface_view_format
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    pub fn pre_present_notify(&self) {
        self.window.pre_present_notify();
    }
}
