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
        gpu::{PipelineConfig, RenderConfig, RenderPass},
        gpu_data::{
            DeferredInput, DeferredInputViews, DeferredTextureGroup, TextureDepth, TextureGroup,
            TextureMultiplexer, UniformGroup, VertexBuffers,
        },
        gpu_struct::vertex::{Vertex, VertexDeferred, VertexEmitter},
    },
};

pub use gpu_struct::deferred_light::DeferredLight;
pub use gpu_struct::quad::Quad;
pub use gpu_struct::quad_emitter::QuadEmitter;

// Handles for the data stored on the GPU
struct ViewGPUData {
    pub camera: UniformGroup<2>,

    pub depth: TextureDepth,

    pub map_tmux: TextureMultiplexer,
    pub map_quads: VertexBuffers<Vertex, u16>,

    pub deferred_textures: DeferredTextureGroup,
    pub deferred_inputs: DeferredInput,
    pub deferred_lights: VertexBuffers<VertexDeferred, u16>,

    pub light_emitters_tmux: TextureMultiplexer,
    pub light_emitters_quads: VertexBuffers<VertexEmitter, u16>,
}

pub struct View {
    gpu: GPU,
    window: Window,

    gpu_data: ViewGPUData,

    pipeline_prepare: wgpu::RenderPipeline,
    pipeline_deferred: wgpu::RenderPipeline,
    pipeline_light_emitters: wgpu::RenderPipeline,
}

impl View {
    pub fn new(window: Arc<winit::window::Window>, assets: &Assets, game: &Game) -> Result<Self> {
        let gpu = pollster::block_on(GPU::new())?;
        let window = Window::new(&gpu, window)?;

        let gpu_data = {
            let mut main_tmux = Vec::new();
            for (_, tdata) in assets.all_tilesets() {
                let tgroup =
                    TextureGroup::new(&gpu, &[&tdata.texture_color, &tdata.texture_normal])?;
                main_tmux.push(tgroup);
            }
            let map_tmux = TextureMultiplexer::new(&gpu, main_tmux)?;

            let map_quads = VertexBuffers::new_quads(&gpu, &game.map.quads()?)?;

            let camera_view = game.camera.matrix_view();
            let camera_proj = game.camera.matrix_proj(window.size());

            let camera = UniformGroup::new(
                &gpu,
                &[
                    bytemuck::cast_slice(&[camera_view]),
                    bytemuck::cast_slice(&[camera_proj]),
                ],
            )?;

            let depth = TextureDepth::new(&gpu, window.size())?;

            let deferred_textures = DeferredTextureGroup::new(&gpu, window.size())?;
            let deferred_inputs = DeferredInput::new(
                &gpu,
                &DeferredInputViews {
                    color: &deferred_textures.color_view,
                    normal_depth: &deferred_textures.normal_depth_view,
                },
            );

            let deferred_lights =
                VertexBuffers::new_lights(&gpu, &game.lights.deferred_data(game.map)?)?;

            let mut light_tmux = Vec::new();
            for (_, tdata) in assets.all_lights() {
                let tgroup = TextureGroup::new(&gpu, &[&tdata.texture])?;
                light_tmux.push(tgroup);
            }
            let light_emitters_tmux = TextureMultiplexer::new(&gpu, light_tmux)?;

            let light_emitters_quads =
                VertexBuffers::new_emitters(&gpu, &game.lights.quad_data(0)?)?;

            ViewGPUData {
                camera,

                depth,

                map_tmux,
                map_quads,

                deferred_textures,
                deferred_inputs,
                deferred_lights,

                light_emitters_tmux,
                light_emitters_quads,
            }
        };

        let pipeline_prepare = {
            let shader_name = "prepare";
            let shader_source = assets.find_shader(shader_name)?;
            let shader = gpu.create_shader(shader_name, shader_source);

            let pipeline = gpu.create_pipeline(PipelineConfig {
                label: "Prepare",
                shader: &shader,
                groups: &[
                    gpu_data.camera.get_bind_group_layout(),
                    gpu_data.map_tmux.get_bind_group_layout(),
                ],
                targets: &[
                    window.output_format().into(),                    // Color Ambient
                    DeferredTextureGroup::FORMAT_COLOR.into(),        // Color
                    DeferredTextureGroup::FORMAT_NORMAL_DEPTH.into(), // Normal & Depth
                ],
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: TextureDepth::FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                vertex_layout: Vertex::LAYOUT,
            });

            pipeline
        };

        let pipeline_deferred = {
            let shader_name = "deferred";
            let shader_source = assets.find_shader(shader_name)?;
            let shader = gpu.create_shader(shader_name, shader_source);

            let target = wgpu::ColorTargetState {
                format: window.output_format(),
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent::REPLACE,
                }),
                write_mask: wgpu::ColorWrites::COLOR,
            };

