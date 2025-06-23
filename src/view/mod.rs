pub mod camera;
pub mod lights;
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
        camera::GPUCameraData,
        lights::GPULightsData,
        renderer::{GPUPipelineData, Renderer},
        texture::GPUTextureData,
        vertex::GPUVertexData,
    },
};

pub use vertex::Vertex;

pub struct View {
    renderer: Renderer,

    camera_data: GPUCameraData,
    lights_data: GPULightsData,
    pipelines: Vec<GPUPipelineData>,
}

impl View {
    pub fn new(window: Arc<Window>, assets: &Assets, game: &Game) -> Result<Self> {
        let shader_name = "main";

        let renderer = pollster::block_on(Renderer::new(window));

        let stage = assets
            .stages
            .get(&game.stage_name)
            .with_context(|| format!("No stage called '{}'?", game.stage_name))?;

        let shader = assets
            .shaders
            .get(shader_name)
            .with_context(|| format!("No shader called '{shader_name}'?"))?;

        let camera_data = GPUCameraData::new(&renderer, &game);
        let lights_data = GPULightsData::new(&renderer, &game);

        let mut pipelines = Vec::with_capacity(stage.layers.len());
        for layer in &stage.layers {
            let (tile_set, sprites) = layer.sprites(&assets.tile_sets, assets.tile_size)?;

            let texture_data = GPUTextureData::new(&renderer, &tile_set);
            let vertex_data = GPUVertexData::new(&renderer, &sprites);

            let pipeline =
                renderer.create_pipeline(shader, &camera_data, &lights_data, &texture_data);

            pipelines.push(GPUPipelineData {
                texture_data,
                vertex_data,
                pipeline,
            });
        }

        Ok(Self {
            renderer,
            camera_data,
            lights_data,
            pipelines,
        })
    }

    pub fn update_camera(&self, game: &Game) {
        self.camera_data.update(&self.renderer, game);
    }

    pub fn update_lights(&self, game: &Game) {
        self.lights_data.update(&self.renderer, game);
    }

    pub fn request_redraw(&self) {
        self.renderer.request_redraw();
    }

    pub fn resize(&mut self) {
        self.renderer.resize();
    }

    pub fn render(&mut self) {
        self.renderer
            .render(&self.camera_data, &self.lights_data, &self.pipelines);
    }
}
