@group(0) @binding(0)
var render_texture: texture_storage_2d<bgra8unorm, read_write>;

@group(0) @binding(1)
var<uniform> camera: CameraMatrices;

var<push_constant> push_constants: PushConstants;

var<private> rng_state: u32;

struct CameraMatrices {
  inverse_proj: mat4x4<f32>,
  inverse_view: mat4x4<f32>,
}

struct PushConstants {
  time: f32,
  num_samples: u32,
}

struct Ray {
  origin: vec3f,
  direction: vec3f,
}

fn hit_sphere(center: vec3f, radius: f32, ray: Ray) -> f32 {
  let oc = center - ray.origin;
  let a = dot(ray.direction, ray.direction);
  let b = -2.0 * dot(ray.direction, oc);
  let c = dot(oc, oc) - radius*radius;
  let discriminant = b*b - 4*a*c;

  if discriminant <= 0 {
    return -1.0;
  } else {
    return (-b - sqrt(discriminant)) / 2.0 * a;
  }
}

fn sky_color(ray: Ray) -> vec3f {
  let a = 0.5 * (ray.direction.y + 1.0);
  return (1.0 - a) * vec3(1.0, 1.0, 1.0) + a * vec3(0.5, 0.7, 1.0);
}

fn trace_ray(ray: Ray) -> vec3f {
  let dist = hit_sphere(vec3(0, 0, -1), 0.5, ray);
  if (dist > 0) {
    let normal = normalize((ray.origin + ray.direction * dist) - vec3f(0, 0, -1));
    return 0.5 * (normal + 1.0);
  }
  if (ray.direction.y < 0) {
    return vec3(0.0, 0.5, 0.0);
  }

  return sky_color(ray);
}

fn rand_xorshift() -> u32 {
  rng_state ^= (rng_state << 13);
  rng_state ^= (rng_state >> 17);
  rng_state ^= (rng_state << 5);
  return rng_state;
}

fn rand_lcg() -> u32 {
  rng_state = 1664525 * rng_state + 1013904223;
  return rng_state;
}

fn rand_float() -> f32 {
  return f32(rand_lcg()) / pow(2.0, 32.0);
}

fn random_noise(coord: vec2f) -> vec2f {
  return vec2f(rand_float(), rand_float()) - 0.5;
}

@compute
@workgroup_size(10, 10, 1)
fn render(@builtin(global_invocation_id) gid: vec3u) {
  rng_state = (gid.x * 1973 + gid.y * 9277 + push_constants.num_samples * 26688) | 1;

  let render_texture_size = vec2f(textureDimensions(render_texture).xy);

  let pixel = vec2f(gid.xy) + random_noise(vec2f(gid.xy));

  let ndc = vec2f(
    pixel.x / render_texture_size.x * 2.0 - 1.0,
    1.0 - pixel.y / render_texture_size.y * 2.0
  );

  let origin_world_space = camera.inverse_view * vec4(0, 0, 0, 1);
  let direction_view_space = normalize(camera.inverse_proj * vec4(ndc, 0.0, 1.0));
  let direction_world_space = normalize(camera.inverse_view * vec4(direction_view_space.xyz, 0));

  let ray_color = trace_ray(Ray(origin_world_space.xyz, direction_world_space.xyz));

  let old_color = textureLoad(render_texture, gid.xy).xyz;
  let mixed_color = mix(old_color, ray_color, (1.0 / f32(push_constants.num_samples)));
  textureStore(render_texture, gid.xy, vec4(mixed_color, 1.0));

  //let rand = rand_float();
  //textureStore(render_texture, gid.xy, vec4(rand, rand, rand, 1.0));
}
