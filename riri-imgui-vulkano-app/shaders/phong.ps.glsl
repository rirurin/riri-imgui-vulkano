#version 450 core

layout(location = 0) in vec4 pos;
layout(location = 1) in vec3 nrm;
layout(location = 2) in vec3 col;
layout(location = 3) in vec3 uv;

layout(location = 0) out vec4 fColor;

layout(set = 0, binding = 0) uniform Light {
    vec3 position;
} light;

void main()
{
    // ambient color
    vec3 light_color = vec3(1, 1, 1);
    float ambient_strength = 0.1;
    vec3 ambient_color = light_color * ambient_strength;
    // diffuse color
    vec3 norm = normalize(nrm);
    vec3 light_dir = normalize(light.position - pos.xyz);
    float diff = max(dot(norm, light_dir), 0);
    vec3 diffuse_color = diff * light_color;
    // output color
    vec3 out_color = (ambient_color + diffuse_color) * col;
    fColor = vec4(out_color, 1);
}