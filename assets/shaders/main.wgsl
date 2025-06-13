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

@group(1) @binding(0)
var<uniform> light_pos: vec4<f32>;

@group(2) @binding(0)
var texture_color: texture_2d<f32>;

@group(2) @binding(1)
var texture_normal: texture_2d<f32>;

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let light_color = vec3(1.0, 1.0, 1.0);

    let view_dir = normalize(-vertex.frag_pos).xyz;
    let light_vec = light_pos - vertex.frag_pos;
    let light_dir = normalize(light_vec).xyz;

    let light_distance_factor = max(4 - length(light_vec.xyz), 0.0);

    let tex_normal = textureLoad(texture_normal, vec2<i32>(vertex.tex_coord), 0).xyz;
    let frag_normal = normalize(tex_normal - vec3(0.5));

    let light_ambient_strength = 0.15;

    let light_diffuse_strength = 0.5;
    let light_diffuse = max(dot(frag_normal, light_dir), 0.0);

    let light_specular_strength = 0.75;
    let light_specular = pow(max(dot(view_dir, reflect(-light_dir, frag_normal)), 0.0), 32);

    let color = textureLoad(texture_color, vec2<i32>(vertex.tex_coord), 0);

    if color.a == 0.0 {
        discard;
    }

    let light = light_color * (
        light_ambient_strength
        + light_diffuse * light_diffuse_strength * light_distance_factor
        + light_specular * light_specular_strength * light_distance_factor
    );

    return vec4(color.rgb * light, 1.0);
}
