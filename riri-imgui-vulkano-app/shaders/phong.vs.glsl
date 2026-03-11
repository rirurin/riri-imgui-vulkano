#version 450 core

layout(location = 0) in vec3 pos;
layout(location = 1) in vec3 nrm;
layout(location = 2) in vec3 col;
layout(location = 3) in vec3 uv;

layout(location = 0) out vec4 oPos;
layout(location = 1) out vec3 oNrm;
layout(location = 2) out vec3 oCol;
layout(location = 3) out vec3 oUV;

layout(set = 0, binding = 0) uniform MVP {
    mat4 view_projection;
    mat4 model;
} mvp;

void main()
{
    mat4 world = mvp.view_projection * mvp.model;
    oPos = world * vec4(pos, 1);
    oNrm = nrm;
    oCol = col;
    oUV = uv;
}