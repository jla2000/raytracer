@group(0) @binding(0)
var render_texture: texture_storage_2d<rgba32float, write>;

@compute
@workgroup_size(10, 10, 1)
fn render(@builtin(global_invocation_id) gid: vec3<u32>) {
  var color_rg = vec2<f32>(gid.xy) / vec2<f32>(textureDimensions(render_texture).xy);
  textureStore(render_texture, gid.xy, vec4<f32>(color_rg, 1.0, 1.0));
}
