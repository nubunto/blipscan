#version 330

in vec2 fragTexCoord;
uniform sampler2D texture0;
out vec4 finalColor;

void main() {
    vec4 color = texture(texture0, fragTexCoord);

    // distance from center (0 at center, ~0.7 at corners)
    vec2 uv = fragTexCoord - 0.5;
    float vignette = 1.0 - dot(uv, uv) * 2.;

    finalColor = vec4(color.rgb * vignette, color.a);
}
