use std::sync::Arc;

use winit::{dpi::PhysicalSize, window::Window};

pub struct Surface {
    size: PhysicalSize<u32>,
    pub surface: wgpu::Surface<'static>,
    pub surface_format: wgpu::TextureFormat,
    pub surface_view_format: wgpu::TextureFormat,
}

impl Surface {
    pub fn new(
        instance: &wgpu::Instance,
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        window: &Arc<Window>,
    ) -> Self {
        let surface = instance.create_surface(window.clone()).unwrap();
        let capabilities = surface.get_capabilities(&adapter);

        let surface_format = capabilities
            .formats
            .into_iter()
            .find(|fmt| fmt.has_color_aspect() && fmt.is_srgb())
            .expect("No suitable surface format found");

        let surface_view_format = surface_format.remove_srgb_suffix();

        let render_target = Self {
            size: window.inner_size(),
            surface,
            surface_format,
            surface_view_format,
        };

        render_target.configure(device);

        render_target
    }

    pub fn configure(&self, device: &wgpu::Device) {
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            view_formats: vec![self.surface_view_format],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.size.width,
            height: self.size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };

        self.surface.configure(device, &surface_config);
    }

    pub fn size(&self) -> PhysicalSize<u32> {
        self.size
    }

    pub fn resize(&mut self, device: &wgpu::Device, size: PhysicalSize<u32>) {
        self.size = size;
        self.configure(device);
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.size.width as f32 / self.size.height as f32
    }

    pub fn texture(&self) -> (wgpu::SurfaceTexture, wgpu::TextureView) {
        let surface_texture = self
            .surface
            .get_current_texture()
            .expect("failed to acquire next swapchain texture");

        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                format: Some(self.surface_view_format),
                ..Default::default()
            });

        (surface_texture, surface_view)
    }
}
