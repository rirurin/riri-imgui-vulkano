#version 450 core

layout(location = 0) in vec4 col;
layout(location = 1) in vec2 uv;

// layout(set = 0, binding = 0) uniform sampler2D tex;

layout(location = 0) out vec4 fColor;

void main()
{
    // fColor = col * texture(tex, uv);
    fColor = col;
}