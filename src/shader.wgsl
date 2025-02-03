@group(0) @binding(0)
var render_texture: texture_storage_2d<bgra8unorm, read_write>;

@group(0) @binding(1)
var<uniform> camera: RayCamera;

struct RayCamera {
  inverse_proj: mat4x4<f32>,
  inverse_view: mat4x4<f32>,
}

fn hit_sphere(center: vec3<f32>, radius: f32, ray_origin: vec3<f32>, ray_direction: vec3<f32>) -> bool {
  let oc = center - ray_origin;
  let a = dot(ray_direction, ray_direction);
  let b = -2.0 * dot(ray_direction, oc);
  let c = dot(oc, oc) - radius*radius;
  let discriminant = b*b - 4*a*c;
  return discriminant >= 0;
}

fn sky_color(ray_origin: vec3<f32>, ray_direction: vec3<f32>) -> vec3<f32> {
  let a = 0.5 * (normalize(ray_direction).y + 1.0);
  return (1.0 - a) * vec3(1.0, 1.0, 1.0) + a * vec3(0.5, 0.7, 1.0);
}

fn ray_color(ray_origin: vec3<f32>, ray_direction: vec3<f32>) -> vec3<f32> {
  if (hit_sphere(vec3(0, 0, 1), 0.5, ray_origin, ray_direction)) {
    return vec3(1.0, 0.0, 0.0);
  }

  return sky_color(ray_origin, ray_direction);
}

@compute
@workgroup_size(10, 10, 1)
fn render(@builtin(global_invocation_id) gid: vec3<u32>) {
  let pixel_coords = gid.xy;
  let surface_size = vec2<f32>(textureDimensions(render_texture).xy);

  let ndc = vec2<f32>(
      f32(pixel_coords.x) / surface_size.x * 2.0 - 1.0,
      1.0 - f32(pixel_coords.y) / surface_size.y * 2.0
  );

  let direction_view_space = camera.inverse_proj * vec4(ndc, 0.0, 1.0);
  let direction_world_space = normalize(camera.inverse_view * vec4(direction_view_space.xyz, 0));

  let ray_origin = (camera.inverse_view * vec4(0, 0, 0, 1)).xyz;
  let ray_direction = direction_world_space.xyz;

  textureStore(render_texture, pixel_coords, vec4(ray_color(ray_origin, ray_direction), 1.0));
}
