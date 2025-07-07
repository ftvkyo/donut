// This shader receives visibility polygons for each of the lights in the scene
// and samples textures generated in a different shader.

struct VertexInput {
    @location(0) pos: vec4<f32>,
    @location(1) light_pos: vec4<f32>,
    @location(2) light_color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) @interpolate(linear) frag_pos: vec4<f32>,
    @location(1) light_pos: vec4<f32>,
    @location(2) light_color: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> view: mat4x4<f32>;

@group(0) @binding(1)
var<uniform> proj: mat4x4<f32>;

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var result: VertexOutput;
    result.pos = proj * view * vertex.pos;
    result.frag_pos = view * vertex.pos;
    result.light_pos = view * vertex.light_pos;
    result.light_color = vertex.light_color;

    return result;
}

// The value is the color.
// Sampled by the integer XY position in screen space.
@group(1) @binding(0)
var tex_color: texture_2d<f32>;

// The value of the normal in View space in XYZ and the depth in View space in W.
// Sampled by the integer XY position in screen space.
@group(1) @binding(1)
var tex_normal_depth: texture_2d<f32>;

struct FragmentSample {
    color: vec3<f32>,
    normal: vec3<f32>,
    depth: f32,
}

fn get_sample(frag_pos_fb: vec2<f32>) -> FragmentSample {
    let pos = vec2<u32>(frag_pos_fb);

    var result: FragmentSample;
    result.color = textureLoad(tex_color, pos, 0).rgb;
    let normal_depth = textureLoad(tex_normal_depth, pos, 0);
    result.normal = normal_depth.xyz;
    result.depth = normal_depth.w;

    return result;
}

fn get_light(frag_pos: vec4<f32>, frag_normal: vec3<f32>, light_pos: vec4<f32>, light_color: vec4<f32>) -> vec4<f32> {
    let specular_shininess = 32.0;

    if (light_color.a == 0.0) {
        return vec4(0.0);
    }

    // Direction: Fragment -> Light
    let dir_light = normalize(light_pos - frag_pos).xyz;

    // Don't illuminate from the back
    // if (dir_light.z >= 0.0) {
    //     return vec4(0.0);
    // }

    // Diffuse component:
    // - depends on the angle between the light ray and fragment normal
    // - clamped with 0 from below
    let diffuse = max(dot(frag_normal, dir_light), 0.0);

    // Direction: Fragment -> Eye
    let dir_eye = normalize(- frag_pos).xyz;

    // Direction: halfway (between dir_light & dir_eye)
    let dir_halfway = normalize(dir_light + dir_eye);

    // Specular component:
    // - depends on the fragment normal, light ray and eye ray
    // - clamped with 0 from below
    let specular = pow(max(dot(frag_normal, dir_halfway), 0.0), specular_shininess);

    // Decrease the strength of the diffuse and specular components with distance
    let distance = length((light_pos - frag_pos).xyz);
    let distance_factor = 1 / (distance * distance);

    return vec4(light_color.rgb, light_color.a * distance_factor * (diffuse + specular));
}

@fragment
fn fs_main(frag: VertexOutput) -> @location(0) vec4<f32> {
    let frag_sample = get_sample(frag.pos.xy);
    let frag_sample_pos = vec4(frag.frag_pos.xy, frag_sample.depth, 1.0);

    let light = get_light(frag_sample_pos, frag_sample.normal, frag.light_pos, frag.light_color);

    return vec4(frag_sample.color * light.rgb, light.a);
}
