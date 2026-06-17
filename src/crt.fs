#version 330

in vec2 fragTexCoord;
uniform sampler2D texture0;
out vec4 finalColor;

void main() {
    vec2 uv = fragTexCoord;
    // example: CRT scanlines
    vec4 color = texture(texture0, uv);
    color.rgb *= 0.8 + 0.2 * sin(uv.y * 600 * 3.14159);
    finalColor = color;
}
