@group(0) @binding(0)
var render_texture: texture_storage_2d<bgra8unorm, read_write>;

@group(0) @binding(1)
var<uniform> camera: CameraMatrices;

var<push_constant> push_constants: PushConstants;

struct CameraMatrices {
  inverse_proj: mat4x4<f32>,
  inverse_view: mat4x4<f32>,
}

struct PushConstants {
  time: f32,
  num_samples: u32,
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

fn trace_ray(ray: Ray) -> vec3<f32> {
  if (hit_sphere(vec3(0, 0.0, 1), 0.5, ray)) {
    return vec3(1.0, 0.0, 0.0);
  }
  if (ray.direction.y < 0) {
    return vec3(0.0, 1.0, 0.0);
  }

  return sky_color(ray);
}

fn random(x: f32) -> f32 {
    return fract(sin(x) * 43758.5453); // Generates a pseudo-random value
}
fn random2d(coord: vec2<f32>) -> vec2f {
    let randX = random(push_constants.time + coord.x + 0.0);  // Offset to ensure randomness in X
    let randY = random(push_constants.time + coord.y + 1.0);  // Offset to ensure randomness in Y
    return vec2f(randX, randY);    // Return the random 2D vector
}

@compute
@workgroup_size(10, 10, 1)
fn render(@builtin(global_invocation_id) gid: vec3<u32>) {
  let render_texture_size = vec2<f32>(textureDimensions(render_texture).xy);

  let pixel = vec2<f32>(gid.xy) + random2d(vec2<f32>(gid.xy));

  let ndc = vec2<f32>(
      f32(pixel.x) / render_texture_size.x * 2.0 - 1.0,
      1.0 - f32(pixel.y) / render_texture_size.y * 2.0
  );

  let origin_world_space = camera.inverse_view * vec4(0, 0, 0, 1);
  let direction_view_space = normalize(camera.inverse_proj * vec4(ndc, 0.0, 1.0));
  let direction_world_space = normalize(camera.inverse_view * vec4(direction_view_space.xyz, 0));

  let ray_color = trace_ray(Ray(origin_world_space.xyz, direction_world_space.xyz));

  let old_color = textureLoad(render_texture, gid.xy).xyz;
  let mixed_color = mix(old_color, ray_color, (1.0 / f32(push_constants.num_samples)));

  textureStore(render_texture, gid.xy, vec4(mixed_color, 1.0));
}
