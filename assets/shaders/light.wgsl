struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) tex_coord: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) frag_pos: vec4<f32>,
    @location(1) tex_coord: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> view: mat4x4<f32>;

@group(0) @binding(1)
var<uniform> proj: mat4x4<f32>;

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var result: VertexOutput;
    result.position = proj * view * vertex.position;
    result.frag_pos = view * vertex.position;
    result.tex_coord = vertex.tex_coord;

    return result;
}

// Texture color
@group(1) @binding(0)
var texture_color: texture_2d<f32>;

// Texture normal in Model coordinate space
// TODO: this binding is unnecessary, but the group layout is made to match the main shader
@group(1) @binding(1)
var texture_normal: texture_2d<f32>;

fn get_color(tex_coord: vec2<f32>) -> vec4<f32> {
    return textureLoad(texture_color, vec2<i32>(tex_coord), 0);
}

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let pos = vertex.frag_pos;
    let color = get_color(vertex.tex_coord);

    if color.a == 0.0 {
        discard;
    }

    return vec4(color.rgb, 1.0);
}
