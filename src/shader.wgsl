@group(0) @binding(0)
var render_texture: texture_storage_2d<bgra8unorm, read_write>;

@compute
@workgroup_size(10, 10, 1)
fn render(@builtin(global_invocation_id) gid: vec3<u32>) {
  let coords = gid.xy;

  // Read the current color from the texture
  var old_color = textureLoad(render_texture, coords).rgb;

  // Simple color transformation logic
  let new_r = fract(old_color.r + 0.01);
  let new_g = fract(old_color.g + 0.03);
  let new_b = fract(old_color.b + 0.02);

  // Store the updated color back to the texture
  textureStore(render_texture, coords, vec4<f32>(new_r, new_g, new_b, 1.0));
}
