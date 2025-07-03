mod gpu;
mod gpu_data;
mod gpu_struct;
mod window;

use std::sync::Arc;

use anyhow::Result;

use gpu::GPU;
use window::Window;

use crate::{
    assets::Assets,
    game::Game,
    view::{
        gpu::{PipelineConfig, PipelineExecution, PipelineShaderConfig, RenderPass},
        gpu_data::{TextureDepth, TextureGroup, TextureMultiplexer, UniformGroup, VertexData},
    },
};

pub use gpu_struct::light;
pub use gpu_struct::quad::Quad;

// Handles for the data stored on the GPU
struct ViewGPUData {
    pub camera_uniform: UniformGroup,

    pub depth: TextureDepth,

    pub main_tmux: TextureMultiplexer,
    pub main_quads: VertexData,

    pub light_tmux: TextureMultiplexer,
    pub light_uniform: UniformGroup,
    pub light_quads: VertexData,
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

        let shader_name = "main";
        let shader_source = assets.find_shader(shader_name)?;

        let gpu_data = {
            let mut main_tmux = Vec::new();
            for (_, tdata) in assets.all_tilesets() {
                let tgroup =
                    TextureGroup::new(&gpu, &[&tdata.texture_color, &tdata.texture_normal])?;
                main_tmux.push(tgroup);
            }
            let main_tmux = TextureMultiplexer::new(&gpu, main_tmux)?;

            let mut light_tmux = Vec::new();
            for (_, tdata) in assets.all_lights() {
                let tgroup = TextureGroup::new(&gpu, &[&tdata.texture])?;
                light_tmux.push(tgroup);
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

            let depth = TextureDepth::new(&gpu, window.size())?;

            let map = assets.get_map(game.map_num).unwrap();
            let main_quads = VertexData::new_quads(&gpu, &map.quads()?)?;

            let lights_uniform = game.lights.uniform_data(&camera_view)?;
            let light_uniform =
                UniformGroup::new(&gpu, &[bytemuck::cast_slice(&[lights_uniform])])?;
            let light_quads = VertexData::new_quads(&gpu, &game.lights.quad_data(0)?)?;

            ViewGPUData {
                camera_uniform,

                depth,

                main_tmux,
                main_quads,

                light_uniform,
                light_tmux,
                light_quads,
            }
        };

        let shader = gpu.create_shader(shader_source);

        let pipeline_main = {
            let groups = [
                gpu_data.camera_uniform.get_bind_group_layout(),
                gpu_data.light_uniform.get_bind_group_layout(),
                gpu_data.main_tmux.get_bind_group_layout(),
            ];

            let pipeline = gpu.create_pipeline(&PipelineConfig {
                shader: &PipelineShaderConfig {
                    shader: &shader,
                    entrypoint_vertex: None,
                    entrypoint_fragment: Some("fs_main"),
                },
                groups: &groups,
                output: window.output_format(),
            });

            pipeline
        };

        let pipeline_light = {
            let groups = [
                gpu_data.camera_uniform.get_bind_group_layout(),
                gpu_data.light_uniform.get_bind_group_layout(),
                gpu_data.light_tmux.get_bind_group_layout(),
            ];

            let pipeline = gpu.create_pipeline(&PipelineConfig {
                shader: &PipelineShaderConfig {
                    shader: &shader,
                    entrypoint_vertex: None,
                    entrypoint_fragment: Some("fs_light"),
                },
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

    pub fn resize(&mut self) -> Result<()> {
        self.window.configure(&self.gpu);
        self.gpu_data.depth = TextureDepth::new(&self.gpu, self.window.size())?;
        Ok(())
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
        self.gpu_data
            .light_quads
            .update_quads(&self.gpu, &lights_quads)?;

        Ok(())
    }

    pub fn render(&self) -> Result<()> {
        let gdata_main = [
            self.gpu_data.camera_uniform.get_bind_group(),
            self.gpu_data.light_uniform.get_bind_group(),
            self.gpu_data.main_tmux.get_bind_group(),
        ];

        let gdata_light = [
            self.gpu_data.camera_uniform.get_bind_group(),
            self.gpu_data.light_uniform.get_bind_group(),
            self.gpu_data.light_tmux.get_bind_group(),
        ];

        let pipelines = [
            PipelineExecution {
                pipeline: &self.pipeline_main,
                gdata: &gdata_main,
                vdata: &self.gpu_data.main_quads,
            },
            PipelineExecution {
                pipeline: &self.pipeline_light,
                gdata: &gdata_light,
                vdata: &self.gpu_data.light_quads,
            },
        ];

        self.gpu.render(
            &self.window,
            &RenderPass {
                depth: &self.gpu_data.depth.texture_view,
                pipelines: &pipelines,
            },
        )
    }
}
