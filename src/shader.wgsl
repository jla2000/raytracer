@group(0) @binding(0)
var render_texture: texture_storage_2d<bgra8unorm, read_write>;

@group(0) @binding(1)
var<uniform> camera: CameraMatrices;

@group(0) @binding(2)
var acc_struct: acceleration_structure;

@group(0) @binding(3)
var<storage, read> vertices: array<Vertex>;

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

struct Vertex {
  position: vec3f,
  _pad0: f32,
  normal: vec3f,
  material: u32,
}

fn sky_color(ray_desc: RayDesc) -> vec3f {
  let a = 0.5 * (ray_desc.dir.y + 1.0);
  return (1.0 - a) * vec3(1.0, 1.0, 1.0) + a * vec3(0.5, 0.7, 1.0);
}

fn trace_ray(ray_desc: RayDesc) -> vec3f {
  var ray = ray_desc;
  var color = sky_color(ray);

  var ray_query: ray_query;

  rayQueryInitialize(&ray_query, acc_struct, ray);
  rayQueryProceed(&ray_query);

  var intersection = rayQueryGetCommittedIntersection(&ray_query);

  for (var i = 0; i < 10; i++) {
    if (intersection.kind != RAY_QUERY_INTERSECTION_NONE) {
      let n0 = vertices[intersection.primitive_index * 3 + 0].normal;
      let n1 = vertices[intersection.primitive_index * 3 + 1].normal;
      let n2 = vertices[intersection.primitive_index * 3 + 2].normal;
      let material = vertices[intersection.primitive_index * 3].material;

      let u = intersection.barycentrics.x;
      let v = intersection.barycentrics.y;
      let w = 1.0 - u - v;

      let normal = normalize(w * n0 + u * n1 + v * n2);

      ray.origin = ray.origin + ray.dir * intersection.t;
      if (material != 0) {
        ray.dir = reflect(ray.dir, normal);
      } else {
        ray.dir = normalize(normal + random_on_hemisphere(normal));
      }
      color *= 0.5;

      rayQueryInitialize(&ray_query, acc_struct, ray);
      rayQueryProceed(&ray_query);
      intersection = rayQueryGetCommittedIntersection(&ray_query);
    } else {
      if ray.dir.y < 0.0 {
        let t = -ray.origin.y / ray.dir.y;
        let normal = vec3(0.0, 1.0, 0.0);

        ray.origin = ray.origin + ray.dir * t;
        ray.dir = normalize(normal + random_on_hemisphere(normal));
        color *= 0.4;

        rayQueryInitialize(&ray_query, acc_struct, ray);
        rayQueryProceed(&ray_query);
        intersection = rayQueryGetCommittedIntersection(&ray_query);
      } else {
        break;
      }
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
  return normalize((vec3f(rand_float(), rand_float(), rand_float()) - 0.5) * 2);
}

fn random_on_hemisphere(normal: vec3f) -> vec3f {
  let rand = random_unit_vec();
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
  rng_state = (gid.x * 1973 + gid.y * 9277 + push_constants.num_samples * 26699) | 1;

  let render_texture_size = vec2f(textureDimensions(render_texture).xy);
  let pixel = vec2f(gid.xy) + vec2f(rand_float(), rand_float()) - 0.5;

  let ndc = vec2f(
    pixel.x / render_texture_size.x * 2.0 - 1.0,
    1.0 - pixel.y / render_texture_size.y * 2.0
  );

  let origin_world_space = camera.inverse_view * vec4(0, 0, 0, 1);
  let direction_view_space = normalize(camera.inverse_proj * vec4(ndc, 0.0, 1.0));
  let direction_world_space = normalize(camera.inverse_view * vec4(direction_view_space.xyz, 0));

  let ray_color = trace_ray(RayDesc(
    0,
    0xff,
    0.1,
    100.0,
    origin_world_space.xyz,
    direction_world_space.xyz
  ));

  let accumulated_color = textureLoad(render_texture, gid.xy).xyz;
  let mixed_color = mix(accumulated_color, ray_color, 1 / (f32(push_constants.num_samples) + 1));
  textureStore(render_texture, gid.xy, vec4(mixed_color, 1.0));
}
