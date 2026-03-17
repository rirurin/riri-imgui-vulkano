#version 450 core

layout(location = 0) in vec4 oColor;
layout(location = 1) in vec2 oUV;

layout(set = 0, binding = 0) uniform sampler2D sTexture;

layout(location = 0) out vec4 fColor;

void main()
{
    fColor = oColor * texture(sTexture, oUV);
}