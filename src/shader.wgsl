@group(0) @binding(0)
var render_texture: texture_storage_2d<bgra8unorm, read_write>;

@group(0) @binding(1)
var<uniform> camera: CameraMatrices;

struct CameraMatrices {
  inverse_proj: mat4x4<f32>,
  inverse_view: mat4x4<f32>,
}

struct Ray {
  origin: vec3<f32>,
  direction: vec3<f32>,
}

fn hit_sphere(center: vec3<f32>, radius: f32, ray: Ray) -> bool {
  let oc = center - ray.origin;
  let a = dot(ray.direction, ray.direction);
  let b = -2.0 * dot(ray.direction, oc);
  let c = dot(oc, oc) - radius*radius;
  let discriminant = b*b - 4*a*c;
  return discriminant >= 0;
}

fn sky_color(ray: Ray) -> vec3<f32> {
  let a = 0.5 * (ray.direction.y + 1.0);
  return (1.0 - a) * vec3(1.0, 1.0, 1.0) + a * vec3(0.5, 0.7, 1.0);
}

fn ray_color(ray: Ray) -> vec3<f32> {
  if (hit_sphere(vec3(0, 0, 1), 0.5, ray)) {
    return vec3(1.0, 0.0, 0.0);
  }

  return sky_color(ray);
}

@compute
@workgroup_size(10, 10, 1)
fn render(@builtin(global_invocation_id) gid: vec3<u32>) {
  let render_texture_size = vec2<f32>(textureDimensions(render_texture).xy);

  let ndc = vec2<f32>(
      f32(gid.x) / render_texture_size.x * 2.0 - 1.0,
      1.0 - f32(gid.y) / render_texture_size.y * 2.0
  );

  let origin_world_space = camera.inverse_view * vec4(0, 0, 0, 1);
  let direction_view_space = normalize(camera.inverse_proj * vec4(ndc, 0.0, 1.0));
  let direction_world_space = normalize(camera.inverse_view * vec4(direction_view_space.xyz, 0));

  textureStore(render_texture, gid.xy, vec4(ray_color(Ray(origin_world_space.xyz, direction_world_space.xyz)), 1.0));
}
