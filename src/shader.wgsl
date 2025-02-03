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

struct HitInfo {
  distance: f32,
  normal: vec3f,
  hit: bool,
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

fn trace_impl(ray: Ray) -> HitInfo {
  var dist = 0.0;

  dist = hit_sphere(vec3(0, 0, -1), 0.5, ray);
  if (dist > 0) {
    let normal = normalize((ray.origin + ray.direction * dist) - vec3f(0, 0, -1));
    return HitInfo(dist, normal, true);
  }

  dist = hit_sphere(vec3(0, -10, -5), 10, ray);
  if (dist > 0) {
    let normal = normalize((ray.origin + ray.direction * dist) - vec3f(0, -10, -5));
    return HitInfo(dist, normal, true);
  }

  return HitInfo(0.0, vec3f(0.0), false);
}

fn trace_ray(ray: Ray) -> vec3f {
  var color = sky_color(ray);
  var walk_ray = ray;

  let max_bounces = 10;
  for (var i = 0; i < max_bounces; i++) {
    let hit = trace_impl(walk_ray);
    if (hit.hit) {
      color *= 0.5;
      walk_ray.origin = walk_ray.origin + walk_ray.direction * hit.distance;
      walk_ray.direction = random_on_hemisphere(hit.normal);
    }
  }

  return color;
}


fn rand_wang() -> u32 {
  rng_state = (rng_state ^ 61) ^ (rng_state >> 16);
  rng_state *= 9;
  rng_state = rng_state ^ (rng_state >> 4);
  rng_state *= 0x27d4eb2d;
  rng_state = rng_state ^ (rng_state >> 15);
  return rng_state;
}

fn rand_float() -> f32 {
  return f32(rand_wang()) / pow(2.0, 32.0);
}

fn random_unit_vec() -> vec3f {
  return normalize(vec3f(rand_float(), rand_float(), rand_float()) - 0.5 * 2);
}

fn random_on_hemisphere(normal: vec3f) -> vec3f {
  let rand = random_unit_vec();
  if (dot(rand, normal) > 0.0) {
    return rand;
  } else {
    return -rand;
  }
}

@compute
@workgroup_size(10, 10, 1)
fn render(@builtin(global_invocation_id) gid: vec3u) {
  rng_state = (gid.x * 1973 + gid.y * 9277 + push_constants.num_samples * 26688) | 1;

  let render_texture_size = vec2f(textureDimensions(render_texture).xy);

  let pixel = vec2f(gid.xy) + vec2f(rand_float(), rand_float()) - 0.5;

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
}
