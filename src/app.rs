use std::sync::Arc;

use log::info;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

use crate::{game::Game, renderer::Renderer};

pub struct App {
    renderer: Option<Renderer>,
    game: Game,
}

impl App {
    pub fn new(game: Game) -> Self {
        Self {
            renderer: None,
            game,
        }
    }
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
        let mut moved = false;

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
                renderer.update_camera(self.game.position);
                // No need to re-render as the next event will be RedrawRequested
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key,
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => match physical_key {
                PhysicalKey::Code(KeyCode::ArrowUp) => {
                    self.game.position += glam::vec2(0.0, 1.0);
                    moved = true;
                }
                PhysicalKey::Code(KeyCode::ArrowRight) => {
                    self.game.position += glam::vec2(1.0, 0.0);
                    moved = true;
                }
                PhysicalKey::Code(KeyCode::ArrowDown) => {
                    self.game.position += glam::vec2(0.0, -1.0);
                    moved = true;
                }
                PhysicalKey::Code(KeyCode::ArrowLeft) => {
                    self.game.position += glam::vec2(-1.0, 0.0);
                    moved = true;
                }
                _ => (),
            },
            _ => (),
        }

        if moved {
            renderer.update_camera(self.game.position);
        }
    }
}