            let pipeline = gpu.create_pipeline(PipelineConfig {
                label: "Deferred",
                shader: &shader,
                groups: &[
                    gpu_data.camera.get_bind_group_layout(),
                    &gpu_data.deferred_inputs.bind_group_layout,
                ],
                targets: &[target],
                depth_stencil: None,
                vertex_layout: VertexDeferred::LAYOUT,
            });

            pipeline
        };

        let pipeline_light_emitters = {
            let shader_name = "emitters";
            let shader_source = assets.find_shader(shader_name)?;
            let shader = gpu.create_shader(shader_name, shader_source);

            let pipeline = gpu.create_pipeline(PipelineConfig {
                label: "Emitters",
                shader: &shader,
                groups: &[
                    gpu_data.camera.get_bind_group_layout(),
                    gpu_data.light_emitters_tmux.get_bind_group_layout(),
                ],
                targets: &[window.output_format().into()],
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: TextureDepth::FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                vertex_layout: VertexEmitter::LAYOUT,
            });

            pipeline
        };

        Ok(Self {
            gpu,
            window,
            gpu_data,
            pipeline_prepare,
            pipeline_deferred,
            pipeline_light_emitters,
        })
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    pub fn resize(&mut self) -> Result<()> {
        self.window.configure(&self.gpu);

        self.gpu_data.depth.update(&self.gpu, self.window.size())?;

        self.gpu_data
            .deferred_textures
            .update(&self.gpu, self.window.size())?;

        self.gpu_data.deferred_inputs.update(
            &self.gpu,
            &DeferredInputViews {
                color: &self.gpu_data.deferred_textures.color_view,
                normal_depth: &self.gpu_data.deferred_textures.normal_depth_view,
            },
        );

        Ok(())
    }

    pub fn update_camera(&self, game: &Game) -> Result<()> {
        let camera_view = game.camera.matrix_view();
        let camera_proj = game.camera.matrix_proj(self.window.size());

        self.gpu_data.camera.update(
            &self.gpu,
            &[
                bytemuck::cast_slice(&[camera_view]),
                bytemuck::cast_slice(&[camera_proj]),
            ],
        )
    }

    pub fn update_lights(&mut self, game: &Game) -> Result<()> {
        let frame = (game.elapsed().as_millis() * 60 / 1000 / 10) as usize;

        let lights_quads = game.lights.quad_data(frame % game.lights.frame_count)?;
        self.gpu_data
            .light_emitters_quads
            .update_emitters(&self.gpu, &lights_quads)?;

        let lights_deferred = game.lights.deferred_data(game.map)?;
        self.gpu_data
            .deferred_lights
            .update_lights(&self.gpu, &lights_deferred)?;

        Ok(())
    }

    pub fn render(&self) -> Result<()> {
        let (window_texture, window_view) = self.window.texture()?;

        let rpass_prepare = RenderPass {
            descriptor: &wgpu::RenderPassDescriptor {
                label: Some("Render ambient light, prepare colors, normals and depths data"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &window_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.gpu_data.deferred_textures.color_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.gpu_data.deferred_textures.normal_depth_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.gpu_data.depth.texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            },
            pipeline: &self.pipeline_prepare,
            gdata: &[
                self.gpu_data.camera.get_bind_group(),
                self.gpu_data.map_tmux.get_bind_group(),
            ],
            vdata: &self.gpu_data.map_quads,
        };

        let rpass_deferred = RenderPass {
            descriptor: &wgpu::RenderPassDescriptor {
                label: Some("Render deferred diffuse and specular light"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &window_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            },
            pipeline: &self.pipeline_deferred,
            gdata: &[
                self.gpu_data.camera.get_bind_group(),
                &self.gpu_data.deferred_inputs.bind_group,
            ],
            vdata: &self.gpu_data.deferred_lights,
        };

        let rpass_light_emitters = RenderPass {
            descriptor: &wgpu::RenderPassDescriptor {
                label: Some("Render light emitters"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &window_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.gpu_data.depth.texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            },
            pipeline: &self.pipeline_light_emitters,
            gdata: &[
                self.gpu_data.camera.get_bind_group(),
                self.gpu_data.light_emitters_tmux.get_bind_group(),
            ],
            vdata: &self.gpu_data.light_emitters_quads,
        };

        self.gpu.render(&RenderConfig {
            passes: &[&rpass_prepare, &rpass_deferred, &rpass_light_emitters],
        })?;

        self.window.pre_present_notify();
        window_texture.present();

        Ok(())
    }
}
