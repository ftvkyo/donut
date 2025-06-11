use std::sync::Arc;

use log::info;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

use crate::{game::Game, view::View};

pub struct App {
    view: Option<View>,
    game: Game,
}

impl App {
    pub fn new(game: Game) -> Self {
        Self { view: None, game }
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

        let state = pollster::block_on(View::new(window.clone(), &self.game));
        self.view = Some(state);

        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let view = self.view.as_mut().unwrap();

        match event {
            WindowEvent::CloseRequested => {
                info!("Received close request, stopping...");
                event_loop.exit();
                return;
            }
            WindowEvent::RedrawRequested => {
                view.render(&self.game);
            }
            WindowEvent::Resized(_) => {
                view.resize();
                // No need to re-render as the next event will be RedrawRequested
            }
            WindowEvent::KeyboardInput { event, .. } => match event.physical_key {
                PhysicalKey::Code(KeyCode::ArrowUp) => {
                    self.game.movement.accel_u = event.state.is_pressed();
                }
                PhysicalKey::Code(KeyCode::ArrowRight) => {
                    self.game.movement.accel_r = event.state.is_pressed();
                }
                PhysicalKey::Code(KeyCode::ArrowDown) => {
                    self.game.movement.accel_d = event.state.is_pressed();
                }
                PhysicalKey::Code(KeyCode::ArrowLeft) => {
                    self.game.movement.accel_l = event.state.is_pressed();
                }
                _ => (),
            },
            _ => (),
        }

        // TODO: make sure this is called at a reasonable frequency (not too frequently, not too infrequently)
        self.game.movement.advance();
        let position = self.game.movement.get_position();

        view.update_light(|light| {
            light.position = position;
        });
    }
}
