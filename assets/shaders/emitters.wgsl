struct VertexInput {
    @location(0) pos: vec4<f32>,
    @location(1) tint: vec4<f32>,
    @location(2) tex_num: u32,
    @location(3) tex_coord: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) tint: vec4<f32>,
    @location(1) tex_num: u32,
    @location(2) tex_coord: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> view: mat4x4<f32>;

@group(0) @binding(1)
var<uniform> proj: mat4x4<f32>;

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var result: VertexOutput;
    result.pos = proj * view * vertex.pos;
    result.tint = vertex.tint;
    result.tex_num = vertex.tex_num;
    result.tex_coord = vertex.tex_coord;

    return result;
}

@group(1) @binding(0)
var tex_color: binding_array<texture_2d<f32>>;

@group(1) @binding(1)
var tex_normal: binding_array<texture_2d<f32>>;

fn get_color(tex_num: u32, tex_coord: vec2<f32>) -> vec4<f32> {
    return textureLoad(tex_color[tex_num], vec2<i32>(tex_coord), 0);
}

@fragment
fn fs_main(frag: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = get_color(frag.tex_num, frag.tex_coord);

    if tex_color.a == 0.0 {
        discard;
    }

    return tex_color * frag.tint;
}
