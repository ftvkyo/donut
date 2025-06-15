use std::sync::Arc;

use bytemuck::Zeroable;
use glam::vec4;
use log::info;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

use crate::{
    game::Game,
    view::{
        View,
        light::{LIGHT_COUNT, Light},
    },
};

pub struct App {
    view: Option<View>,
    game: Game,
    scene: isize,
    reset: bool,
}

impl App {
    const SCENES: isize = 4;

    pub fn new(game: Game) -> Self {
        Self {
            view: None,
            game,
            scene: 0,
            reset: true,
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
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        physical_key,
                        ..
                    },
                ..
            } => match physical_key {
                PhysicalKey::Code(KeyCode::ArrowRight) => {
                    self.scene += 1;
                    self.reset = true;
                }
                PhysicalKey::Code(KeyCode::ArrowLeft) => {
                    self.scene -= 1;
                    self.reset = true;
                }
                _ => (),
            },
            _ => (),
        }

        if self.scene < 0 {
            self.scene += Self::SCENES;
        }
        if self.scene >= Self::SCENES {
            self.scene -= Self::SCENES;
        }

        if self.reset {
            self.reset = false;

            view.update_lights(|_camera, lights| {
                *lights = [Light::zeroed(); LIGHT_COUNT];
            });

            match self.scene {
                0 => view.update_lights(|camera, lights| {
                    let view = camera.matrix_view();
                    lights[0].position = view * vec4(4.0, 4.0, 0.2, 1.0);
                    lights[0].color = vec4(1.0, 1.0, 1.0, 1.0);
                }),
                1 => view.update_lights(|camera, lights| {
                    let view = camera.matrix_view();
                    lights[0].position = view * vec4(3.0, 4.0, 0.2, 1.0);
                    lights[0].color = vec4(1.0, 1.0, 1.0, 1.0);
                    lights[1].position = view * vec4(5.0, 4.0, 0.2, 1.0);
                    lights[1].color = vec4(1.0, 1.0, 1.0, 1.0);
                }),
                2 => view.update_lights(|camera, lights| {
                    let view = camera.matrix_view();
                    lights[0].position = view * vec4(4.0, 5.5, 0.5, 1.0);
                    lights[0].color = vec4(1.0, 1.0, 1.0, 1.0);
                    lights[1].position = view * vec4(6.0, 2.5, 1.0, 1.0);
                    lights[1].color = vec4(1.0, 1.0, 1.0, 1.0);
                    lights[2].position = view * vec4(2.0, 2.5, 2.0, 1.0);
                    lights[2].color = vec4(1.0, 1.0, 1.0, 1.0);
                }),
                3 => view.update_lights(|camera, lights| {
                    let view = camera.matrix_view();
                    lights[0].position = view * vec4(3.0, 3.0, 0.5, 1.0);
                    lights[0].color = vec4(1.0, 0.0, 0.0, 1.0);
                    lights[1].position = view * vec4(3.0, 5.0, 0.5, 1.0);
                    lights[1].color = vec4(0.0, 1.0, 0.0, 1.0);
                    lights[2].position = view * vec4(5.0, 3.0, 0.5, 1.0);
                    lights[2].color = vec4(0.0, 0.0, 1.0, 1.0);
                    lights[3].position = view * vec4(5.0, 5.0, 0.5, 1.0);
                    lights[3].color = vec4(0.5, 0.5, 0.5, 1.0);
                }),
                _ => panic!("No such scene: {}", self.scene),
            }
        }
    }
}
