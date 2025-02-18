@group(0) @binding(0)
var render_texture: texture_storage_2d<bgra8unorm, write>;

@group(0) @binding(1)
var skybox_texture: texture_storage_2d<rgba32float, read>;

@group(0) @binding(2)
var<uniform> camera: CameraMatrices;

var<push_constant> frame_id: u32;

var<private> rng_state: u32;

const PI: f32 = 3.14159265359;

struct CameraMatrices {
  inverse_proj: mat4x4<f32>,
  inverse_view: mat4x4<f32>,
}

fn sky_color(ray_desc: RayDesc) -> vec3f {
  let theta = atan2(ray_desc.dir.z, ray_desc.dir.x);
  let phi = acos(ray_desc.dir.y);

  let u = (theta + PI) / (2*PI);
  let v = phi / PI;

  let pos = vec2f(textureDimensions(skybox_texture).xy) * vec2f(u, v);

  return textureLoad(skybox_texture, vec2u(pos)).rgb;
}

fn hit_sphere(center: vec3f, radius: f32, ray: RayDesc) -> f32 {
  let oc = center - ray.origin;
  let a = dot(ray.dir, ray.dir);
  let b = -2.0 * dot(ray.dir, oc);
  let c = dot(oc, oc) - radius*radius;
  let discriminant = b*b - 4*a*c;

  if discriminant <= 0 {
    return -1.0;
  } else {
    return (-b - sqrt(discriminant)) / 2.0 * a;
  }
}

struct Sphere {
  center: vec3f,
  radius: f32,
}

fn trace_ray(ray_desc: RayDesc, gid: vec3u) -> vec3f {
  var color = vec3f(1, 1, 1);
  var ray = ray_desc;

  let spheres = array(
    Sphere(vec3f(0, 0, 0), 1.0)
  );

  for (var i = 0; i < 4; i++) {
    let dist = hit_sphere(vec3f(0, 0, 0), 1.0, ray);

    if (dist > 0.0) {
      color *= 0.5;

      let hit = ray.origin + ray.dir * dist;

      ray.origin = hit;
      ray.dir = reflect(ray.dir, normalize(hit));
    } else {
      color *= sky_color(ray);
      break;
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

fn random_unit_vec(gid: vec3u, offset: u32) -> vec3f {
  return normalize(vec3(rand_float(), rand_float(), rand_float()));
}

fn random_on_hemisphere(gid: vec3u, offset: u32, normal: vec3f) -> vec3f {
  let rand = random_unit_vec(gid, offset);
  if (dot(rand, normal) > 0.0) {
    return rand;
  } else {
    return -rand;
  }
}

fn gamma_correct(color: vec3f) -> vec3f {
  return pow(color, vec3(1.0 / 2.2));
}

@compute
@workgroup_size(10, 10, 1)
fn render(@builtin(global_invocation_id) gid: vec3u) {
  rng_state = (gid.x * 1973 + gid.y * 9277 + frame_id * 26699) | 1;

  let render_texture_size = vec2f(textureDimensions(render_texture).xy);
  let pixel = vec2f(gid.xy) + vec2f(rand_float(), rand_float()) - 0.5;

  let ndc = vec2f(
    pixel.x / render_texture_size.x * 2.0 - 1.0,
    1.0 - pixel.y / render_texture_size.y * 2.0
  );

  let origin_world_space = camera.inverse_view * vec4(0, 0, 0, 1);
  let direction_view_space = normalize(camera.inverse_proj * vec4(ndc, 0.0, 1.0));
  let direction_world_space = normalize(camera.inverse_view * vec4(direction_view_space.xyz, 0));

  let ray_color = min(trace_ray(RayDesc(
    0,
    0xff,
    0.1,
    100.0,
    origin_world_space.xyz,
    direction_world_space.xyz
  ), gid), vec3f(1, 1, 1));

  textureStore(render_texture, gid.xy, vec4(ray_color, 1.0));
}
