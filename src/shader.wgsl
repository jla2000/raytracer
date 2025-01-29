@group(0) @binding(0)
var render_texture: texture_storage_2d<bgra8unorm, read_write>;

@group(0) @binding(1)
var<uniform> camera: RayCamera;

struct RayCamera {
  inverse_proj: mat4x4<f32>,
  inverse_view: mat4x4<f32>,
  position: vec3<f32>,
}

@compute
@workgroup_size(10, 10, 1)
fn render(@builtin(global_invocation_id) gid: vec3<u32>) {
  let pixel_coords = gid.xy;
  let surface_size = vec2<f32>(textureDimensions(render_texture).xy);
  let normalized_device_coords = (vec2<f32>(pixel_coords)) / surface_size * 2.0 - 1.0;
  let view_coords = camera.inverse_proj * vec4(normalized_device_coords, 0.0, 1.0);
  let world_coords = camera.inverse_view * view_coords;

  let ray_origin = camera.position;
  let ray_direction = normalize(world_coords.xyz - ray_origin);

  textureStore(render_texture, pixel_coords, vec4(ray_direction, 1.0));
}
