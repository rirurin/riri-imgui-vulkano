#version 450 core

layout(location = 0) in vec2 pos;
layout(location = 1) in vec2 uv;
layout(location = 2) in vec4 col;

layout(set = 1, binding = 0) uniform Matrices {
    mat4 ortho;
} matrices;

layout(location = 0) out vec4 oColor;
layout(location = 1) out vec2 oUV;

void main()
{
    oColor = col;
    oUV = uv;
    gl_Position = matrices.ortho * vec4(pos, 0, 1);
}