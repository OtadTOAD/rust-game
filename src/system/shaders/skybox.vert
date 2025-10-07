#version 450

layout(location = 0) in vec2 position;

layout(location = 0) out vec2 frag_coord;

void main() {
    gl_Position = vec4(position, 1.0, 1.0);
    frag_coord  = position * 0.5 + 0.5;
}