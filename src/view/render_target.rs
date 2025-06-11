use std::sync::Arc;

use winit::window::Window;

pub struct RenderTarget {
    pub window: Arc<Window>,

    pub surface: wgpu::Surface<'static>,
    pub surface_format: wgpu::TextureFormat,
    pub surface_view_format: wgpu::TextureFormat,
}

impl RenderTarget {
    pub fn new(
        instance: &wgpu::Instance,
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        window: Arc<Window>,
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
            window,
            surface,
            surface_format,
            surface_view_format,
        };

        render_target.configure(device);

        render_target
    }

    pub fn configure(&self, device: &wgpu::Device) {
        let size = self.get_size();

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

        self.surface.configure(device, &surface_config);
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    pub fn pre_present_notify(&self) {
        self.window.pre_present_notify();
    }

    pub fn get_size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.window.inner_size()
    }

    pub fn get_aspect_ratio(&self) -> f32 {
        let size = self.get_size();
        size.width as f32 / size.height as f32
    }

    pub fn get_texture(&self) -> (wgpu::SurfaceTexture, wgpu::TextureView) {
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
