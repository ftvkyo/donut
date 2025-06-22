pub mod camera;
pub mod light;
pub mod renderer;
pub mod surface;
pub mod texture;
pub mod vertex;

use std::sync::Arc;

use anyhow::{Context, Result};
use winit::window::Window;

use crate::{
    assets::Assets,
    game::Game,
    view::{
        camera::GPUCameraData, light::GPULightsData, renderer::Renderer, texture::GPUTextureData,
        vertex::GPUVertexData,
    },
};

pub use vertex::Vertex;

pub struct View {
    renderer: Renderer,

    camera_data: GPUCameraData,
    lights_data: GPULightsData,
    texture_data: GPUTextureData,
    vertex_data: GPUVertexData,

    pipeline: wgpu::RenderPipeline,
}

impl View {
    pub fn new(window: Arc<Window>, assets: &Assets, game: &Game) -> Result<Self> {
        let renderer = pollster::block_on(Renderer::new(window));

        let stage = assets
            .stages
            .get(&game.stage_name)
            .with_context(|| format!("No stage called '{}'?", game.stage_name))?;

        let (tile_set, sprites) = stage.layers[0].sprites(&assets.tile_sets, assets.tile_size)?;

        let mut vertex_data = Vec::with_capacity(sprites.len() * 4);
        let mut index_data = Vec::with_capacity(sprites.len() * 6);

        for (i, sprite) in sprites.iter().enumerate() {
            vertex_data.extend_from_slice(&sprite.vertex_data());
            index_data.extend_from_slice(&sprite.index_data(i as u16 * 4));
        }

        let camera_data = renderer.create_camera_data(&game.camera);
        let lights_data = renderer.create_light_data(&game.lights, &game.camera);
        let texture_data =
            renderer.create_texture_data(&tile_set.texture_color, &tile_set.texture_normal);
        let vertex_data = renderer.create_vertex_data(vertex_data, index_data);

        let shader = assets
            .shaders
            .get("main")
            .context("No shader called 'main'?")?;

        let pipeline = renderer.create_pipeline(shader, &camera_data, &lights_data, &texture_data);

        Ok(Self {
            renderer,

            camera_data,
            lights_data,
            texture_data,
            vertex_data,

            pipeline,
        })
    }
    
    pub fn request_redraw(&self) {
        self.renderer.request_redraw();
    }
    
    pub fn resize(&mut self) {
        self.renderer.resize();
    }

    pub fn render(&mut self) {
        self.renderer.render(
            &self.pipeline,
            &self.camera_data,
            &self.lights_data,
            &self.texture_data,
            &self.vertex_data,
        );
    }
}
