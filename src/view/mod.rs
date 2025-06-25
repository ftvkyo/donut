mod gpu;
mod gpu_data;
mod gpu_struct;
mod window;

use std::{collections::BTreeMap, sync::Arc};

use anyhow::{Context, Result};

use gpu::GPU;
use window::Window;

use crate::{
    assets::Assets,
    game::Game,
    view::{
        gpu::{PipelineConfig, PipelineExecution, RenderPass},
        gpu_data::{TextureGroup, TextureMultiplexer, UniformGroup, VertexData},
    },
};

pub use gpu_struct::light;
pub use gpu_struct::quad::Quad;

// Handles for the data stored on the GPU
struct ViewGPUData {
    pub camera_uniform: UniformGroup,

    pub main_tmux: TextureMultiplexer,
    // (Which texture to use, Sprites to draw)
    pub main_quads: Vec<(String, VertexData)>,

    pub light_tmux: TextureMultiplexer,
    pub light_uniform: UniformGroup,
    // (Which animation to use, Sprites to draw)
    pub light_quads: Vec<(String, VertexData)>,
}

pub struct View {
    gpu: GPU,
    window: Window,

    gpu_data: ViewGPUData,

    pipeline_main: wgpu::RenderPipeline,
    pipeline_light: wgpu::RenderPipeline,
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
            let mut main_tmux = BTreeMap::new();
            for (tname, tdata) in &assets.tile_sets {
                let tgroup =
                    TextureGroup::new(&gpu, &[&tdata.texture_color, &tdata.texture_normal])?;
                main_tmux.insert(tname.clone(), tgroup);
            }
            let main_tmux = TextureMultiplexer::new(&gpu, main_tmux)?;

            let mut light_tmux = BTreeMap::new();
            for (tname, tdata) in &assets.lights {
                let tgroup = TextureGroup::new(&gpu, &[&tdata.texture])?;
                light_tmux.insert(tname.clone(), tgroup);
            }
            let light_tmux = TextureMultiplexer::new(&gpu, light_tmux)?;

            let camera_view = game.camera.matrix_view();
            let camera_proj = game.camera.matrix_proj(window.aspect_ratio());

            let camera_uniform = UniformGroup::new(
                &gpu,
                &[
                    bytemuck::cast_slice(&[camera_view]),
                    bytemuck::cast_slice(&[camera_proj]),
                ],
            )?;

            let lights_uniform = game.lights.uniform_data(&camera_view)?;
            let light_uniform =
                UniformGroup::new(&gpu, &[bytemuck::cast_slice(&[lights_uniform])])?;
            let light_quads = vec![(
                "fire".to_string(),
                VertexData::new_quads(&gpu, &game.lights.quad_data(0)?)?,
            )];

            let mut main_quads = Vec::with_capacity(stage.layers.len());
            for layer in &stage.layers {
                let layer_quads = layer.quads(&assets.tile_sets, assets.tile_size)?;
                let stage_layer = VertexData::new_quads(&gpu, &layer_quads)?;
                main_quads.push((layer.tile_name.clone(), stage_layer));
            }

            ViewGPUData {
                camera_uniform,

                main_tmux,
                main_quads,

                light_uniform,
                light_tmux,
                light_quads,
            }
        };

        let pipeline_main = {
            let shader_name = "main";
            let shader = assets
                .shaders
                .get(shader_name)
                .with_context(|| format!("No shader called '{shader_name}'?"))?;

            let groups = [
                gpu_data.camera_uniform.get_bind_group_layout(),
                gpu_data.light_uniform.get_bind_group_layout(),
                gpu_data.main_tmux.get_bind_group_layout(),
            ];

            let pipeline = gpu.create_pipeline(&PipelineConfig {
                shader,
                groups: &groups,
                output: window.output_format(),
            });

            pipeline
        };

        let pipeline_light = {
            let shader_name = "light";
            let shader = assets
                .shaders
                .get(shader_name)
                .with_context(|| format!("No shader called '{shader_name}'?"))?;

            let groups = [
                gpu_data.camera_uniform.get_bind_group_layout(),
                gpu_data.light_tmux.get_bind_group_layout(),
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
            pipeline_main,
            pipeline_light,
        })
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    pub fn resize(&mut self) {
        self.window.configure(&self.gpu);
    }

    pub fn update_camera(&self, game: &Game) -> Result<()> {
        self.gpu_data.camera_uniform.update(
            &self.gpu,
            &[
                bytemuck::cast_slice(&[game.camera.matrix_view()]),
                bytemuck::cast_slice(&[game.camera.matrix_proj(self.window.aspect_ratio())]),
            ],
        )
    }

    pub fn update_lights(&mut self, game: &Game) -> Result<()> {
        let lights_uniform = game.lights.uniform_data(&game.camera.matrix_view())?;
        self.gpu_data
            .light_uniform
            .update(&self.gpu, &[&bytemuck::cast_slice(&[lights_uniform])])?;

        let frame = (game.elapsed().as_millis() * 60 / 1000 / 10) as usize;
        let lights_quads = game.lights.quad_data(frame % game.lights.frame_count)?;
        self.gpu_data.light_quads[0]
            .1
            .update_quads(&self.gpu, &lights_quads)?;

        Ok(())
    }

    pub fn render(&self) -> Result<()> {
        let mut gdatas = Vec::with_capacity(self.gpu_data.main_quads.len());
        for (texture_name, _) in &self.gpu_data.main_quads {
            gdatas.push([
                self.gpu_data.camera_uniform.get_bind_group(),
                self.gpu_data.light_uniform.get_bind_group(),
                self.gpu_data.main_tmux.get_bind_group(&texture_name)?,
            ]);
        }

        let mut gdatas_lights = Vec::with_capacity(self.gpu_data.light_quads.len());
        for (light_name, _) in &self.gpu_data.light_quads {
            gdatas_lights.push([
                self.gpu_data.camera_uniform.get_bind_group(),
                self.gpu_data.light_tmux.get_bind_group(&light_name)?,
            ]);
        }

        let mut pipelines = Vec::with_capacity(self.gpu_data.main_quads.len());

        for ((_, vdata), gdata) in self.gpu_data.main_quads.iter().zip(gdatas.iter()) {
            pipelines.push(PipelineExecution {
                pipeline: &self.pipeline_main,
                gdata,
                vdata,
            })
        }

        for ((_, vdata), gdata) in self.gpu_data.light_quads.iter().zip(gdatas_lights.iter()) {
            pipelines.push(PipelineExecution {
                pipeline: &self.pipeline_light,
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
