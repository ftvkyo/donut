struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) frag_pos: vec4<f32>,
    @location(1) frag_norm: vec3<f32>,
    @location(2) light_pos: vec4<f32>,
    @location(3) tex_coord: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> view: mat4x4<f32>;

@group(0) @binding(1)
var<uniform> proj: mat4x4<f32>;

@group(1) @binding(0)
var<uniform> light_pos: vec4<f32>;

@vertex
fn vs_main(
    @location(0) position: vec4<f32>,
    @location(1) tex_coord: vec2<f32>,
) -> VertexOutput {
    let normal = vec3(0.0, 0.0, 1.0);

    var result: VertexOutput;
    result.position = proj * view * position;
    result.frag_pos = view * position;
    result.frag_norm = normal;
    result.light_pos = view * light_pos;
    result.tex_coord = tex_coord;

    return result;
}

@group(2) @binding(0)
var texture: texture_2d<f32>;

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let light_color = vec3(1.0, 1.0, 1.0);

    let view_dir = normalize(-vertex.frag_pos).xyz;
    let light_dir = normalize(vertex.light_pos - vertex.frag_pos).xyz;

    let light_ambient = 0.1;

    let light_diffuse_strength = 0.75;
    let light_diffuse = max(dot(vertex.frag_norm, light_dir), 0.0);

    let light_specular_strength = 0.75;
    let light_specular = pow(max(dot(view_dir, reflect(-light_dir, vertex.frag_norm)), 0.0), 32);

    let tex = textureLoad(texture, vec2<i32>(vertex.tex_coord), 0);

    if tex.a == 0.0 {
        discard;
    }

    let light = light_color * (
        light_ambient
        + light_diffuse * light_diffuse_strength
        + light_specular * light_specular_strength
    );

    return vec4(tex.rgb * light, 1.0);
}
