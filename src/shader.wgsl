@group(0) @binding(0)
var render_texture: texture_storage_2d<rgba32float, write>;

@compute
@workgroup_size(16, 16, 1)
fn render(@builtin(global_invocation_id) gid: vec3<u32>) {
  textureStore(render_texture, gid.xy, vec4<f32>(1.0, 0.0, 0.0, 1.0));
}
