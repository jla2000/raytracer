#version 460

layout(binding = 0, location = 0) uniform sampler2D rawTexture;

layout(location = 0) out vec4 outputColor;

void main() {
  outputColor = texture(rawTexture, gl_FragCoord.xy);
}
