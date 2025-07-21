// This shader renders the provided map onto the surfaces
// that would be sampled in the process of deferred light rendering.

struct VertexInput {
    @location(0) pos: vec4<f32>,
    @location(1) tex_num: u32,
    @location(2) tex_coord: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) @interpolate(linear) depth: f32,
    @location(1) tex_num: u32,
    @location(2) tex_coord: vec2<f32>,
};

struct FragmentOutput {
    @location(0) color_ambient: vec4<f32>,
    @location(1) color_specular: vec4<f32>,
    @location(2) normal_depth: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> view: mat4x4<f32>;

@group(0) @binding(1)
var<uniform> proj: mat4x4<f32>;

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var result: VertexOutput;
    result.pos = proj * view * vertex.pos;
    result.depth = (view * vertex.pos).z;
    result.tex_num = vertex.tex_num;
    result.tex_coord = vertex.tex_coord;

    return result;
}

@group(1) @binding(0)
var tex_color: binding_array<texture_2d<f32>>;

@group(1) @binding(1)
var tex_normal_specular: binding_array<texture_2d<f32>>;

fn get_color(tex_num: u32, tex_coord: vec2<f32>) -> vec4<f32> {
    return textureLoad(tex_color[tex_num], vec2<i32>(tex_coord), 0);
}

fn get_normal(tex_num: u32, tex_coord: vec2<f32>) -> vec3<f32> {
    // x, y, z in range [0.0, 1.0], model coordinate space
    let model_tex = textureLoad(tex_normal_specular[tex_num], vec2<i32>(tex_coord), 0).xyz;
    // x, y, z in range [-1.0, 1.0], length = 1.0, model coordinate space
    // w = 0.0 blocks translations from getting applied by the view matrix.
    let model_dir = vec4(normalize(model_tex - vec3(0.5)), 0.0);

    return normalize((view * model_dir).xyz);
}

fn get_specular(tex_num: u32, tex_coord: vec2<f32>) -> f32 {
    return textureLoad(tex_normal_specular[tex_num], vec2<i32>(tex_coord), 0).w;
}

@fragment
fn fs_main(frag: VertexOutput) -> FragmentOutput {
    let tex_color = get_color(frag.tex_num, frag.tex_coord);
    let tex_normal = get_normal(frag.tex_num, frag.tex_coord);
    let tex_specular = get_specular(frag.tex_num, frag.tex_coord);

    if tex_color.a == 0.0 {
        discard;
    }

    let ambient_strength = 0.1;

    var result: FragmentOutput;
    result.color_ambient = vec4(ambient_strength * tex_color.rgb, 1.0);
    result.color_specular = vec4(tex_color.rgb, tex_specular);
    result.normal_depth = vec4(tex_normal, frag.depth);

    return result;
}
