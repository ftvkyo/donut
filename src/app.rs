use std::sync::Arc;

use log::info;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

use crate::{assets::Assets, game::Game, view::View};

pub struct App<'a> {
    assets: &'a Assets,
    game: Game<'a>,
    view: Option<View>,
}

impl<'a> App<'a> {
    pub fn new(assets: &'a Assets, game: Game<'a>) -> Self {
        Self {
            assets,
            game,
            view: None,
        }
    }
}

impl ApplicationHandler for App<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_size = winit::dpi::LogicalSize::new(1366, 768);

        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_inner_size(window_size)
                        .with_resizable(false),
                )
                .expect("Could not create the window"),
        );

        let view =
            View::new(window.clone(), &self.assets, &self.game).expect("Could not create the view");

        self.view = Some(view);

        window.request_redraw();
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: ()) {
        let _ = (event_loop, event);
        todo!("Trigger these events and handle the game loop here?")
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
                view.update_lights(&self.game).unwrap();
                view.render().unwrap();
                // Schedule rendering of the next frame
                view.request_redraw();
            }
            WindowEvent::Resized(_) => {
                view.resize().unwrap();
                view.update_camera(&self.game).unwrap();
                // No need to re-render as the next event will be RedrawRequested
            }
            _ => (),
        }

        self.game.advance();
    }
}
