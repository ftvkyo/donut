mod gpu;
mod gpu_data;
mod window;

use std::{collections::BTreeMap, sync::Arc};

use anyhow::{Context, Result};

use gpu::GPU;
use window::Window;

use crate::{
    assets::Assets,
    game::Game,
    view::gpu_data::{
        PipelineConfig, PipelineExecution, RenderPass, TextureGroup, TextureMultiplexer,
        UniformGroup, VertexData,
    },
};

pub use gpu_data::{Vertex, VertexIndex};

// Handles for the data stored on the GPU
struct ViewGPUData {
    pub camera: UniformGroup,
    pub lights: UniformGroup,
    pub tmux: TextureMultiplexer,
    // (Which texture to use, Sprites to draw)
    pub stage_layers: Vec<(String, VertexData)>,
}

pub struct View {
    gpu: GPU,
    window: Window,

    gpu_data: ViewGPUData,

    main_pipeline: wgpu::RenderPipeline,
}

impl View {
    pub fn new(window: Arc<winit::window::Window>, assets: &Assets, game: &Game) -> Result<Self> {
        let gpu = pollster::block_on(GPU::new())?;
        let window = Window::new(&gpu, window)?;

        let stage = assets
            .stages
            .get(&game.stage_name)
            .with_context(|| format!("No stage called '{}'?", game.stage_name))?;

        let gpu_data = {
            let mut tmux = BTreeMap::new();
            for (tname, tdata) in &assets.tile_sets {
                let tgroup =
                    TextureGroup::new(&gpu, &[&tdata.texture_color, &tdata.texture_normal])?;
                tmux.insert(tname.clone(), tgroup);
            }
            let tmux = TextureMultiplexer::new(&gpu, tmux)?;

            let camera_view = game.camera.matrix_view();
            let camera_proj = game.camera.matrix_proj(window.aspect_ratio());

            let camera = UniformGroup::new(
                &gpu,
                &[
                    bytemuck::cast_slice(&[camera_view]),
                    bytemuck::cast_slice(&[camera_proj]),
                ],
            )?;

            let lights = UniformGroup::new(&gpu, &[&game.lights.data(camera_view)])?;

            let mut stage_layers = Vec::with_capacity(stage.layers.len());
            for layer in &stage.layers {
                let layer_sprites = layer.sprites(&assets.tile_sets, assets.tile_size)?;

                let mut layer_vertices = Vec::with_capacity(layer_sprites.len() * 4);
                let mut layer_indices = Vec::with_capacity(layer_sprites.len() * 6);
                for (i, sprite) in layer_sprites.iter().enumerate() {
                    layer_vertices.extend_from_slice(&sprite.vertex_data());
                    layer_indices.extend_from_slice(&sprite.index_data(i as u16 * 4));
                }

                let stage_layer = VertexData::new(&gpu, &layer_vertices, &layer_indices)?;
                stage_layers.push((layer.tile_name.clone(), stage_layer));
            }

            ViewGPUData {
                tmux,
                camera,
                lights,
                stage_layers,
            }
        };

        let main_pipeline = {
            let shader_name = "main";
            let shader = assets
                .shaders
                .get(shader_name)
                .with_context(|| format!("No shader called '{shader_name}'?"))?;

            let groups = [
                gpu_data.camera.get_bind_group_layout(),
                gpu_data.lights.get_bind_group_layout(),
                gpu_data.tmux.get_bind_group_layout(),
            ];

            let pipeline = gpu.create_pipeline(&PipelineConfig {
                shader,
                groups: &groups,
                output: window.output_format(),
            });

            pipeline
        };

        Ok(Self {
            gpu,
            window,
            gpu_data,
            main_pipeline,
        })
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    pub fn resize(&mut self) {
        self.window.configure(&self.gpu);
    }

    pub fn update_camera(&self, game: &Game) -> Result<()> {
        self.gpu_data.camera.update(
            &self.gpu,
            &[
                bytemuck::cast_slice(&[game.camera.matrix_view()]),
                bytemuck::cast_slice(&[game.camera.matrix_proj(self.window.aspect_ratio())]),
            ],
        )
    }

    pub fn update_lights(&self, game: &Game) -> Result<()> {
        self.gpu_data
            .lights
            .update(&self.gpu, &[&game.lights.data(game.camera.matrix_view())])
    }

    pub fn render(&self) -> Result<()> {
        let mut gdatas = Vec::with_capacity(self.gpu_data.stage_layers.len());
        for (texture_name, _) in &self.gpu_data.stage_layers {
            gdatas.push([
                self.gpu_data.camera.get_bind_group(),
                self.gpu_data.lights.get_bind_group(),
                self.gpu_data.tmux.get_bind_group(&texture_name)?,
            ]);
        }

        let mut pipelines = Vec::with_capacity(self.gpu_data.stage_layers.len());
        for ((_, vdata), gdata) in self.gpu_data.stage_layers.iter().zip(gdatas.iter()) {
            pipelines.push(PipelineExecution {
                pipeline: &self.main_pipeline,
                gdata,
                vdata,
            })
        }

        self.gpu.render(
            &self.window,
            &RenderPass {
                pipelines: &pipelines,
            },
        )
    }
}
