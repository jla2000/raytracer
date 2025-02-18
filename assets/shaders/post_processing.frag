#version 460

layout(local_size_x = 32, local_size_y = 32) in;
layout(binding = 0, location = 0, rgba8) uniform image2D outputTexture;

void main() {
  ivec2 pixelCoords = ivec2(gl_GlobalInvocationID.xy);
  ivec2 outputTextureSize = imageSize(outputTexture);

  if (pixelCoords.x >= outputTextureSize.x || pixelCoords.y >= outputTextureSize.y) {
    return;
  }

  imageStore(outputTexture, pixelCoords, vec4(1.0, 0.0, 0.0, 1.0));
}
