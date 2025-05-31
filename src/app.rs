use std::sync::Arc;

use log::info;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

use crate::{game::Game, renderer::Renderer};

#[derive(Default)]
pub struct App {
    renderer: Option<Renderer>,
    game: Game,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_size = winit::dpi::LogicalSize::new(1366, 768);

        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_inner_size(window_size)
                        .with_resizable(false),
                )
                .unwrap(),
        );

        let state = pollster::block_on(Renderer::new(window.clone(), &self.game));
        self.renderer = Some(state);

        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let renderer = self.renderer.as_mut().unwrap();
        match event {
            WindowEvent::CloseRequested => {
                info!("Received close request, stopping...");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                renderer.render(&self.game);
                renderer.get_window().request_redraw();
            }
            WindowEvent::Resized(size) => {
                renderer.resize(size);
                // No need to re-render as the next event will be RedrawRequested
            }
            _ => (),
        }
    }
}
