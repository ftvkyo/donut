struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) frag_pos: vec4<f32>,
    @location(1) frag_normal: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
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
    result.frag_normal = vertex.normal;
    result.tex_coord = vertex.tex_coord;

    return result;
}

const LIGHT_COUNT: u32 = 32;

struct Light {
    position: vec4<f32>,
    color: vec4<f32>,
}

// Lights (positions in view coordinate space)
@group(1) @binding(0)
var<uniform> lights: array<Light, LIGHT_COUNT>;

// Texture color
@group(2) @binding(0)
var texture_color: texture_2d<f32>;

// Texture normal in Model coordinate space
@group(2) @binding(1)
var texture_normal: texture_2d<f32>;

fn get_color(tex_coord: vec2<f32>) -> vec4<f32> {
    return textureLoad(texture_color, vec2<i32>(tex_coord), 0);
}

fn get_normal(tex_coord: vec2<f32>) -> vec3<f32> {
    // x, y, z in range [0.0, 1.0], model coordinate space
    let model_tex = textureLoad(texture_normal, vec2<i32>(tex_coord), 0).xyz;
    // x, y, z in range [-1.0, 1.0], length = 1.0, model coordinate space
    let model_dir = normalize(model_tex - vec3(0.5));

    // View matrix may include operations which should not affect normals.
    // To cancel out translation but preserve other relative transformations,
    // the vector is recalculated after the transformation.

    let view_origin = view * vec4(0.0, 0.0, 0.0, 1.0);
    let view_point = view * vec4(model_dir, 1.0);
    let view_dir = normalize(view_point.xyz - view_origin.xyz);

    return view_dir;
}

fn get_light(frag_pos: vec4<f32>, frag_normal: vec3<f32>, tex_normal: vec3<f32>) -> vec3<f32> {
    let ambient_strength = 0.15;
    let specular_strength = 0.5;
    let specular_shininess = 32.0;
    let diffuse_strength = 0.5;

    let radius = 3.0;

    var val: vec3<f32> = vec3(ambient_strength);

    for (var i: u32 = 0; i < LIGHT_COUNT; i++) {
        let light = lights[i];

        if (light.color.a == 0.0) {
            continue;
        }

        // Direction: Fragment -> Light
        let dir_light = normalize(light.position - frag_pos).xyz;
        
        if (dot(frag_normal, dir_light) <= 0.0) {
            continue;
        }

        // Diffuse component:
        // - depends on the angle between the light ray and fragment normal
        // - clamped with 0 from below
        let diffuse = diffuse_strength * max(dot(tex_normal, dir_light), 0.0);

        // Direction: Fragment -> Eye
        let dir_eye = normalize(- frag_pos).xyz;

        // Direction: halfway (between dir_light & dir_eye)
        let dir_halfway = normalize(dir_light + dir_eye);

        // Specular component:
        // - depends on the fragment normal, light ray and eye ray
        // - clamped with 0 from below
        let specular = specular_strength * pow(max(dot(tex_normal, dir_halfway), 0.0), specular_shininess);

        // Decrease the strength of the diffuse and specular components with distance
        let distance_factor = max(radius - length((light.position - frag_pos).xyz), 0.0);

        val += light.color.rgb * distance_factor * (diffuse + specular);
    }

    return val;
}

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = get_color(vertex.tex_coord);
    let tex_normal = get_normal(vertex.tex_coord);

    if tex_color.a == 0.0 {
        discard;
    }

    let light = get_light(vertex.frag_pos, vertex.frag_normal, tex_normal);

    return vec4(tex_color.rgb * light, 1.0);
}
