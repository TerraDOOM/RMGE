#version 450

layout (location = 0) in vec3 vert_pos;
layout (location = 0) out gl_PerVertex {
  vec4 gl_Position;
};
layout (location = 1) in uint tex_index;
layout (location = 1) flat out uint tex_index_out;

void main() {
  gl_Position = vec4(vert_pos, 0);
  tex_index_out = tex_index;
}
